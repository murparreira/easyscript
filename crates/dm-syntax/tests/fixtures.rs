//! Every fixture round-trips, holds the tree invariants, and matches its tree snapshot
//! (`fixtures/<name>.tree`; refresh with `UPDATE_EXPECT=1`).

mod common;

use std::fs;
use std::path::Path;

use expect_test::expect_file;

#[test]
fn fixtures() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut names: Vec<_> = fs::read_dir(&dir)
        .expect("fixtures directory")
        .map(|e| e.expect("entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "dialogue"))
        .collect();
    names.sort();
    assert!(!names.is_empty());

    for path in names {
        let text = fs::read_to_string(&path).expect("UTF-8 fixture");
        let parse = common::check(&text);
        let stem = path.file_stem().expect("stem").to_string_lossy();
        expect_file![format!("fixtures/{stem}.tree")].assert_eq(&format!("{:#?}", parse.syntax()));
    }
}
