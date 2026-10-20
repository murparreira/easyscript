//! Expressions (conditions, mutations, `{{ }}` replacements) as a flat token stream,
//! with the tokens of Dialogue Manager's `DMExpressionParser`. Never fails: what it
//! cannot read becomes `ERROR`.

use crate::SyntaxKind::{self, *};
use crate::line::is_blank;

/// Operators and punctuation, longest first.
const OPERATORS: &[(&str, SyntaxKind)] = &[
    ("==", EQ2),
    ("!=", NEQ),
    ("<=", LTEQ),
    (">=", GTEQ),
    ("+=", PLUS_EQ),
    ("-=", MINUS_EQ),
    ("*=", STAR_EQ),
    ("/=", SLASH_EQ),
    ("&&", AMP2),
    ("||", PIPE2),
    ("?.", QUESTION_DOT),
    ("(", L_PAREN),
    (")", R_PAREN),
    ("[", L_BRACK),
    ("]", R_BRACK),
    ("{", L_BRACE),
    ("}", R_BRACE),
    (",", COMMA),
    (".", DOT),
    (":", COLON),
    ("=", EQ),
    ("<", LT),
    (">", GT),
    ("+", PLUS),
    ("-", MINUS),
    ("*", STAR),
    ("/", SLASH),
    ("%", PERCENT),
    ("!", BANG),
];

/// Split an expression into `(kind, byte length)` tokens covering all of it.
pub(crate) fn lex(text: &str) -> Vec<(SyntaxKind, usize)> {
    let mut tokens = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let token = next_token(rest);
        tokens.push(token);
        rest = &rest[token.1..];
    }
    tokens
}

fn next_token(s: &str) -> (SyntaxKind, usize) {
    let c = s.chars().next().expect("not empty");
    if is_blank(c) {
        return (WHITESPACE, s.len() - s.trim_start_matches(is_blank).len());
    }
    if c == '#' {
        return (COMMENT, s.len());
    }
    let prefix = usize::from(c == '&');
    if let Some(quote) = s[prefix..]
        .chars()
        .next()
        .filter(|q| matches!(q, '"' | '\''))
    {
        // No escapes inside strings, as in Dialogue Manager.
        return match s[prefix + 1..].find(quote) {
            Some(i) => (STRING, prefix + 1 + i + 1),
            None => (ERROR, s.len()),
        };
    }
    if c.is_ascii_digit() {
        let int = digits(s);
        let fraction = s[int..]
            .strip_prefix('.')
            .map(digits)
            .filter(|&n| n > 0)
            .map_or(0, |n| n + 1);
        return (NUMBER, int + fraction);
    }
    if is_ident_start(c) {
        let len = s.find(|c| !is_ident_continue(c)).unwrap_or(s.len());
        return (keyword(&s[..len]).unwrap_or(IDENT), len);
    }
    OPERATORS
        .iter()
        .find(|(op, _)| s.starts_with(op))
        .map_or((ERROR, c.len_utf8()), |&(op, kind)| (kind, op.len()))
}

fn digits(s: &str) -> usize {
    s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len())
}

fn keyword(word: &str) -> Option<SyntaxKind> {
    Some(match word {
        "and" => AND_KW,
        "or" => OR_KW,
        "not" => NOT_KW,
        "in" => IN_KW,
        "true" => TRUE_KW,
        "false" => FALSE_KW,
        "null" => NULL_KW,
        "if" => IF_KW,
        "else" => ELSE_KW,
        _ => return None,
    })
}

/// Letters, `_`, and any other non-ASCII character that is not blank (Dialogue Manager
/// takes emoji and CJK in names).
pub(crate) fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic() || (!c.is_ascii() && !c.is_whitespace() && !c.is_control())
}

pub(crate) fn is_ident_continue(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<(SyntaxKind, &str)> {
        let mut pos = 0;
        lex(text)
            .into_iter()
            .map(|(kind, len)| {
                pos += len;
                (kind, &text[pos - len..pos])
            })
            .collect()
    }

    #[test]
    fn call_with_string() {
        assert_eq!(
            kinds(r#"Globals.has_game_flag("viu_cartao")"#),
            [
                (IDENT, "Globals"),
                (DOT, "."),
                (IDENT, "has_game_flag"),
                (L_PAREN, "("),
                (STRING, r#""viu_cartao""#),
                (R_PAREN, ")"),
            ]
        );
    }

    #[test]
    fn operators_and_keywords() {
        assert_eq!(
            kinds("not a and b >= 2.5 or c != &'x' # why"),
            [
                (NOT_KW, "not"),
                (WHITESPACE, " "),
                (IDENT, "a"),
                (WHITESPACE, " "),
                (AND_KW, "and"),
                (WHITESPACE, " "),
                (IDENT, "b"),
                (WHITESPACE, " "),
                (GTEQ, ">="),
                (WHITESPACE, " "),
                (NUMBER, "2.5"),
                (WHITESPACE, " "),
                (OR_KW, "or"),
                (WHITESPACE, " "),
                (IDENT, "c"),
                (WHITESPACE, " "),
                (NEQ, "!="),
                (WHITESPACE, " "),
                (STRING, "&'x'"),
                (WHITESPACE, " "),
                (COMMENT, "# why"),
            ]
        );
    }

    #[test]
    fn unreadable_text_is_kept() {
        assert_eq!(
            kinds("a $ \"open"),
            [
                (IDENT, "a"),
                (WHITESPACE, " "),
                (ERROR, "$"),
                (WHITESPACE, " "),
                (ERROR, "\"open")
            ]
        );
        assert_eq!(kinds("1."), [(NUMBER, "1"), (DOT, ".")]);
    }
}
