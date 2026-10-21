//! A project open for editing: edits go to working copies of the files, which are written
//! only when saved. A file is written only when its working copy differs from what was
//! read, and never over a file that changed on disk since it was read: the save is refused
//! and the reader decides.
//!
//! The project file is edited the same way: a working copy, undone and saved with the
//! dialogue files. Changing it rebuilds the project around the dialogue edits.

use std::collections::BTreeMap;
use std::fmt;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::Serialize;
use similar::{ChangeTag, TextDiff};
use ts_rs::TS;

use crate::config::{self, Config};
use crate::edit::{self, Edit, EditError};
use crate::project::{LoadError, Project, SourceFile};
use crate::project_file::{self, ConfigEdit};
use crate::view::{ReadView, editing_view};

pub struct Workspace {
    root: PathBuf,
    /// The project built from the working copies.
    project: Project,
    /// Dialogue files as they were on disk when read, by path.
    disk: BTreeMap<String, String>,
    /// Working copies of the dialogue files with unsaved edits, by path.
    edits: BTreeMap<String, String>,
    /// Where the project file is (or would be), and its name in edit lists.
    config_path: PathBuf,
    config_key: String,
    /// The project file on disk when read (none: there was none), and its working copy.
    config_disk: Option<String>,
    config_text: String,
    /// Files that changed on disk while they had unsaved edits.
    conflicts: Vec<String>,
    undo: Vec<Step>,
    redo: Vec<Step>,
}

/// One edit, as the text of the file it changed before and after.
struct Step {
    path: String,
    before: String,
    after: String,
}

/// The view of the working copies, and the state of the edits.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Session {
    pub view: ReadView,
    pub state: EditState,
}

/// What a reader needs to know about unsaved work.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct EditState {
    /// Files with unsaved edits.
    pub changed: Vec<String>,
    /// Files that changed on disk while they had unsaved edits: saving them is refused.
    pub conflicts: Vec<String>,
    pub can_undo: bool,
    pub can_redo: bool,
    /// Where the project file is, or would be (relative to the project when inside it).
    pub project_file: String,
    /// Whether there is one, on disk or as an unsaved draft.
    pub has_project_file: bool,
}

/// A file's unsaved edits, as lines around each change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct FileDiff {
    pub path: String,
    pub hunks: Vec<Vec<DiffLine>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct DiffLine {
    pub kind: DiffKind,
    /// The line, without its line ending.
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DiffKind {
    Same,
    Removed,
    Added,
}

#[derive(Debug)]
pub enum SaveError {
    /// These files changed on disk since they were read; nothing was written.
    ChangedOnDisk(Vec<String>),
    Io(String, std::io::Error),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChangedOnDisk(paths) => write!(
                f,
                "changed on disk since it was read, so nothing was saved: {} (discard your edits to read the new version)",
                paths.join(", ")
            ),
            Self::Io(path, e) => write!(f, "{path}: {e}"),
        }
    }
}

impl std::error::Error for SaveError {}

impl Workspace {
    pub fn open(root: &Path, config: Option<&Path>) -> Result<Self, LoadError> {
        let config_path = config.map_or_else(|| root.join(config::FILE_NAME), Path::to_owned);
        let config_disk = match std::fs::read_to_string(&config_path) {
            Ok(text) => Some(text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && config.is_none() => None,
            Err(e) => return Err(LoadError::Io(config_path, e)),
        };
        let config_key = shown(root, &config_path);
        let config_text = config_disk.clone().unwrap_or_default();
        let project = build(root, &config_path, &config_text)?;
        let disk = project
            .files
            .iter()
            .map(|f| (f.path.clone(), f.text.clone()))
            .collect();
        Ok(Self {
            root: root.to_owned(),
            project,
            disk,
            edits: BTreeMap::new(),
            config_path,
            config_key,
            config_disk,
            config_text,
            conflicts: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
        })
    }

    /// The project with the working copies of its files.
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The editing view of the working copies, with the state of the edits.
    pub fn session(&self) -> Session {
        Session {
            view: editing_view(&self.project),
            state: self.state(),
        }
    }

    pub fn state(&self) -> EditState {
        EditState {
            changed: self.changed(),
            conflicts: self.conflicts.clone(),
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
            project_file: self.config_key.clone(),
            has_project_file: self.config_disk.is_some() || !self.config_text.is_empty(),
        }
    }

    fn config_changed(&self) -> bool {
        self.config_text != self.config_disk.as_deref().unwrap_or("")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The files dialogue-maker works on, relative to the project root: the dialogue files,
    /// the project file and its documents (those inside the project).
    pub fn owned_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self.project.files.iter().map(|f| f.path.clone()).collect();
        let inside = |path: &Path| {
            path.strip_prefix(&self.root).ok().map(|p| {
                p.components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/")
            })
        };
        if let Some(config) = inside(&self.config_path) {
            paths.push(config);
        }
        for document in &self.project.config.documents {
            if let Some(path) = inside(&self.project.resolve_document(&document.path)) {
                paths.push(path);
            }
        }
        paths
    }

    /// Paths with unsaved edits: the project file first, then dialogue files.
    fn changed(&self) -> Vec<String> {
        let config = self.config_changed().then(|| self.config_key.clone());
        config
            .into_iter()
            .chain(self.edits.keys().cloned())
            .collect()
    }

    /// Make an edit to a file's working copy; returns where the line it was about starts now.
    pub fn edit(&mut self, path: &str, edit: &Edit) -> Result<usize, EditError> {
        self.edit_batch(path, std::slice::from_ref(edit))
    }

    /// Make several edits to one file as one step (undone together); all or none. Returns
    /// where the line the last edit was about starts.
    pub fn edit_batch(&mut self, path: &str, edits: &[Edit]) -> Result<usize, EditError> {
        let index = self
            .project
            .file_index(path)
            .ok_or_else(|| EditError(format!("{path} is not one of the dialogue files")))?;
        // Titles are unique in the whole project: a jump names one without its file.
        for edit in edits {
            if let Edit::AddLabel { id, .. } = edit
                && let Some(other) = self.project.files.iter().find(|f| {
                    f.path != path && edit::titles(&f.text).contains(&id.trim().to_owned())
                })
            {
                return Err(EditError(format!(
                    "there is a label `{}` in {} already",
                    id.trim(),
                    other.path
                )));
            }
        }
        let before = self.project.files[index].text.clone();
        let mut text = before.clone();
        let mut at = 0;
        for edit in edits {
            let applied = edit::apply(&text, edit)?;
            text = applied.text;
            at = applied.at;
        }
        if text != before {
            self.set_dialogue(path, text.clone());
            self.undo.push(Step {
                path: path.to_owned(),
                before,
                after: text,
            });
            self.redo.clear();
        }
        Ok(at)
    }

    /// Make edits to the project file as one step; all or none.
    pub fn edit_project_file(&mut self, edits: &[ConfigEdit]) -> Result<(), EditError> {
        let before = self.config_text.clone();
        let mut text = before.clone();
        for edit in edits {
            text = project_file::apply(&text, edit)?;
        }
        if text != before {
            self.set_config(text.clone())?;
            self.undo.push(Step {
                path: self.config_key.clone(),
                before,
                after: text,
            });
            self.redo.clear();
        }
        Ok(())
    }

    /// For a project with no project file: a draft from its dialogue, as an unsaved edit.
    pub fn draft_project_file(&mut self) -> Result<(), EditError> {
        if self.config_disk.is_some() || !self.config_text.is_empty() {
            return Err(EditError("this project has a project file already".into()));
        }
        let draft = crate::init::draft(&self.root).map_err(|e| EditError(e.to_string()))?;
        self.set_config(draft.clone())?;
        self.undo.push(Step {
            path: self.config_key.clone(),
            before: String::new(),
            after: draft,
        });
        self.redo.clear();
        Ok(())
    }

    pub fn undo(&mut self) -> Option<String> {
        let step = self.undo.pop()?;
        self.restore(&step.path, step.before.clone());
        let path = step.path.clone();
        self.redo.push(step);
        Some(path)
    }

    pub fn redo(&mut self) -> Option<String> {
        let step = self.redo.pop()?;
        self.restore(&step.path, step.after.clone());
        let path = step.path.clone();
        self.undo.push(step);
        Some(path)
    }

    fn restore(&mut self, path: &str, text: String) {
        if path == self.config_key {
            // It read before, so it reads again.
            let _ = self.set_config(text);
        } else {
            self.set_dialogue(path, text);
        }
    }

    fn set_dialogue(&mut self, path: &str, text: String) {
        if self.disk.get(path) == Some(&text) {
            self.edits.remove(path);
        } else {
            self.edits.insert(path.to_owned(), text.clone());
        }
        if let Some(index) = self.project.file_index(path) {
            self.project.files[index] = SourceFile::new(path.to_owned(), text);
        }
    }

    /// Take a new working copy of the project file and rebuild the project around the
    /// dialogue edits.
    fn set_config(&mut self, text: String) -> Result<(), EditError> {
        let mut project =
            build(&self.root, &self.config_path, &text).map_err(|e| EditError(e.to_string()))?;
        for file in &mut project.files {
            self.disk
                .entry(file.path.clone())
                .or_insert_with(|| file.text.clone());
            if let Some(working) = self.edits.get(&file.path) {
                *file = SourceFile::new(file.path.clone(), working.clone());
            }
        }
        self.project = project;
        self.config_text = text;
        Ok(())
    }

    /// The unsaved edits of each changed file, the project file first.
    pub fn diffs(&self) -> Vec<FileDiff> {
        let config = self.config_changed().then(|| {
            diff(
                &self.config_key,
                self.config_disk.as_deref().unwrap_or(""),
                &self.config_text,
            )
        });
        let dialogue = self
            .edits
            .iter()
            .map(|(path, text)| diff(path, self.disk.get(path).map_or("", String::as_str), text));
        config.into_iter().chain(dialogue).collect()
    }

    /// Write the files with unsaved edits, and only them. Refused, with nothing written,
    /// when one of them changed on disk since it was read (or, for a new project file,
    /// appeared).
    pub fn save(&mut self) -> Result<Vec<String>, SaveError> {
        let mut moved = Vec::new();
        if self.config_changed() {
            let now = std::fs::read_to_string(&self.config_path).ok();
            if now != self.config_disk {
                moved.push(self.config_key.clone());
            }
        }
        for path in self.edits.keys() {
            let now = std::fs::read(self.project.resolve(path)).ok();
            if now.as_deref() != self.disk.get(path).map(String::as_bytes) {
                moved.push(path.clone());
            }
        }
        if !moved.is_empty() {
            return Err(SaveError::ChangedOnDisk(moved));
        }
        let saved = self.changed();
        if self.config_changed() {
            write_atomically(&self.config_path, self.config_text.as_bytes())
                .map_err(|e| SaveError::Io(self.config_key.clone(), e))?;
            self.config_disk = Some(self.config_text.clone());
        }
        for (path, text) in std::mem::take(&mut self.edits) {
            write_atomically(&self.project.resolve(&path), text.as_bytes())
                .map_err(|e| SaveError::Io(path.clone(), e))?;
            self.disk.insert(path, text);
        }
        self.conflicts.retain(|c| !saved.contains(c));
        Ok(saved)
    }

    /// Drop every unsaved edit and read the files again.
    pub fn discard(&mut self) -> Result<(), LoadError> {
        let config = (self.config_path != self.root.join(config::FILE_NAME))
            .then(|| self.config_path.clone());
        *self = Self::open(&self.root.clone(), config.as_deref())?;
        Ok(())
    }

    /// Read the project again after files changed on disk. Files without unsaved edits take
    /// what is on disk; files with them keep the edits, and if their file changed on disk
    /// they are marked as conflicts (saving them is refused until they are discarded).
    pub fn refresh(&mut self) -> Result<(), LoadError> {
        let config_now = std::fs::read_to_string(&self.config_path).ok();
        if !self.config_changed() {
            self.config_text = config_now.clone().unwrap_or_default();
            self.config_disk = config_now;
        } else if config_now != self.config_disk && !self.conflicts.contains(&self.config_key) {
            self.conflicts.push(self.config_key.clone());
        }
        let mut project = build(&self.root, &self.config_path, &self.config_text)?;
        let mut disk = BTreeMap::new();
        for file in &mut project.files {
            match self.edits.get(&file.path) {
                Some(working) => {
                    if self.disk.get(&file.path) != Some(&file.text)
                        && !self.conflicts.contains(&file.path)
                    {
                        self.conflicts.push(file.path.clone());
                    }
                    // The edits stay, over what was read before.
                    disk.insert(
                        file.path.clone(),
                        self.disk.get(&file.path).cloned().unwrap_or_default(),
                    );
                    *file = SourceFile::new(file.path.clone(), working.clone());
                }
                None => {
                    disk.insert(file.path.clone(), file.text.clone());
                }
            }
        }
        // Steps on files that are no longer edited cannot be undone safely.
        let edited: Vec<String> = self.changed();
        self.undo.retain(|s| edited.contains(&s.path));
        self.redo.retain(|s| edited.contains(&s.path));
        self.project = project;
        self.disk = disk;
        Ok(())
    }
}

/// The project from a project file's text (empty: the defaults).
fn build(root: &Path, config_path: &Path, text: &str) -> Result<Project, LoadError> {
    let config = Config::parse(text).map_err(|e| LoadError::Config(config_path.to_owned(), e))?;
    Project::with_config(root, config, Some(config_path.to_owned()))
}

/// A path as the reader sees it: relative to the project when inside it.
fn shown(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(relative) => relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => path.display().to_string(),
    }
}

fn diff(path: &str, before: &str, after: &str) -> FileDiff {
    let diff = TextDiff::from_lines(before, after);
    let hunks = diff
        .grouped_ops(2)
        .iter()
        .map(|group| {
            group
                .iter()
                .flat_map(|op| diff.iter_changes(op))
                .map(|change| DiffLine {
                    kind: match change.tag() {
                        ChangeTag::Equal => DiffKind::Same,
                        ChangeTag::Delete => DiffKind::Removed,
                        ChangeTag::Insert => DiffKind::Added,
                    },
                    text: change.value().trim_end_matches(['\n', '\r']).to_owned(),
                })
                .collect()
        })
        .collect();
    FileDiff {
        path: path.to_owned(),
        hunks,
    }
}

/// Write to a temporary file beside the target, then rename it over the target, so the
/// file is never left half written.
fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(dir)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    if let Ok(meta) = std::fs::metadata(path) {
        let _ = std::fs::set_permissions(file.path(), meta.permissions());
    }
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}
