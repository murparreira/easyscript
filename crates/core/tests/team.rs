//! Working with the team through `origin`: what came in, told by scene; what is not sent
//! yet; teammates' branches; bringing changes in without losing the writer's own.

use std::fs;
use std::path::{Path, PathBuf};

use es_core::team::{self, SceneRef};
use git2::{Repository, Signature};

const SCRIPT: &str = "~ a\nKeeper: One.\n\n~ b\nKeeper: Two.\n\n~ c\nKeeper: Three.\n";

/// A team: a shared repository (`origin`) holding the script, and two clones of it.
struct Team {
    _dir: tempfile::TempDir,
    ana: PathBuf,
    beto: PathBuf,
}

fn team() -> Team {
    let dir = tempfile::tempdir().unwrap();
    let seed = dir.path().join("seed");
    // `main` whatever git's default is on this machine.
    let repo = Repository::init_opts(
        &seed,
        git2::RepositoryInitOptions::new().initial_head("main"),
    )
    .unwrap();
    fs::write(seed.join("talk.dialogue"), SCRIPT).unwrap();
    fs::write(seed.join("code.gd"), "extends Node\n").unwrap();
    commit_all(&repo, "Murillo", "The script");
    // A bare copy is what a server has.
    let bare = dir.path().join("shared.git");
    git2::build::RepoBuilder::new()
        .bare(true)
        .clone(seed.to_str().unwrap(), &bare)
        .unwrap();
    let clone = |name: &str| {
        let path = dir.path().join(name);
        let repo = Repository::clone(bare.to_str().unwrap(), &path).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", name).unwrap();
        config
            .set_str("user.email", &format!("{name}@example.com"))
            .unwrap();
        path
    };
    Team {
        ana: clone("ana"),
        beto: clone("beto"),
        _dir: dir,
    }
}

fn commit_all(repo: &Repository, who: &str, message: &str) {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = Signature::now(who, &format!("{}@example.com", who.to_lowercase())).unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .unwrap();
}

/// Write a file, commit everything, push the branch checked out.
fn change_and_push(clone: &Path, who: &str, text: &str, message: &str) {
    fs::write(clone.join("talk.dialogue"), text).unwrap();
    let repo = Repository::open(clone).unwrap();
    commit_all(&repo, who, message);
    push(clone);
}

fn push(clone: &Path) {
    let repo = Repository::open(clone).unwrap();
    let branch = repo.head().unwrap().shorthand().unwrap().to_owned();
    let mut origin = repo.find_remote("origin").unwrap();
    origin
        .push(&[format!("refs/heads/{branch}:refs/heads/{branch}")], None)
        .unwrap();
}

fn fetch(clone: &Path) {
    let repo = Repository::open(clone).unwrap();
    let mut origin = repo.find_remote("origin").unwrap();
    origin
        .fetch(&["+refs/heads/*:refs/remotes/origin/*"], None, None)
        .unwrap();
}

fn paths() -> Vec<String> {
    vec!["talk.dialogue".into()]
}

fn scene(label: &str) -> SceneRef {
    SceneRef {
        path: "talk.dialogue".into(),
        label: Some(label.into()),
    }
}

fn head(clone: &Path) -> git2::Oid {
    Repository::open(clone)
        .unwrap()
        .head()
        .unwrap()
        .peel_to_commit()
        .unwrap()
        .id()
}

#[test]
fn what_came_in_is_told_by_scene() {
    let t = team();
    let start = head(&t.ana).to_string();
    change_and_push(
        &t.beto,
        "Beto",
        &SCRIPT.replace("Two.", "Dois."),
        "Beto's line",
    );
    let before = team::team(&t.ana, &paths(), None).unwrap().unwrap();
    assert!(before.incoming.is_empty(), "nothing fetched yet");
    fetch(&t.ana);
    let state = team::team(&t.ana, &paths(), Some(&start)).unwrap().unwrap();
    assert_eq!(state.shared.as_deref(), Some("main"));
    assert_eq!(state.branch.as_deref(), Some("main"));
    assert!(state.published);
    assert_eq!(state.incoming.len(), 1);
    let commit = &state.incoming[0];
    assert_eq!(
        (commit.author.as_str(), commit.summary.as_str(), commit.mine),
        ("Beto", "Beto's line", false)
    );
    assert_eq!(commit.scenes, [scene("b")]);
    assert!(state.outgoing.is_empty() && state.since.is_empty());
    assert_eq!(state.head.as_deref(), Some(start.as_str()));
}

#[test]
fn bringing_changes_in_keeps_the_writers_own() {
    let t = team();
    let start = head(&t.ana).to_string();
    change_and_push(
        &t.beto,
        "Beto",
        &SCRIPT.replace("Three.", "Tres."),
        "Beto's line",
    );
    fetch(&t.ana);
    // Ana changed another scene, saved and not committed.
    fs::write(t.ana.join("talk.dialogue"), SCRIPT.replace("One.", "Uno.")).unwrap();
    let state = team::team(&t.ana, &paths(), None).unwrap().unwrap();
    assert_eq!(state.uncommitted, [scene("a")]);

    let updated = team::update(&t.ana, &paths()).unwrap();
    assert!(updated.conflicts.is_empty());
    assert_eq!(updated.commits, 1);
    assert_eq!(updated.scenes, [scene("c")]);
    assert_eq!(updated.merged, ["talk.dialogue"]);
    assert_eq!(
        fs::read_to_string(t.ana.join("talk.dialogue")).unwrap(),
        SCRIPT.replace("One.", "Uno.").replace("Three.", "Tres.")
    );
    assert_eq!(head(&t.ana), head(&t.beto), "a fast-forward");

    let state = team::team(&t.ana, &paths(), Some(&start)).unwrap().unwrap();
    assert!(state.incoming.is_empty());
    assert_eq!(state.uncommitted, [scene("a")], "still Ana's to commit");
    assert_eq!(state.since.len(), 1);
    assert_eq!(state.since[0].scenes, [scene("c")]);
}

#[test]
fn the_same_lines_changed_on_both_sides_change_nothing() {
    let t = team();
    change_and_push(
        &t.beto,
        "Beto",
        &SCRIPT.replace("One.", "Um."),
        "Beto's line",
    );
    fetch(&t.ana);
    let mine = SCRIPT.replace("One.", "Uno.");
    fs::write(t.ana.join("talk.dialogue"), &mine).unwrap();
    let before = head(&t.ana);

    let updated = team::update(&t.ana, &paths()).unwrap();
    assert_eq!(updated.conflicts, ["talk.dialogue"]);
    assert_eq!(updated.commits, 0);
    assert_eq!(head(&t.ana), before);
    assert_eq!(
        fs::read_to_string(t.ana.join("talk.dialogue")).unwrap(),
        mine
    );
}

#[test]
fn commits_on_both_sides_are_merged() {
    let t = team();
    change_and_push(
        &t.beto,
        "Beto",
        &SCRIPT.replace("Three.", "Tres."),
        "Beto's line",
    );
    fs::write(t.ana.join("talk.dialogue"), SCRIPT.replace("One.", "Uno.")).unwrap();
    commit_all(&Repository::open(&t.ana).unwrap(), "ana", "Ana's line");
    fetch(&t.ana);
    let state = team::team(&t.ana, &paths(), None).unwrap().unwrap();
    assert_eq!(state.outgoing.len(), 1);
    assert!(state.outgoing[0].mine);
    assert_eq!(state.outgoing[0].scenes, [scene("a")]);

    let updated = team::update(&t.ana, &paths()).unwrap();
    assert_eq!(updated.commits, 1);
    let repo = Repository::open(&t.ana).unwrap();
    let merge = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(merge.parent_count(), 2);
    assert_eq!(
        fs::read_to_string(t.ana.join("talk.dialogue")).unwrap(),
        SCRIPT.replace("One.", "Uno.").replace("Three.", "Tres.")
    );
    let state = team::team(&t.ana, &paths(), None).unwrap().unwrap();
    assert!(state.incoming.is_empty() && state.uncommitted.is_empty());
    assert_eq!(state.outgoing.len(), 2, "Ana's commit and the merge");
}

#[test]
fn teammates_branches_are_told_until_merged() {
    let t = team();
    let beto = Repository::open(&t.beto).unwrap();
    let tip = beto.head().unwrap().peel_to_commit().unwrap();
    beto.branch("beto-taxi", &tip, false).unwrap();
    beto.set_head("refs/heads/beto-taxi").unwrap();
    change_and_push(
        &t.beto,
        "Beto",
        &SCRIPT.replace("Two.", "Dois."),
        "Work in progress",
    );
    // A branch changing only code is not the script's business.
    beto.branch("beto-code", &tip, false).unwrap();
    beto.set_head("refs/heads/beto-code").unwrap();
    beto.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .unwrap();
    fs::write(t.beto.join("code.gd"), "extends Node2D\n").unwrap();
    commit_all(&beto, "Beto", "Code");
    push(&t.beto);

    fetch(&t.ana);
    let state = team::team(&t.ana, &paths(), None).unwrap().unwrap();
    assert!(state.incoming.is_empty(), "branches are not the shared one");
    assert_eq!(state.others.len(), 1);
    let work = &state.others[0];
    assert_eq!(work.branch, "beto-taxi");
    assert_eq!(work.ahead, 1);
    assert_eq!(
        (work.last.author.as_str(), work.last.scenes.as_slice()),
        ("Beto", [scene("b")].as_slice())
    );

    // Merged into main: no longer work in progress.
    beto.set_head("refs/heads/main").unwrap();
    let taxi = beto
        .find_reference("refs/heads/beto-taxi")
        .unwrap()
        .peel_to_commit()
        .unwrap();
    beto.reference("refs/heads/main", taxi.id(), true, "merge")
        .unwrap();
    push(&t.beto);
    fetch(&t.ana);
    let state = team::team(&t.ana, &paths(), None).unwrap().unwrap();
    assert!(state.others.is_empty());
    assert_eq!(state.incoming.len(), 1);
}

#[test]
fn not_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    assert!(team::team(dir.path(), &[], None).unwrap().is_none());
}
