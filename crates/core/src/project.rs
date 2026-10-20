//! A project on disk: its root, its project file, and its dialogue files, read in place.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use dm_syntax::Parse;
use walkdir::WalkDir;

use crate::config::{self, Compiled, Config, ConfigError};

/// A project, loaded.
#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub config: Config,
    pub compiled: Compiled,
    /// The project file, when there is one.
    pub config_path: Option<PathBuf>,
    pub files: Vec<SourceFile>,
    /// Dialogue files that could not be read; they are never written.
    pub unreadable: Vec<(String, String)>,
}

/// A dialogue file as it was read.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// Relative to the project root, with `/`.
    pub path: String,
    pub text: String,
    pub parse: Parse,
    line_starts: Vec<usize>,
}

#[derive(Debug)]
pub enum LoadError {
    NoRoot(PathBuf),
    Io(PathBuf, std::io::Error),
    Config(PathBuf, ConfigError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRoot(path) => write!(f, "{}: not a folder", path.display()),
            Self::Io(path, e) => write!(f, "{}: {e}", path.display()),
            Self::Config(path, e) => write!(f, "{}: {e}", path.display()),
        }
    }
}

impl std::error::Error for LoadError {}

impl Project {
    /// Load the project at `root`, with the project file at `config_path`, or the one at the
    /// root, or none.
    pub fn load(root: &Path, config_path: Option<&Path>) -> Result<Self, LoadError> {
        if !root.is_dir() {
            return Err(LoadError::NoRoot(root.to_owned()));
        }
        let config_path = match config_path {
            Some(path) => Some(path.to_owned()),
            None => Some(root.join(config::FILE_NAME)).filter(|p| p.is_file()),
        };
        let config = match &config_path {
            Some(path) => {
                let text = fs::read_to_string(path).map_err(|e| LoadError::Io(path.clone(), e))?;
                Config::parse(&text).map_err(|e| LoadError::Config(path.clone(), e))?
            }
            None => Config::default(),
        };
        Self::with_config(root, config, config_path)
    }

    /// Load the project at `root` with a project file already read (`config_path` is
    /// where it lives, for paths relative to it).
    pub fn with_config(
        root: &Path,
        config: Config,
        config_path: Option<PathBuf>,
    ) -> Result<Self, LoadError> {
        if !root.is_dir() {
            return Err(LoadError::NoRoot(root.to_owned()));
        }
        let compiled = config
            .compile()
            .map_err(|e| LoadError::Config(config_path.clone().unwrap_or_default(), e))?;

        let mut files = Vec::new();
        let mut unreadable = Vec::new();
        for path in find(root, |rel| {
            compiled.dialogue.is_match(rel) && !compiled.exclude.is_match(rel)
        }) {
            let full = root.join(&path);
            let bytes = fs::read(&full).map_err(|e| LoadError::Io(full.clone(), e))?;
            match String::from_utf8(bytes) {
                Ok(text) => files.push(SourceFile::new(path, text)),
                Err(_) => unreadable.push((path, "not UTF-8".to_owned())),
            }
        }
        Ok(Self {
            root: root.to_owned(),
            config,
            compiled,
            config_path,
            files,
            unreadable,
        })
    }

    /// The text of the code files the project file points at (`sources.code`), by path.
    pub fn code_files(&self) -> Vec<(String, String)> {
        find(&self.root, |rel| {
            self.compiled.code.is_match(rel) && !self.compiled.exclude.is_match(rel)
        })
        .into_iter()
        .filter_map(|path| {
            let text = fs::read(self.root.join(&path)).ok()?;
            Some((path, String::from_utf8_lossy(&text).into_owned()))
        })
        .collect()
    }

    /// A path from the project file, relative to the root.
    pub fn resolve(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    /// A document path, relative to the project file's folder.
    pub fn resolve_document(&self, path: &str) -> PathBuf {
        match self.config_path.as_deref().and_then(Path::parent) {
            Some(dir) => dir.join(path),
            None => self.root.join(path),
        }
    }

    pub fn file_index(&self, path: &str) -> Option<usize> {
        self.files.iter().position(|f| f.path == path)
    }
}

impl SourceFile {
    pub fn new(path: String, text: String) -> Self {
        let parse = dm_syntax::parse(&text);
        let line_starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self {
            path,
            text,
            parse,
            line_starts,
        }
    }

    /// 1-based line and column (in characters) of a byte offset.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let line = self.line_starts.partition_point(|&start| start <= offset) - 1;
        let column = self.text[self.line_starts[line]..offset].chars().count() + 1;
        (line + 1, column)
    }
}

/// Files under `root` (hidden folders skipped) whose relative path passes `keep`, sorted.
fn find(root: &Path, keep: impl Fn(&str) -> bool) -> Vec<String> {
    let mut out: Vec<String> = WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let rel = e.path().strip_prefix(root).ok()?;
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            keep(&rel).then_some(rel)
        })
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_and_column() {
        let file = SourceFile::new("a".into(), "ab\nção\n\nx".into());
        assert_eq!(file.line_col(0), (1, 1));
        assert_eq!(file.line_col(2), (1, 3));
        assert_eq!(file.line_col(3), (2, 1));
        assert_eq!(file.line_col(5), (2, 2));
        assert_eq!(file.line_col(8), (2, 4));
        assert_eq!(file.line_col(9), (3, 1));
        assert_eq!(file.line_col(10), (4, 1));
    }
}
