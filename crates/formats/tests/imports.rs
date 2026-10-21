//! Each importer on a small story: the Dialogue Manager file it writes (a snapshot), its
//! warnings, and that the file reads back with no syntax errors and no jump to nowhere.

use std::path::Path;

use dm_formats::import::{Imported, Source, import};
use expect_test::expect_file;

fn convert(name: &str) -> Imported {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/import")
        .join(name);
    let source = Source::of(&path).expect("a known extension");
    import(source, &std::fs::read_to_string(&path).unwrap())
}

/// The file parses without errors, and the checks find no jump to an unknown title.
/// Returns the speakers it has.
fn reads_back(imported: &Imported) -> Vec<String> {
    let parse = dm_syntax::parse(&imported.text);
    let errors: Vec<String> = parse.errors().map(|e| e.text().to_owned()).collect();
    assert!(
        errors.is_empty(),
        "syntax errors: {errors:?}\n{}",
        imported.text
    );

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("imported.dialogue"), &imported.text).unwrap();
    let project = dm_core::Project::load(dir.path(), None).unwrap();
    let script = dm_core::lower(&project);
    let jumps: Vec<String> = dm_core::check(&project, &script)
        .into_iter()
        .filter(|d| d.code == "unknown-jump-target")
        .map(|d| d.to_string())
        .collect();
    assert!(jumps.is_empty(), "{jumps:?}\n{}", imported.text);
    dm_core::stats(&script)
        .speakers
        .into_iter()
        .map(|s| s.name)
        .filter(|s| !s.is_empty())
        .collect()
}

fn warnings(imported: &Imported) -> String {
    imported
        .warnings
        .iter()
        .map(|w| format!("{}: {}\n", w.line, w.message))
        .collect()
}

#[test]
fn twee() {
    let imported = convert("cellar.twee");
    expect_file!["snapshots/cellar.dialogue"].assert_eq(&imported.text);
    expect_file!["snapshots/cellar.warnings"].assert_eq(&warnings(&imported));
    assert_eq!(
        reads_back(&imported),
        ["Ghost"],
        "a colon in narration stays narration"
    );
}

#[test]
fn yarn() {
    let imported = convert("gate.yarn");
    expect_file!["snapshots/gate.dialogue"].assert_eq(&imported.text);
    expect_file!["snapshots/gate.warnings"].assert_eq(&warnings(&imported));
    assert_eq!(reads_back(&imported), ["Guard"]);
}

#[test]
fn ink() {
    let imported = convert("shop.ink");
    expect_file!["snapshots/shop.dialogue"].assert_eq(&imported.text);
    expect_file!["snapshots/shop.warnings"].assert_eq(&warnings(&imported));
    let mut speakers = reads_back(&imported);
    speakers.sort();
    assert_eq!(speakers, ["Bob", "Shopkeeper"]);
}
