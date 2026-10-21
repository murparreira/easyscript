//! The project's git repository: how its files stand against the last commit, branches,
//! and commits of the files dialogue-maker works on (dialogue files, the project file, its
//! documents). Nothing here commits a file outside those, unstages what the team staged by
//! hand, or resolves a conflict.

use std::fmt;
use std::path::{Path, PathBuf};

use git2::{BranchType, IndexEntry, IndexTime, Oid, Repository, Status, StatusOptions};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GitStatus {
    /// The branch checked out; none on a detached head.
    pub branch: Option<String>,
    /// The last commit: short id and summary.
    pub head: Option<String>,
    pub branches: Vec<String>,
    /// The project's own files that differ from the last commit.
    pub files: Vec<GitFile>,
    /// Paths in conflict (a merge or rebase stopped): shown, never resolved here.
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GitFile {
    /// Relative to the project root.
    pub path: String,
    pub change: GitChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum GitChange {
    Modified,
    Added,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitError(pub String);

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GitError {}

impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        Self(e.message().to_owned())
    }
}

/// The repository the project is in, and the project's folder inside it.
pub(crate) struct Repo {
    pub(crate) repo: Repository,
    /// The project root, relative to the repository's working folder (`""` at its top).
    prefix: PathBuf,
}

impl Repo {
    pub(crate) fn open(root: &Path) -> Result<Option<Self>, GitError> {
        let repo = match Repository::discover(root) {
            Ok(repo) => repo,
            Err(e) if e.code() == git2::ErrorCode::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let Some(workdir) = repo.workdir() else {
            return Ok(None);
        };
        let canonical = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_owned());
        let prefix = canonical(root)
            .strip_prefix(canonical(workdir))
            .map(Path::to_owned)
            .unwrap_or_default();
        Ok(Some(Self { repo, prefix }))
    }

    /// A project path as a repository path (`/` separated).
    pub(crate) fn in_repo(&self, path: &str) -> String {
        let joined = self.prefix.join(path);
        joined
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }

    /// A repository path as a project path, when it is inside the project.
    pub(crate) fn in_project(&self, path: &str) -> Option<String> {
        let prefix = self
            .prefix
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if prefix.is_empty() {
            Some(path.to_owned())
        } else {
            path.strip_prefix(&format!("{prefix}/")).map(str::to_owned)
        }
    }

    pub(crate) fn workdir(&self) -> &Path {
        self.repo.workdir().expect("checked in open")
    }
}

/// Where the project's own files (`paths`, relative to the project root) stand. None when
/// the project is not in a git repository.
pub fn status(root: &Path, paths: &[String]) -> Result<Option<GitStatus>, GitError> {
    let Some(r) = Repo::open(root)? else {
        return Ok(None);
    };
    let repo = &r.repo;
    let head = repo.head().ok();
    let branch = head
        .as_ref()
        .filter(|h| h.is_branch())
        .and_then(|h| h.shorthand().ok().map(str::to_owned));
    let head_text = head
        .as_ref()
        .and_then(|h| h.peel_to_commit().ok())
        .map(|c| {
            let id = c.id().to_string();
            format!("{} {}", &id[..7], c.summary().ok().flatten().unwrap_or(""))
        });
    let mut branches: Vec<String> = repo
        .branches(Some(BranchType::Local))?
        .filter_map(Result::ok)
        .filter_map(|(b, _)| b.name().ok().flatten().map(str::to_owned))
        .collect();
    branches.sort();

    let mut files = Vec::new();
    if !paths.is_empty() {
        let mut options = StatusOptions::new();
        options
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .disable_pathspec_match(true);
        for path in paths {
            options.pathspec(r.in_repo(path));
        }
        for entry in repo.statuses(Some(&mut options))?.iter() {
            let Some(path) = entry.path().ok().and_then(|p| r.in_project(p)) else {
                continue;
            };
            let s = entry.status();
            let change = if s.intersects(Status::WT_DELETED | Status::INDEX_DELETED) {
                GitChange::Deleted
            } else if s.intersects(Status::WT_NEW | Status::INDEX_NEW) {
                GitChange::Added
            } else if s.intersects(
                Status::WT_MODIFIED
                    | Status::INDEX_MODIFIED
                    | Status::WT_RENAMED
                    | Status::INDEX_RENAMED
                    | Status::WT_TYPECHANGE
                    | Status::INDEX_TYPECHANGE,
            ) {
                GitChange::Modified
            } else {
                continue;
            };
            files.push(GitFile { path, change });
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
    }
    let conflicts = conflicts(repo)?;
    Ok(Some(GitStatus {
        branch,
        head: head_text,
        branches,
        files,
        conflicts,
    }))
}

pub(crate) fn conflicts(repo: &Repository) -> Result<Vec<String>, GitError> {
    let index = repo.index()?;
    if !index.has_conflicts() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for conflict in index.conflicts()? {
        let conflict = conflict?;
        let entry = conflict.our.or(conflict.their).or(conflict.ancestor);
        if let Some(entry) = entry {
            out.push(String::from_utf8_lossy(&entry.path).into_owned());
        }
    }
    out.dedup();
    Ok(out)
}

/// Commit these project files as they are on disk now, and nothing else: the commit is
/// the last one with these files changed. Whatever else is staged stays staged. Returns
/// the new commit's short id.
pub fn commit(root: &Path, paths: &[String], message: &str) -> Result<String, GitError> {
    let r = Repo::open(root)?
        .ok_or_else(|| GitError("the project is not in a git repository".into()))?;
    let repo = &r.repo;
    let message = message.trim();
    if message.is_empty() {
        return Err(GitError("a commit needs a message".into()));
    }
    if paths.is_empty() {
        return Err(GitError("no files to commit".into()));
    }
    let conflicted = conflicts(repo)?;
    if !conflicted.is_empty() {
        return Err(GitError(format!(
            "the repository has conflicts to resolve first: {}",
            conflicted.join(", ")
        )));
    }
    let signature = repo.signature().map_err(|_| {
        GitError("git does not know who you are: set user.name and user.email".into())
    })?;
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());

    // The last commit's tree, with these files as they are on disk.
    let mut index = git2::Index::new()?;
    if let Some(parent) = &parent {
        index.read_tree(&parent.tree()?)?;
    }
    let mut real = repo.index()?;
    for path in paths {
        let in_repo = r.in_repo(path);
        let on_disk = r.workdir().join(&in_repo);
        match std::fs::read(&on_disk) {
            Ok(data) => {
                let mode = index
                    .get_path(Path::new(&in_repo), 0)
                    .map_or(0o100644, |e| e.mode);
                let id = repo.blob(&data)?;
                index.add(&entry(&in_repo, mode, id, data.len()))?;
                real.add_path(Path::new(&in_repo))?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let _ = index.remove_path(Path::new(&in_repo));
                let _ = real.remove_path(Path::new(&in_repo));
            }
            Err(e) => return Err(GitError(format!("{path}: {e}"))),
        }
    }
    let tree = repo.find_tree(index.write_tree_to(repo)?)?;
    if parent.as_ref().is_some_and(|p| p.tree_id() == tree.id()) {
        return Err(GitError("these files have no changes to commit".into()));
    }
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    let id = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parents,
    )?;
    real.write()?;
    Ok(id.to_string()[..7].to_owned())
}

pub(crate) fn entry(path: &str, mode: u32, id: Oid, size: usize) -> IndexEntry {
    IndexEntry {
        ctime: IndexTime::new(0, 0),
        mtime: IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode,
        uid: 0,
        gid: 0,
        file_size: u32::try_from(size).unwrap_or(u32::MAX),
        id,
        flags: 0,
        flags_extended: 0,
        path: path.as_bytes().to_vec(),
    }
}

/// Make a branch at the last commit, and switch to it.
pub fn create_branch(root: &Path, name: &str) -> Result<(), GitError> {
    let r = Repo::open(root)?
        .ok_or_else(|| GitError("the project is not in a git repository".into()))?;
    let name = name.trim();
    if !git2::Branch::name_is_valid(name)? {
        return Err(GitError(format!("`{name}` cannot be a branch name")));
    }
    let head = r.repo.head()?.peel_to_commit()?;
    r.repo.branch(name, &head, false)?;
    r.repo.set_head(&format!("refs/heads/{name}"))?;
    Ok(())
}

/// Switch to another branch. Refused when the switch would change a file with changes not
/// committed (git's safe checkout).
pub fn switch_branch(root: &Path, name: &str) -> Result<(), GitError> {
    let r = Repo::open(root)?
        .ok_or_else(|| GitError("the project is not in a git repository".into()))?;
    let reference = format!("refs/heads/{name}");
    let target = r.repo.revparse_single(&reference)?;
    r.repo
        .checkout_tree(&target, Some(git2::build::CheckoutBuilder::new().safe()))
        .map_err(|e| {
            GitError(format!(
                "cannot switch to {name}: {} (commit or put away the changes first)",
                e.message()
            ))
        })?;
    r.repo.set_head(&reference)?;
    Ok(())
}
