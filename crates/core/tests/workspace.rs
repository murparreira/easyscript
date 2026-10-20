//! Editing a project on disk: working copies, undo, saving only what changed, and never
//! over a file that changed on disk.

use std::fs;
use std::path::Path;

use dm_core::edit::Edit;
use dm_core::workspace::DiffKind;
use dm_core::{SaveError, Workspace};

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(
        dir.path().join("a.dialogue"),
        "~ a\nKeeper: One.\nKeeper: Two.\n",
    )
    .unwrap();
    fs::write(dir.path().join("b.dialogue"), "~ b\nKeeper: Three.\n").unwrap();
    dir
}

fn line(text: &str, n: usize) -> usize {
    text.split_inclusive('\n').take(n - 1).map(str::len).sum()
}

fn read(dir: &Path, name: &str) -> String {
    fs::read_to_string(dir.join(name)).unwrap()
}

#[test]
fn edits_stay_in_memory_until_saved() {
    let dir = project();
    let mut ws = Workspace::open(dir.path(), None).unwrap();
    let at = line("~ a\nKeeper: One.\n", 2);
    ws.edit(
        "a.dialogue",
        &Edit::SetText {
            at,
            text: "Uno.".into(),
        },
    )
    .unwrap();
    assert_eq!(
        read(dir.path(), "a.dialogue"),
        "~ a\nKeeper: One.\nKeeper: Two.\n",
        "nothing written yet"
    );
    assert_eq!(ws.state().changed, ["a.dialogue"]);

    let diffs = ws.diffs();
    assert_eq!(diffs.len(), 1);
    let lines: Vec<_> = diffs[0].hunks[0]
        .iter()
        .map(|l| (l.kind, l.text.as_str()))
        .collect();
    assert!(
        lines.contains(&(DiffKind::Removed, "Keeper: One."))
            && lines.contains(&(DiffKind::Added, "Keeper: Uno."))
    );

    let b_before = fs::metadata(dir.path().join("b.dialogue"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(ws.save().unwrap(), ["a.dialogue"]);
    assert_eq!(
        read(dir.path(), "a.dialogue"),
        "~ a\nKeeper: Uno.\nKeeper: Two.\n"
    );
    assert_eq!(
        fs::metadata(dir.path().join("b.dialogue"))
            .unwrap()
            .modified()
            .unwrap(),
        b_before,
        "b was not written"
    );
    assert!(ws.state().changed.is_empty());
}

#[test]
fn undo_and_redo() {
    let dir = project();
    let mut ws = Workspace::open(dir.path(), None).unwrap();
    let at = line("~ a\n", 2);
    ws.edit(
        "a.dialogue",
        &Edit::SetText {
            at,
            text: "Uno.".into(),
        },
    )
    .unwrap();
    ws.edit("a.dialogue", &Edit::RemoveLine { at }).unwrap();
    assert_eq!(ws.project().files[0].text, "~ a\nKeeper: Two.\n");
    ws.undo();
    assert_eq!(
        ws.project().files[0].text,
        "~ a\nKeeper: Uno.\nKeeper: Two.\n"
    );
    ws.undo();
    assert!(ws.state().changed.is_empty() && !ws.state().can_undo && ws.state().can_redo);
    ws.redo();
    assert_eq!(
        ws.project().files[0].text,
        "~ a\nKeeper: Uno.\nKeeper: Two.\n"
    );
    // A new edit drops what could be redone.
    ws.edit(
        "a.dialogue",
        &Edit::SetText {
            at,
            text: "Um.".into(),
        },
    )
    .unwrap();
    assert!(!ws.state().can_redo);
}

#[test]
fn never_saves_over_a_file_changed_on_disk() {
    let dir = project();
    let mut ws = Workspace::open(dir.path(), None).unwrap();
    ws.edit(
        "a.dialogue",
        &Edit::SetText {
            at: line("~ a\n", 2),
            text: "Uno.".into(),
        },
    )
    .unwrap();
    fs::write(
        dir.path().join("a.dialogue"),
        "~ a\nKeeper: Changed in Godot.\n",
    )
    .unwrap();

    let error = ws.save().unwrap_err();
    assert!(
        matches!(&error, SaveError::ChangedOnDisk(paths) if paths == &["a.dialogue"]),
        "{error}"
    );
    assert_eq!(
        read(dir.path(), "a.dialogue"),
        "~ a\nKeeper: Changed in Godot.\n",
        "left as it is"
    );

    // Reading the project again keeps the edits and marks the conflict; saving stays refused.
    ws.refresh().unwrap();
    assert_eq!(ws.state().conflicts, ["a.dialogue"]);
    assert_eq!(
        ws.project().files[0].text,
        "~ a\nKeeper: Uno.\nKeeper: Two.\n"
    );
    assert!(ws.save().is_err());

    // Discarding reads what is on disk now.
    ws.discard().unwrap();
    assert_eq!(
        ws.project().files[0].text,
        "~ a\nKeeper: Changed in Godot.\n"
    );
    assert_eq!(ws.state(), Default::default());
}

#[test]
fn refresh_takes_new_text_for_files_without_edits() {
    let dir = project();
    let mut ws = Workspace::open(dir.path(), None).unwrap();
    ws.edit(
        "a.dialogue",
        &Edit::SetText {
            at: line("~ a\n", 2),
            text: "Uno.".into(),
        },
    )
    .unwrap();
    fs::write(dir.path().join("b.dialogue"), "~ b\nKeeper: Four.\n").unwrap();
    ws.refresh().unwrap();
    let b = ws
        .project()
        .files
        .iter()
        .find(|f| f.path == "b.dialogue")
        .unwrap();
    assert_eq!(b.text, "~ b\nKeeper: Four.\n");
    assert!(ws.state().conflicts.is_empty());
    assert_eq!(ws.save().unwrap(), ["a.dialogue"]);
    assert_eq!(read(dir.path(), "b.dialogue"), "~ b\nKeeper: Four.\n");
}

#[test]
fn edits_name_lines_that_exist() {
    let dir = project();
    let mut ws = Workspace::open(dir.path(), None).unwrap();
    assert!(
        ws.edit("a.dialogue", &Edit::RemoveLine { at: 3 }).is_err(),
        "not the start of a line"
    );
    assert!(
        ws.edit("nope.dialogue", &Edit::RemoveLine { at: 0 })
            .is_err()
    );
    assert!(ws.state().changed.is_empty());
}
