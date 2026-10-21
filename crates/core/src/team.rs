//! Working with the team through the repository's `origin`: what came in from it that is
//! not here yet, what is here that it does not have, what teammates are changing on their
//! own branches, and bringing the team's changes in. Changes are told by scene (the labels
//! whose lines they touch), so a writer reads "Beto changed taxista_confissao", not a diff.
//!
//! Nothing here talks to the network: fetching and pushing are the GitHub crate's. Bringing
//! changes in never leaves the repository half way: when the team's changes and the
//! writer's touch the same lines, nothing is changed and the files are named.

use std::collections::BTreeSet;
use std::path::Path;

use es_syntax::AstNode;
use es_syntax::SyntaxKind::TITLE_LINE;
use es_syntax::ast;
use git2::{Commit, Oid, Repository, RepositoryState, Sort, Tree};
use serde::Serialize;
use similar::{DiffOp, TextDiff};
use ts_rs::TS;

use crate::git::{GitError, Repo, entry};

/// A scene a change touched: its file and its label; no label for the lines before a
/// file's first label, or a file that is not dialogue (the project file, a document).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[ts(export)]
pub struct SceneRef {
    /// Relative to the project root.
    pub path: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CommitInfo {
    /// The full id.
    pub id: String,
    pub author: String,
    /// When it was made, in seconds since 1970.
    #[ts(type = "number")]
    pub time: i64,
    pub summary: String,
    /// Made by whoever this repository commits as.
    pub mine: bool,
    /// The project's scenes it changed.
    pub scenes: Vec<SceneRef>,
}

/// A teammate's branch on `origin` with work the shared branch does not have yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct BranchWork {
    /// Its name on origin, without `origin/`.
    pub branch: String,
    /// Its last commit, with the scenes the whole branch changes.
    pub last: CommitInfo,
    /// Commits on it the shared branch does not have.
    pub ahead: usize,
}

/// Where this copy of the project stands against the team.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct TeamState {
    /// The branch on `origin` the team shares (its default), when known.
    pub shared: Option<String>,
    /// The branch checked out here.
    pub branch: Option<String>,
    /// Whether `origin` has this branch.
    pub published: bool,
    /// Commits on `origin` (the shared branch, and this branch there) not here, newest first.
    pub incoming: Vec<CommitInfo>,
    /// Commits here that `origin` does not have, newest first.
    pub outgoing: Vec<CommitInfo>,
    /// Commits here since the one last seen, newest first.
    pub since: Vec<CommitInfo>,
    /// Teammates' branches with work not on the shared branch, latest first.
    pub others: Vec<BranchWork>,
    /// Scenes changed on disk and not committed.
    pub uncommitted: Vec<SceneRef>,
    /// The last commit here, to remember as seen.
    pub head: Option<String>,
}

/// What bringing the team's changes in did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Updated {
    /// Commits brought in.
    pub commits: usize,
    /// Scenes they changed.
    pub scenes: Vec<SceneRef>,
    /// Files with the writer's own changes not committed, merged with the team's.
    pub merged: Vec<String>,
    /// Files the team and the writer changed in the same lines: when there are any,
    /// nothing was changed.
    pub conflicts: Vec<String>,
}

/// At most this many commits are listed in each list.
const LISTED: usize = 100;

/// How the project stands against `origin`, as of the last fetch. `paths` are the project's
/// own files (dialogue, project file, documents), relative to its root; `seen` is a commit
/// the writer saw last, for what came in since. None outside a git repository.
pub fn team(
    root: &Path,
    paths: &[String],
    seen: Option<&str>,
) -> Result<Option<TeamState>, GitError> {
    let Some(r) = Repo::open(root)? else {
        return Ok(None);
    };
    let repo = &r.repo;
    let head = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let branch = repo
        .head()
        .ok()
        .filter(|h| h.is_branch())
        .and_then(|h| h.shorthand().ok().map(str::to_owned));
    let shared = shared_branch(repo);
    let shared_tip = shared.as_deref().and_then(|b| remote_tip(repo, b));
    let own_tip = branch.as_deref().and_then(|b| remote_tip(repo, b));
    let me = repo
        .config()
        .ok()
        .and_then(|c| c.get_string("user.email").ok());
    let info = |c: &Commit, scenes: Vec<SceneRef>| CommitInfo {
        id: c.id().to_string(),
        author: c.author().name().ok().unwrap_or("?").to_owned(),
        time: c.author().when().seconds(),
        summary: c.summary().ok().flatten().unwrap_or("").to_owned(),
        mine: me.is_some() && c.author().email().ok() == me.as_deref(),
        scenes,
    };
    let listed = |from: &[Oid], hide: &[Oid]| -> Result<Vec<CommitInfo>, GitError> {
        let mut out = Vec::new();
        for id in walk(repo, from, hide)?.into_iter().take(LISTED) {
            let commit = repo.find_commit(id)?;
            let parent = commit.parents().next();
            let scenes = scenes_between(
                &r,
                parent.map(|p| p.tree()).transpose()?.as_ref(),
                Some(&commit.tree()?),
                paths,
            )?;
            out.push(info(&commit, scenes));
        }
        Ok(out)
    };

    let mut incoming = Vec::new();
    let mut outgoing = Vec::new();
    let mut since = Vec::new();
    if let Some(head) = &head {
        let from: Vec<Oid> = [shared_tip, own_tip].into_iter().flatten().collect();
        incoming = listed(&from, &[head.id()])?;
        if let Some(base) = own_tip.or(shared_tip) {
            outgoing = listed(&[head.id()], &[base])?;
        }
        if let Some(seen) = seen
            .and_then(|s| Oid::from_str(s).ok())
            .filter(|s| repo.find_commit(*s).is_ok())
        {
            since = listed(&[head.id()], &[seen])?;
        }
    }

    let mut others = Vec::new();
    if let Some(shared_tip) = shared_tip {
        for reference in repo.references_glob("refs/remotes/origin/*")? {
            let reference = reference?;
            let Some(name) = reference
                .shorthand()
                .ok()
                .and_then(|n| n.strip_prefix("origin/"))
            else {
                continue;
            };
            if name == "HEAD" || Some(name) == shared.as_deref() || Some(name) == branch.as_deref()
            {
                continue;
            }
            let Ok(tip) = reference.peel_to_commit() else {
                continue;
            };
            if tip.id() == shared_tip || repo.graph_descendant_of(shared_tip, tip.id())? {
                continue;
            }
            let Ok(base) = repo.merge_base(tip.id(), shared_tip) else {
                continue;
            };
            let scenes = scenes_between(
                &r,
                Some(&repo.find_commit(base)?.tree()?),
                Some(&tip.tree()?),
                paths,
            )?;
            if scenes.is_empty() {
                continue;
            }
            let ahead = walk(repo, &[tip.id()], &[base])?.len();
            others.push(BranchWork {
                branch: name.to_owned(),
                last: info(&tip, scenes),
                ahead,
            });
        }
        others.sort_by_key(|o| std::cmp::Reverse(o.last.time));
    }

    let mut uncommitted = BTreeSet::new();
    let head_tree = head.as_ref().map(Commit::tree).transpose()?;
    for path in paths {
        let in_repo = r.in_repo(path);
        let before = head_tree
            .as_ref()
            .and_then(|t| blob_text(repo, t, &in_repo));
        let now = std::fs::read(r.workdir().join(&in_repo))
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned());
        if before != now {
            scenes_of(path, before.as_deref(), now.as_deref(), &mut uncommitted);
        }
    }

    Ok(Some(TeamState {
        shared,
        published: own_tip.is_some(),
        branch,
        incoming,
        outgoing,
        since,
        others,
        uncommitted: uncommitted.into_iter().collect(),
        head: head.map(|h| h.id().to_string()),
    }))
}

/// The branch the team shares: what `origin/HEAD` points at, else `main` or `master` when
/// origin has one.
pub fn shared_branch(repo: &Repository) -> Option<String> {
    if let Ok(head) = repo.find_reference("refs/remotes/origin/HEAD")
        && let Some(target) = head.symbolic_target().ok().flatten()
        && let Some(name) = target.strip_prefix("refs/remotes/origin/")
    {
        return Some(name.to_owned());
    }
    ["main", "master"]
        .into_iter()
        .find(|b| remote_tip(repo, b).is_some())
        .map(str::to_owned)
}

fn remote_tip(repo: &Repository, branch: &str) -> Option<Oid> {
    repo.find_reference(&format!("refs/remotes/origin/{branch}"))
        .ok()?
        .peel_to_commit()
        .ok()
        .map(|c| c.id())
}

/// Commits reachable from `from` and not from `hide`, newest first.
fn walk(repo: &Repository, from: &[Oid], hide: &[Oid]) -> Result<Vec<Oid>, GitError> {
    if from.is_empty() {
        return Ok(Vec::new());
    }
    let mut walk = repo.revwalk()?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    for id in from {
        walk.push(*id)?;
    }
    for id in hide {
        walk.hide(*id)?;
    }
    Ok(walk.filter_map(Result::ok).collect())
}

fn blob_text(repo: &Repository, tree: &Tree, path: &str) -> Option<String> {
    let entry = tree.get_path(Path::new(path)).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    Some(String::from_utf8_lossy(blob.content()).into_owned())
}

/// The project's scenes that differ between two trees. A dialogue file is told by the labels
/// whose lines changed; any other of the project's own files as a whole.
fn scenes_between(
    r: &Repo,
    before: Option<&Tree>,
    after: Option<&Tree>,
    paths: &[String],
) -> Result<Vec<SceneRef>, GitError> {
    let repo = &r.repo;
    let diff = repo.diff_tree_to_tree(before, after, None)?;
    let mut out = BTreeSet::new();
    for delta in diff.deltas() {
        let file = delta.new_file().path().or_else(|| delta.old_file().path());
        let Some(in_repo) = file.map(|p| p.to_string_lossy().replace('\\', "/")) else {
            continue;
        };
        let Some(path) = r.in_project(&in_repo) else {
            continue;
        };
        if !path.ends_with(".dialogue") && !paths.contains(&path) {
            continue;
        }
        let old = before.and_then(|t| blob_text(repo, t, &in_repo));
        let new = after.and_then(|t| blob_text(repo, t, &in_repo));
        scenes_of(&path, old.as_deref(), new.as_deref(), &mut out);
    }
    Ok(out.into_iter().collect())
}

/// The scenes of one file that differ between two versions of it (none: no file).
fn scenes_of(path: &str, before: Option<&str>, after: Option<&str>, out: &mut BTreeSet<SceneRef>) {
    if !path.ends_with(".dialogue") {
        out.insert(SceneRef {
            path: path.to_owned(),
            label: None,
        });
        return;
    }
    let (before, after) = (before.unwrap_or(""), after.unwrap_or(""));
    let (old_titles, new_titles) = (titles(before), titles(after));
    let mut add = |titles: &[(usize, String)], line: usize| {
        let label = titles
            .iter()
            .rev()
            .find(|(at, _)| *at <= line)
            .map(|(_, name)| name.clone());
        out.insert(SceneRef {
            path: path.to_owned(),
            label,
        });
    };
    for op in TextDiff::from_lines(before, after).ops() {
        if matches!(op, DiffOp::Equal { .. }) {
            continue;
        }
        for line in op.old_range() {
            add(&old_titles, line);
        }
        for line in op.new_range() {
            add(&new_titles, line);
        }
    }
}

/// Each label's title: the line it is on (from 0), and its name.
fn titles(text: &str) -> Vec<(usize, String)> {
    let parse = es_syntax::parse(text);
    parse
        .syntax()
        .children()
        .filter(|n| n.kind() == TITLE_LINE)
        .filter_map(|node| {
            let name = ast::TitleLine::cast(node.clone())?
                .name()?
                .text()
                .to_owned();
            let start: usize = node.text_range().start().into();
            Some((text[..start].matches('\n').count(), name))
        })
        .collect()
}

/// Bring the team's changes in: this branch as it is on `origin` (pushed from elsewhere),
/// then the shared branch. A fast-forward when this branch has no commits of its own,
/// else a merge commit. Changes on disk not committed stay, merged with the team's when
/// they touch other lines; when any file was changed in the same lines by both, nothing at
/// all is changed and the files are named in [`Updated::conflicts`]. Brings in what the
/// last fetch got.
pub fn update(root: &Path, paths: &[String]) -> Result<Updated, GitError> {
    let r = Repo::open(root)?
        .ok_or_else(|| GitError("the project is not in a git repository".into()))?;
    let repo = &r.repo;
    if repo.state() != RepositoryState::Clean || !crate::git::conflicts(repo)?.is_empty() {
        return Err(GitError(
            "the repository is in the middle of a merge or rebase: finish it in git first".into(),
        ));
    }
    let head = repo.head()?;
    if !head.is_branch() {
        return Err(GitError("check out a branch first".into()));
    }
    let branch = head.shorthand()?.to_owned();
    let mut out = Updated::default();
    let shared = shared_branch(repo);
    let mut theirs: Vec<(String, Oid)> = Vec::new();
    if let Some(tip) = remote_tip(repo, &branch) {
        theirs.push((format!("origin/{branch}"), tip));
    }
    if let Some(shared) = shared.filter(|s| *s != branch)
        && let Some(tip) = remote_tip(repo, &shared)
    {
        theirs.push((format!("origin/{shared}"), tip));
    }
    let mut scenes = BTreeSet::new();
    for (name, tip) in theirs {
        let current = repo.head()?.peel_to_commit()?;
        if current.id() == tip || repo.graph_descendant_of(current.id(), tip)? {
            continue;
        }
        let commits = walk(repo, &[tip], &[current.id()])?.len();
        let step = integrate(&r, &branch, &name, &current, &repo.find_commit(tip)?)?;
        if !step.conflicts.is_empty() {
            return Ok(Updated {
                conflicts: step.conflicts,
                ..Updated::default()
            });
        }
        let now = repo.head()?.peel_to_commit()?;
        scenes.extend(scenes_between(
            &r,
            Some(&current.tree()?),
            Some(&now.tree()?),
            paths,
        )?);
        out.commits += commits;
        out.merged.extend(step.merged);
    }
    out.scenes = scenes.into_iter().collect();
    out.merged.sort();
    out.merged.dedup();
    Ok(out)
}

/// What one step of bringing changes in did to the files on disk.
struct Step {
    merged: Vec<String>,
    conflicts: Vec<String>,
}

/// A file to write (or remove: none) on disk, by its path in the repository.
struct Write {
    path: String,
    content: Option<Vec<u8>>,
    mode: u32,
    blob: Option<Oid>,
}

fn integrate(
    r: &Repo,
    branch: &str,
    name: &str,
    ours: &Commit,
    theirs: &Commit,
) -> Result<Step, GitError> {
    let repo = &r.repo;
    let forward = repo.graph_descendant_of(theirs.id(), ours.id())?;
    let tree = if forward {
        theirs.tree()?
    } else {
        let mut index = repo.merge_commits(ours, theirs, None)?;
        if index.has_conflicts() {
            let mut files: Vec<String> = index
                .conflicts()?
                .filter_map(Result::ok)
                .filter_map(|c| c.our.or(c.their).or(c.ancestor))
                .map(|e| String::from_utf8_lossy(&e.path).into_owned())
                .collect();
            files.dedup();
            return Ok(Step {
                merged: Vec::new(),
                conflicts: shown(r, files),
            });
        }
        repo.find_tree(index.write_tree_to(repo)?)?
    };

    // Plan every file first: nothing is written unless all of them can be.
    let old_tree = ours.tree()?;
    let diff = repo.diff_tree_to_tree(Some(&old_tree), Some(&tree), None)?;
    let mut writes = Vec::new();
    let mut merged = Vec::new();
    let mut conflicts = Vec::new();
    for delta in diff.deltas() {
        let Some(path) = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
        else {
            continue;
        };
        let blob = |t: &Tree| {
            t.get_path(Path::new(&path))
                .ok()
                .map(|e| (e.id(), e.filemode()))
        };
        let old = blob(&old_tree)
            .map(|(id, _)| repo.find_blob(id).map(|b| b.content().to_vec()))
            .transpose()?;
        let new = blob(&tree);
        let new_bytes = new
            .map(|(id, _)| repo.find_blob(id).map(|b| b.content().to_vec()))
            .transpose()?;
        let on_disk = std::fs::read(r.workdir().join(&path)).ok();
        let mode = new.map_or(0o100644, |(_, m)| u32::try_from(m).unwrap_or(0o100644));
        let content = if on_disk == old || on_disk == new_bytes {
            new_bytes
        } else {
            let three = (
                old.as_deref().map(std::str::from_utf8),
                on_disk.as_deref().map(std::str::from_utf8),
                new_bytes.as_deref().map(std::str::from_utf8),
            );
            match three {
                (Some(Ok(base)), Some(Ok(mine)), Some(Ok(team))) => {
                    match diffy::merge(base, mine, team) {
                        Ok(text) => {
                            merged.push(path.clone());
                            Some(text.into_bytes())
                        }
                        Err(_) => {
                            conflicts.push(path.clone());
                            continue;
                        }
                    }
                }
                _ => {
                    conflicts.push(path.clone());
                    continue;
                }
            }
        };
        writes.push(Write {
            path,
            content,
            mode,
            blob: new.map(|(id, _)| id),
        });
    }
    if !conflicts.is_empty() {
        return Ok(Step {
            merged: Vec::new(),
            conflicts: shown(r, conflicts),
        });
    }

    // The commit first, then the files: a failure half way leaves files to commit, never a
    // commit whose files are missing.
    if forward {
        repo.reference(
            &format!("refs/heads/{branch}"),
            theirs.id(),
            true,
            &format!("easyscript: bring in {name}"),
        )?;
    } else {
        let signature = repo.signature().map_err(|_| {
            GitError("git does not know who you are: set user.name and user.email".into())
        })?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &format!("Bring in the team's changes from {name}"),
            &tree,
            &[ours, theirs],
        )?;
    }
    let mut index = repo.index()?;
    for write in &writes {
        let full = r.workdir().join(&write.path);
        match &write.content {
            Some(bytes) => {
                if let Some(dir) = full.parent() {
                    std::fs::create_dir_all(dir)
                        .map_err(|e| GitError(format!("{}: {e}", write.path)))?;
                }
                if std::fs::read(&full).ok().as_ref() != Some(bytes) {
                    std::fs::write(&full, bytes)
                        .map_err(|e| GitError(format!("{}: {e}", write.path)))?;
                }
                if let Some(blob) = write.blob {
                    let size = repo.find_blob(blob)?.content().len();
                    index.add(&entry(&write.path, write.mode, blob, size))?;
                }
            }
            None => {
                let _ = std::fs::remove_file(&full);
                let _ = index.remove_path(Path::new(&write.path));
            }
        }
    }
    index.write()?;
    Ok(Step {
        merged: shown(r, merged),
        conflicts: Vec::new(),
    })
}

/// Repository paths as the writer knows them: relative to the project when inside it.
fn shown(r: &Repo, paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .map(|p| r.in_project(&p).unwrap_or(p))
        .collect()
}
