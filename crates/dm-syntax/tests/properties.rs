//! Any text parses without panicking and prints back unchanged.

mod common;

use proptest::prelude::*;

/// Pieces of the grammar, so that generated text hits the parser's interesting paths
/// far more often than random characters would.
const FRAGMENTS: &[&str] = &[
    "\n",
    "\r\n",
    "\r",
    "\t",
    "    ",
    "  ",
    " ",
    "\\t",
    "\u{feff}",
    "~ ",
    "title",
    "=> ",
    "=>< ",
    "END",
    "- ",
    "| ",
    "% ",
    "%2 ",
    "if ",
    "elif ",
    "else",
    "else if ",
    "while ",
    "match ",
    "when ",
    "do ",
    "do! ",
    "set ",
    "$> ",
    "$>> ",
    "import ",
    "using ",
    " as ",
    "#",
    "## ",
    "Speaker",
    ": ",
    ":",
    "[#",
    "reaction=x",
    ", ",
    ",",
    "=",
    "]",
    "[",
    "[[",
    "]]",
    "|",
    "{{",
    "}}",
    "}",
    "{",
    "[if ",
    "[/if]",
    "[else]",
    "[do ",
    "[wave]",
    "[/wave]",
    "[ID:",
    "\\",
    "\\[",
    "\\:",
    "\"",
    "'",
    "&",
    "a.b(\"c\")",
    " and ",
    " not ",
    "==",
    "+=",
    "1.5",
    "?.",
    "(",
    ")",
    "ç",
    "…",
    "🙂",
    "é",
];

fn fragments() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(FRAGMENTS), 0..80).prop_map(|parts| parts.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4096))]

    #[test]
    fn any_string_round_trips(text in any::<String>()) {
        common::check(&text);
    }

    #[test]
    fn grammar_fragments_round_trip(text in fragments()) {
        common::check(&text);
    }
}

#[test]
fn deep_nesting_is_capped() {
    let text: String = (0..20_000)
        .map(|depth| format!("{}- a\n", "\t".repeat(depth % 2_000)))
        .collect();
    let parse = common::check(&text);
    let deepest = parse
        .syntax()
        .descendants()
        .map(|n| {
            n.ancestors()
                .filter(|a| a.kind() == dm_syntax::SyntaxKind::BLOCK)
                .count()
        })
        .max();
    // MAX_DEPTH lines deep: the deepest sits in one block fewer.
    assert_eq!(deepest, Some(dm_syntax::MAX_DEPTH - 1));
}
