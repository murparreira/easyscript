//! Physical lines: where their parts are, how deep they sit, what type they are.
//!
//! Typing follows Dialogue Manager 3.10's `DMCompilation.get_line_type`, quirks included
//! (`elif` and `else` need no space after them, `~` and `if` do).

use std::ops::Range;

/// What a line is, as Dialogue Manager types it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineType {
    Blank,
    Comment,
    Import,
    Using,
    Title,
    Condition,
    While,
    Match,
    When,
    Mutation,
    Goto,
    Response,
    Random,
    Dialogue,
}

impl LineType {
    /// Lines that take no part in nesting: they sit at the level of the next line that does.
    pub(crate) fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::Blank | Self::Comment | Self::Import | Self::Using
        )
    }
}

/// One physical line, as byte offsets into the source.
#[derive(Debug, Clone)]
pub(crate) struct Line {
    /// Leading indentation.
    pub indent: Range<usize>,
    /// Whitespace between the indentation and the content (on blank lines, the rest of it).
    pub lead: Range<usize>,
    /// From the first to the last non-blank character.
    pub content: Range<usize>,
    /// Whitespace after the content, before the newline.
    pub trail: Range<usize>,
    /// `\n`, `\r\n`, or empty on a last line without one.
    pub newline: Range<usize>,
    pub ty: LineType,
    /// The weighted random prefix at the start of the content, if any.
    pub random: Option<RandomPrefix>,
    /// Nesting level from the indentation (for trivia, from the next non-trivia line).
    pub level: usize,
}

/// `%`, an optional weight and `[if ...]`, before a line's content. Offsets are absolute.
#[derive(Debug, Clone)]
pub(crate) struct RandomPrefix {
    pub weight: Option<Range<usize>>,
    /// The `[if ...]`, without the space before it.
    pub condition: Option<Range<usize>>,
    /// Where the prefix ends, after the space that closes it.
    pub end: usize,
}

/// Split `text` into lines and type them.
pub(crate) fn split_lines(text: &str, start: usize) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut pos = start;
    while pos < text.len() {
        let (text_end, end) = match text[pos..].find('\n') {
            Some(i) if i > 0 && text.as_bytes()[pos + i - 1] == b'\r' => (pos + i - 1, pos + i + 1),
            Some(i) => (pos + i, pos + i + 1),
            None => (text.len(), text.len()),
        };
        lines.push(read_line(text, pos, text_end, end));
        pos = end;
    }
    resolve_trivia_levels(&mut lines);
    lines
}

fn read_line(text: &str, start: usize, text_end: usize, end: usize) -> Line {
    let raw = &text[start..text_end];
    let indent_len = indent_len(raw);
    let after_indent = &raw[indent_len..];
    let lead_len = after_indent.len() - after_indent.trim_start_matches(is_blank).len();
    let content_start = start + indent_len + lead_len;
    let content_end = start
        + raw
            .trim_end_matches(is_blank)
            .len()
            .max(indent_len + lead_len);
    let newline = text_end..end;

    if content_start == content_end {
        return Line {
            indent: start..start + indent_len,
            lead: start + indent_len..text_end,
            content: text_end..text_end,
            trail: text_end..text_end,
            newline,
            ty: LineType::Blank,
            random: None,
            level: 0,
        };
    }

    let content = &text[content_start..content_end];
    let random = random_prefix(content, content_start);
    let rest = match &random {
        Some(prefix) => content[prefix.end - content_start..].trim_start_matches(is_blank),
        None => content,
    };
    Line {
        indent: start..start + indent_len,
        lead: start + indent_len..content_start,
        content: content_start..content_end,
        trail: content_end..text_end,
        newline,
        ty: line_type(content.starts_with('%'), rest),
        random,
        level: indent_level(&raw[..indent_len]),
    }
}

/// Trivia lines sit at the level of the next line that is not trivia (0 at the end).
fn resolve_trivia_levels(lines: &mut [Line]) {
    let mut next_level = 0;
    for line in lines.iter_mut().rev() {
        if line.ty.is_trivia() {
            line.level = next_level;
        } else {
            next_level = line.level;
        }
    }
}

/// Blank as Godot's `strip_edges` sees it: any ASCII control character or space.
pub(crate) fn is_blank(c: char) -> bool {
    c <= ' '
}

/// The indentation units Dialogue Manager knows, in the order it tries them.
const INDENT_UNITS: [&str; 4] = ["\t", "\\t", "    ", "  "];

/// Length of a line's indentation token: tabs and spaces, or a run of written-out `\t`.
fn indent_len(raw: &str) -> usize {
    if raw.starts_with("\\t") {
        let mut rest = raw;
        while let Some(r) = rest.strip_prefix("\\t") {
            rest = r;
        }
        raw.len() - rest.len()
    } else {
        raw.len() - raw.trim_start_matches([' ', '\t']).len()
    }
}

/// Nesting level of an indentation, as Dialogue Manager counts it: the first unit the
/// indentation starts with, counted while it repeats. Mixing units within a file is fine
/// (a line indented with four spaces and the next with a tab sit at the same level).
pub fn indent_level(indent: &str) -> usize {
    let Some(unit) = INDENT_UNITS.into_iter().find(|u| indent.starts_with(u)) else {
        return 0;
    };
    let mut level = 0;
    let mut rest = indent;
    while let Some(r) = rest.strip_prefix(unit) {
        level += 1;
        rest = r;
    }
    level
}

/// A weighted random prefix: `^%([\d.]+)?( \[if (.+?)\])? ` on the content plus a space.
/// `offset` is where `content` starts in the source.
fn random_prefix(content: &str, offset: usize) -> Option<RandomPrefix> {
    let bytes = content.as_bytes();
    if bytes.first() != Some(&b'%') {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
        i += 1;
    }
    let weight = (i > 1).then_some(1..i);
    let mut condition = None;
    if content[i..].starts_with(" [if ") {
        // The condition ends at the first `]` followed by a space or the end.
        let from = i + " [if ".len() + 1;
        if let Some(close) = (from..bytes.len())
            .find(|&j| bytes[j] == b']' && bytes.get(j + 1).is_none_or(|&b| b == b' '))
        {
            condition = Some(i + 1..close + 1);
            i = close + 1;
        }
    }
    let end = match bytes.get(i) {
        None => i,
        Some(b' ') => i + 1,
        Some(_) => return None,
    };
    let shift = |r: Range<usize>| r.start + offset..r.end + offset;
    Some(RandomPrefix {
        weight: weight.map(shift),
        condition: condition.map(shift),
        end: offset + end,
    })
}

/// `DMCompilation.get_line_type`, on the content with any random prefix removed.
fn line_type(starts_with_percent: bool, text: &str) -> LineType {
    let starts = |prefixes: &[&str]| prefixes.iter().any(|p| text.starts_with(p));
    if text.starts_with("import ") {
        LineType::Import
    } else if text.starts_with("using ") {
        LineType::Using
    } else if text.starts_with('#') {
        LineType::Comment
    } else if text.starts_with("~ ") {
        LineType::Title
    } else if starts(&["if ", "elif", "else"]) {
        LineType::Condition
    } else if text.starts_with("while ") {
        LineType::While
    } else if text.starts_with("match ") {
        LineType::Match
    } else if text.starts_with("when ") {
        LineType::When
    } else if starts(&["do ", "do! ", "set ", "$> ", "$>> "]) {
        LineType::Mutation
    } else if starts(&["=> ", "=>< "]) {
        LineType::Goto
    } else if text.starts_with("- ") {
        LineType::Response
    } else if text.is_empty() && starts_with_percent {
        LineType::Random
    } else {
        LineType::Dialogue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels() {
        assert_eq!(indent_level(""), 0);
        assert_eq!(indent_level(" "), 0);
        assert_eq!(indent_level("\t\t"), 2);
        assert_eq!(indent_level("    "), 1);
        assert_eq!(indent_level("        "), 2);
        assert_eq!(indent_level("  "), 1);
        // Four spaces are tried before two: six spaces are one level, not three.
        assert_eq!(indent_level("      "), 1);
        assert_eq!(indent_level("    \t"), 1);
        assert_eq!(indent_level("\\t\\t"), 2);
    }

    #[test]
    fn random_prefixes() {
        let random_prefix = |s| random_prefix(s, 0).map(|p| (p.weight, p.condition, p.end));
        assert_eq!(random_prefix("%"), Some((None, None, 1)));
        assert_eq!(random_prefix("% Hi"), Some((None, None, 2)));
        assert_eq!(random_prefix("%2.5 Hi"), Some((Some(1..4), None, 5)));
        assert_eq!(random_prefix("% [if a] Hi"), Some((None, Some(2..8), 9)));
        assert_eq!(
            random_prefix("%3 [if b[\"x\"]] Hi"),
            Some((Some(1..2), Some(3..14), 15))
        );
        assert_eq!(random_prefix("%abc"), None);
        assert_eq!(random_prefix("%2x"), None);
        assert_eq!(random_prefix("Hi"), None);
    }

    #[test]
    fn types() {
        let ty = |s: &str| split_lines(s, 0)[0].ty;
        assert_eq!(ty("~ start"), LineType::Title);
        assert_eq!(ty("~start"), LineType::Dialogue);
        assert_eq!(ty("if x"), LineType::Condition);
        assert_eq!(ty("elif x"), LineType::Condition);
        assert_eq!(ty("else:"), LineType::Condition);
        assert_eq!(ty("ifx"), LineType::Dialogue);
        assert_eq!(ty("do x()"), LineType::Mutation);
        assert_eq!(ty("$>> x()"), LineType::Mutation);
        assert_eq!(ty("=> END"), LineType::Goto);
        assert_eq!(ty("=>< snippet"), LineType::Goto);
        assert_eq!(ty("=>"), LineType::Dialogue);
        assert_eq!(ty("- Choice"), LineType::Response);
        assert_eq!(ty("%"), LineType::Random);
        assert_eq!(ty("%2"), LineType::Random);
        assert_eq!(ty("% Hi"), LineType::Dialogue);
        assert_eq!(ty("% => x"), LineType::Goto);
        assert_eq!(ty("  # note"), LineType::Comment);
        assert_eq!(ty(" \t "), LineType::Blank);
        assert_eq!(ty("\\if"), LineType::Dialogue);
    }

    #[test]
    fn trivia_takes_the_next_level() {
        let lines = split_lines("- a\n\n# c\n\tb\n# d\n", 0);
        let levels: Vec<_> = lines.iter().map(|l| l.level).collect();
        assert_eq!(levels, [0, 1, 1, 1, 0]);
    }
}
