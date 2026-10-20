//! Editing a project on disk: working copies, undo, saving only what changed, and never
//! over a file that changed on disk.

use std::fs;
use std::path::Path;

use dm_core::edit::Edit;
use dm_core::project_file::ConfigEdit;
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
    assert!(ws.state().changed.is_empty() && ws.state().conflicts.is_empty());
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

fn keeper() -> ConfigEdit {
    ConfigEdit::SetCharacter {
        id: "keeper".into(),
        name: "The Keeper".into(),
        speakers: vec!["Keeper".into()],
        portrait: None,
        role: None,
    }
}

#[test]
fn the_project_file_is_edited_like_the_dialogue() {
    let dir = project();
    let mut ws = Workspace::open(dir.path(), None).unwrap();
    assert!(!ws.state().has_project_file);

    // A dialogue edit, then a project file edit: the dialogue edit survives the rebuild.
    ws.edit(
        "a.dialogue",
        &Edit::SetText {
            at: line("~ a\n", 2),
            text: "Uno.".into(),
        },
    )
    .unwrap();
    ws.edit_project_file(&[keeper()]).unwrap();
    assert_eq!(ws.state().changed, ["dialogue-maker.toml", "a.dialogue"]);
    assert_eq!(ws.project().config.characters[0].name, "The Keeper");
    assert!(ws.project().files[0].text.contains("Uno."));
    assert!(
        !dir.path().join("dialogue-maker.toml").exists(),
        "nothing written yet"
    );

    ws.undo();
    assert!(ws.project().config.characters.is_empty());
    ws.redo();
    assert_eq!(ws.diffs()[0].path, "dialogue-maker.toml");

    assert_eq!(ws.save().unwrap(), ["dialogue-maker.toml", "a.dialogue"]);
    assert!(read(dir.path(), "dialogue-maker.toml").contains("name = \"The Keeper\""));
    assert!(ws.state().has_project_file && ws.state().changed.is_empty());
}

#[test]
fn a_new_project_file_is_not_written_over_one_that_appeared() {
    let dir = project();
    let mut ws = Workspace::open(dir.path(), None).unwrap();
    ws.draft_project_file().unwrap();
    assert!(
        ws.project()
            .config
            .characters
            .iter()
            .any(|c| c.name == "Keeper"),
        "the draft has the speaker"
    );
    assert!(ws.draft_project_file().is_err(), "only once");
    fs::write(
        dir.path().join("dialogue-maker.toml"),
        "name = \"Someone else's\"\n",
    )
    .unwrap();
    assert!(
        matches!(ws.save(), Err(SaveError::ChangedOnDisk(paths)) if paths == ["dialogue-maker.toml"])
    );
    assert_eq!(
        read(dir.path(), "dialogue-maker.toml"),
        "name = \"Someone else's\"\n"
    );
}

#[test]
fn refresh_takes_a_project_file_changed_on_disk_when_not_edited() {
    let dir = project();
    fs::write(dir.path().join("dialogue-maker.toml"), "name = \"One\"\n").unwrap();
    let mut ws = Workspace::open(dir.path(), None).unwrap();
    fs::write(dir.path().join("dialogue-maker.toml"), "name = \"Two\"\n").unwrap();
    ws.refresh().unwrap();
    assert_eq!(ws.project().config.name.as_deref(), Some("Two"));
    ws.edit_project_file(&[keeper()]).unwrap();
    fs::write(dir.path().join("dialogue-maker.toml"), "name = \"Three\"\n").unwrap();
    ws.refresh().unwrap();
    assert_eq!(ws.state().conflicts, ["dialogue-maker.toml"]);
    assert_eq!(
        ws.project().config.name.as_deref(),
        Some("Two"),
        "the edits stay"
    );
}
