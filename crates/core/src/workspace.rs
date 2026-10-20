//! A project open for editing: edits go to working copies of the files, which are written
//! only when saved. A file is written only when its working copy differs from what was
//! read, and never over a file that changed on disk since it was read: the save is refused
//! and the reader decides.

use std::collections::BTreeMap;
use std::fmt;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::Serialize;
use similar::{ChangeTag, TextDiff};
use ts_rs::TS;

use crate::edit::{self, Edit, EditError};
use crate::project::{LoadError, Project, SourceFile};
use crate::view::{ReadView, editing_view};

pub struct Workspace {
    project: Project,
    config: Option<PathBuf>,
    /// Each file's text as it was on disk when read, by path.
    disk: BTreeMap<String, String>,
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
        let project = Project::load(root, config)?;
        let disk = project
            .files
            .iter()
            .map(|f| (f.path.clone(), f.text.clone()))
            .collect();
        Ok(Self {
            project,
            config: config.map(Path::to_owned),
            disk,
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
            changed: self.changed().map(|f| f.path.clone()).collect(),
            conflicts: self.conflicts.clone(),
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
        }
    }

    fn changed(&self) -> impl Iterator<Item = &SourceFile> {
        self.project
            .files
            .iter()
            .filter(|f| self.disk.get(&f.path) != Some(&f.text))
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
        let before = self.project.files[index].text.clone();
        let mut text = before.clone();
        let mut at = 0;
        for edit in edits {
            let applied = edit::apply(&text, edit)?;
            text = applied.text;
            at = applied.at;
        }
        if text != before {
            self.set(index, text.clone());
            self.undo.push(Step {
                path: path.to_owned(),
                before,
                after: text,
            });
            self.redo.clear();
        }
        Ok(at)
    }

    pub fn undo(&mut self) -> Option<String> {
        let step = self.undo.pop()?;
        let index = self.project.file_index(&step.path)?;
        self.set(index, step.before.clone());
        let path = step.path.clone();
        self.redo.push(step);
        Some(path)
    }

    pub fn redo(&mut self) -> Option<String> {
        let step = self.redo.pop()?;
        let index = self.project.file_index(&step.path)?;
        self.set(index, step.after.clone());
        let path = step.path.clone();
        self.undo.push(step);
        Some(path)
    }

    fn set(&mut self, index: usize, text: String) {
        let path = self.project.files[index].path.clone();
        self.project.files[index] = SourceFile::new(path, text);
    }

    /// The unsaved edits of each changed file.
    pub fn diffs(&self) -> Vec<FileDiff> {
        self.changed()
            .map(|file| {
                let disk = self.disk.get(&file.path).map_or("", String::as_str);
                let diff = TextDiff::from_lines(disk, &file.text);
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
                    path: file.path.clone(),
                    hunks,
                }
            })
            .collect()
    }

    /// Write the files with unsaved edits, and only them. Refused, with nothing written,
    /// when one of them changed on disk since it was read.
    pub fn save(&mut self) -> Result<Vec<String>, SaveError> {
        let changed: Vec<(String, String)> = self
            .changed()
            .map(|f| (f.path.clone(), f.text.clone()))
            .collect();
        let moved: Vec<String> = changed
            .iter()
            .filter(|(path, _)| {
                std::fs::read(self.project.resolve(path)).ok().as_deref()
                    != self.disk.get(path).map(String::as_bytes)
            })
            .map(|(path, _)| path.clone())
            .collect();
        if !moved.is_empty() {
            return Err(SaveError::ChangedOnDisk(moved));
        }
        for (path, text) in &changed {
            write_atomically(&self.project.resolve(path), text.as_bytes())
                .map_err(|e| SaveError::Io(path.clone(), e))?;
            self.disk.insert(path.clone(), text.clone());
        }
        self.conflicts
            .retain(|c| !changed.iter().any(|(p, _)| p == c));
        Ok(changed.into_iter().map(|(path, _)| path).collect())
    }

    /// Drop every unsaved edit and read the files again.
    pub fn discard(&mut self) -> Result<(), LoadError> {
        *self = Self::open(&self.project.root.clone(), self.config.clone().as_deref())?;
        Ok(())
    }

    /// Read the project again after files changed on disk. Files without unsaved edits take
    /// what is on disk; files with them keep the edits, and if their file changed on disk
    /// they are marked as conflicts (saving them is refused until they are discarded).
    pub fn refresh(&mut self) -> Result<(), LoadError> {
        let fresh = Project::load(&self.project.root, self.config.as_deref())?;
        let edited: BTreeMap<String, String> = self
            .changed()
            .map(|f| (f.path.clone(), f.text.clone()))
            .collect();
        let mut disk = BTreeMap::new();
        let mut project = fresh;
        for file in &mut project.files {
            disk.insert(file.path.clone(), file.text.clone());
            if let Some(working) = edited.get(&file.path) {
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
        }
        // Steps on files that are gone or no longer edited cannot be undone safely.
        self.undo.retain(|s| edited.contains_key(&s.path));
        self.redo.retain(|s| edited.contains_key(&s.path));
        self.project = project;
        self.disk = disk;
        Ok(())
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
