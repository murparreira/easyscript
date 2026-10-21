//! Git from the app: only the project's own files are committed; what the team staged by
//! hand stays staged; branches.

use std::fs;
use std::path::Path;

use es_core::git::{self, GitChange};
use git2::{Repository, Signature};

/// A repository with the project in `game/`, a README at its top, all committed.
fn repo() -> (tempfile::TempDir, Repository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    repo.config()
        .unwrap()
        .set_str("user.name", "Writer")
        .unwrap();
    repo.config()
        .unwrap()
        .set_str("user.email", "writer@example.com")
        .unwrap();
    fs::create_dir(dir.path().join("game")).unwrap();
    fs::write(dir.path().join("README.md"), "readme\n").unwrap();
    fs::write(dir.path().join("game/a.dialogue"), "~ a\nKeeper: One.\n").unwrap();
    fs::write(dir.path().join("game/other.txt"), "other\n").unwrap();
    {
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = Signature::now("Writer", "writer@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "start", &tree, &[])
            .unwrap();
    }
    (dir, repo)
}

fn head_file(repo: &Repository, path: &str) -> String {
    let tree = repo.head().unwrap().peel_to_tree().unwrap();
    let blob = tree
        .get_path(Path::new(path))
        .unwrap()
        .to_object(repo)
        .unwrap()
        .peel_to_blob()
        .unwrap();
    String::from_utf8(blob.content().to_vec()).unwrap()
}

#[test]
fn commits_only_the_projects_files() {
    let (dir, repo) = repo();
    let game = dir.path().join("game");
    fs::write(game.join("a.dialogue"), "~ a\nKeeper: Uno.\n").unwrap();
    fs::write(game.join("other.txt"), "changed by someone\n").unwrap();
    // Staged by hand, outside the app.
    fs::write(dir.path().join("README.md"), "readme, staged\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("README.md")).unwrap();
    index.write().unwrap();

    let status = git::status(&game, &["a.dialogue".into()]).unwrap().unwrap();
    assert_eq!(status.files.len(), 1);
    assert_eq!(
        (status.files[0].path.as_str(), status.files[0].change),
        ("a.dialogue", GitChange::Modified)
    );
    assert!(status.conflicts.is_empty());

    let id = git::commit(&game, &["a.dialogue".into()], "Keeper says uno").unwrap();
    assert_eq!(id.len(), 7);
    assert_eq!(head_file(&repo, "game/a.dialogue"), "~ a\nKeeper: Uno.\n");
    assert_eq!(
        head_file(&repo, "game/other.txt"),
        "other\n",
        "not ours: not committed"
    );
    assert_eq!(
        head_file(&repo, "README.md"),
        "readme\n",
        "staged by hand: not committed"
    );
    let staged = repo
        .statuses(None)
        .unwrap()
        .iter()
        .filter(|e| e.status().contains(git2::Status::INDEX_MODIFIED))
        .map(|e| e.path().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(staged, ["README.md"], "still staged");

    let after = git::status(&game, &["a.dialogue".into()]).unwrap().unwrap();
    assert!(after.files.is_empty());
    assert!(after.head.unwrap().ends_with("Keeper says uno"));
    assert!(
        git::commit(&game, &["a.dialogue".into()], "again").is_err(),
        "nothing to commit"
    );
}

#[test]
fn new_and_deleted_files() {
    let (dir, repo) = repo();
    let game = dir.path().join("game");
    fs::write(game.join("easyscript.toml"), "name = \"Game\"\n").unwrap();
    fs::remove_file(game.join("a.dialogue")).unwrap();
    let paths = vec!["easyscript.toml".to_owned(), "a.dialogue".to_owned()];
    let status = git::status(&game, &paths).unwrap().unwrap();
    let changes: Vec<_> = status
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.change))
        .collect();
    assert_eq!(
        changes,
        [
            ("a.dialogue", GitChange::Deleted),
            ("easyscript.toml", GitChange::Added)
        ]
    );
    git::commit(&game, &paths, "Project file").unwrap();
    assert_eq!(
        head_file(&repo, "game/easyscript.toml"),
        "name = \"Game\"\n"
    );
    assert!(
        repo.head()
            .unwrap()
            .peel_to_tree()
            .unwrap()
            .get_path(Path::new("game/a.dialogue"))
            .is_err()
    );
}

#[test]
fn branches() {
    let (dir, _repo) = repo();
    let game = dir.path().join("game");
    let first = git::status(&game, &[]).unwrap().unwrap().branch.unwrap();
    git::create_branch(&game, "rewrite-keeper").unwrap();
    let status = git::status(&game, &[]).unwrap().unwrap();
    assert_eq!(status.branch.as_deref(), Some("rewrite-keeper"));
    assert!(status.branches.contains(&first));
    git::switch_branch(&game, &first).unwrap();
    assert_eq!(
        git::status(&game, &[]).unwrap().unwrap().branch.as_deref(),
        Some(first.as_str())
    );
    assert!(git::create_branch(&game, "bad name..").is_err());
}

#[test]
fn not_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(git::status(dir.path(), &[]).unwrap(), None);
}
