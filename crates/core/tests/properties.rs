//! Lowering and checking never panic, whatever the dialogue text: the app runs them on
//! every keystroke.

use std::path::Path;

use es_core::{Project, SourceFile, check, lower, stats};
use proptest::prelude::*;

const FRAGMENTS: &[&str] = &[
    "\n",
    "\t",
    "    ",
    " ",
    "~ ",
    "a",
    "b",
    "=> ",
    "=>< ",
    "END",
    "- ",
    "| ",
    "%",
    "% ",
    "%2 ",
    "if ",
    "elif ",
    "else",
    "else if ",
    "match ",
    "when ",
    "while ",
    "do ",
    "set ",
    "# ",
    "## ",
    "# === A ===",
    "Keeper",
    ": ",
    "[#",
    "face=x",
    ", ",
    "]",
    "[",
    "[[",
    "]]",
    "|",
    "{{",
    "}}",
    "[if ",
    "[/if]",
    "[else]",
    "[do ",
    "[ID:",
    "\\",
    "\"",
    "x(\"f\")",
    "has_flag(\"f\")",
    "set_flag(\"f\")",
    "import ",
    " as ",
    "using ",
    "✏ prod",
];

fn text() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(FRAGMENTS), 0..120).prop_map(|parts| parts.concat())
}

fn project(files: Vec<String>) -> Project {
    // The lighthouse fixture's project file, with generated files in place of its dialogue.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lighthouse");
    let mut project = Project::load(&root, None).expect("fixture loads");
    project.files = files
        .into_iter()
        .enumerate()
        .map(|(i, text)| SourceFile::new(format!("dialogue/f{i}.dialogue"), text))
        .collect();
    project
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    #[test]
    fn lowering_and_checks_never_panic(files in prop::collection::vec(text(), 1..3)) {
        let project = project(files);
        let script = lower(&project);
        check(&project, &script);
        stats(&script);
    }

    #[test]
    fn any_string_lowers(text in any::<String>()) {
        let project = project(vec![text]);
        check(&project, &lower(&project));
    }
}

#[test]
fn a_random_prefixed_comment_is_a_comment() {
    let project = project(vec![
        "~ a\n% Keeper: one\n% # a note\n% Keeper: two\n".into(),
    ]);
    let script = lower(&project);
    let body = &script.files[0].sections[0].labels[0].body;
    assert_eq!(
        body.len(),
        2,
        "two random groups around the comment: {body:#?}"
    );
}

/// Deep nesting stays within a small stack (Windows gives threads less than Linux does).
#[test]
fn deep_nesting_lowers_on_a_small_stack() {
    let text: String = (0..5_000)
        .map(|depth| format!("{}- a\n", "\t".repeat(depth % 1_000)))
        .collect();
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(move || {
            let project = project(vec![text]);
            let script = lower(&project);
            check(&project, &script);
            serde_json::to_string(&script).expect("serializes");
        })
        .expect("thread starts")
        .join()
        .expect("no overflow");
}
