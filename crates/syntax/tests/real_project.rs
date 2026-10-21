//! Every `.dialogue` file of a real project round-trips. The project's path comes from
//! `EASYSCRIPT_PROJECT`; without it the test does nothing (CI never has one).
//!
//! `cargo test -p es-syntax --test real_project -- --nocapture` also prints counts.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use es_syntax::{SyntaxKind, ast::Line};

#[test]
fn real_project_round_trips() {
    let Some(root) = std::env::var_os("EASYSCRIPT_PROJECT") else {
        eprintln!("skipped: EASYSCRIPT_PROJECT is not set");
        return;
    };
    // Relative to the workspace root, where the command is typed (tests run in the crate).
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(root);
    assert!(
        root.is_dir(),
        "EASYSCRIPT_PROJECT is not a directory: {}",
        root.display()
    );
    let mut files = Vec::new();
    find_dialogue_files(&root, &mut files);
    files.sort();
    assert!(!files.is_empty(), "no .dialogue files under {root:?}");

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut errors = Vec::new();
    for path in &files {
        let bytes = fs::read(path).expect("readable");
        let text =
            String::from_utf8(bytes).unwrap_or_else(|_| panic!("{} is not UTF-8", path.display()));
        let parse = common::check(&text);

        for line in parse.tree().all_lines() {
            let name = match &line {
                Line::Title(_) => "titles",
                Line::Goto(_) => "jumps (own line)",
                Line::Response(_) => "responses",
                Line::Dialogue(_) => "dialogue lines",
                Line::Mutation(_) => "mutations",
                Line::Condition(_) => "conditions",
                Line::Match(_) => "matches",
                Line::When(_) => "whens",
                Line::Comment(_) => "comments",
                _ => continue,
            };
            *counts.entry(name).or_default() += 1;
        }
        let syntax = parse.syntax();
        let lines_with = |kind| {
            syntax
                .descendants()
                .filter(|n| n.kind().is_line() && n.children().any(|c| c.kind() == kind))
                .count()
        };
        *counts.entry("lines with tags").or_default() += lines_with(SyntaxKind::TAG_GROUP);
        *counts.entry("lines with a speaker").or_default() += lines_with(SyntaxKind::SPEAKER);
        *counts.entry("inline jumps").or_default() += syntax
            .descendants()
            .filter(|n| {
                n.kind() == SyntaxKind::JUMP
                    && n.parent()
                        .is_some_and(|p| p.kind() != SyntaxKind::GOTO_LINE)
            })
            .count();

        for token in parse.errors() {
            let line = text[..usize::from(token.text_range().start())]
                .matches('\n')
                .count()
                + 1;
            errors.push(format!("{}:{line}: {:?}", path.display(), token.text()));
        }
    }

    eprintln!("{} files", files.len());
    for (name, count) in &counts {
        eprintln!("{count:>6} {name}");
    }
    assert!(errors.is_empty(), "unreadable text:\n{}", errors.join("\n"));
}

fn find_dialogue_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable directory") {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if path.is_dir() {
            if !name.starts_with('.') {
                find_dialogue_files(&path, out);
            }
        } else if name.ends_with(".dialogue") {
            out.push(path);
        }
    }
}
