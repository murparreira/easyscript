//! Invariants every parse must hold, whatever the input.

use es_syntax::{AstNode, Parse, SyntaxKind, SyntaxNode, ast::Line, parse};

/// Parse `text` and check the tree against it; returns the parse.
pub fn check(text: &str) -> Parse {
    let parse = parse(text);
    let root = parse.syntax();
    assert_eq!(root.to_string(), text, "the tree prints back to the input");

    let lines: Vec<Line> = parse.tree().all_lines().collect();
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    assert_eq!(
        lines.len(),
        body.split_inclusive('\n').count(),
        "one line node per line"
    );

    for line in &lines {
        let own = line.own_text();
        let newlines = own.matches('\n').count();
        assert!(
            newlines <= 1 && (newlines == 0 || own.ends_with('\n')),
            "a line node holds one line: {own:?}"
        );
        if let Some(block) = line.block() {
            assert_eq!(
                line.syntax().last_child_or_token().map(|e| e.kind()),
                Some(SyntaxKind::BLOCK)
            );
            check_block(line, block.syntax());
        }
    }
    root.descendants()
        .filter(|n| n.kind() == SyntaxKind::BLOCK)
        .for_each(|b| assert!(b.parent().is_some_and(|p| p.kind().is_line())));
    parse
}

/// Lines in a block are deeper than the line that holds it (trivia aside).
fn check_block(parent: &Line, block: &SyntaxNode) {
    for child in block.children() {
        let child = Line::cast(child).expect("blocks hold lines");
        if !matches!(
            child,
            Line::Blank(_) | Line::Comment(_) | Line::Import(_) | Line::Using(_)
        ) {
            assert!(
                child.indent_level() > parent.indent_level(),
                "{:?} nested under {:?}",
                child.own_text(),
                parent.own_text()
            );
        }
    }
}
