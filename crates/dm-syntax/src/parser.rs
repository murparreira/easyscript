//! Builds the green tree: lines into blocks by indentation, then each line's parts.
//!
//! Every byte of the input goes into exactly one token, so the tree prints back to the
//! input. Nothing here fails: what does not fit becomes `TEXT` or `ERROR`.

use std::ops::Range;

use rowan::{GreenNode, GreenNodeBuilder};

use crate::SyntaxKind::{self, *};
use crate::expr;
use crate::line::{self, Line, LineType, RandomPrefix, is_blank};

pub(crate) fn parse(text: &str) -> GreenNode {
    let mut p = Parser {
        text,
        b: GreenNodeBuilder::new(),
    };
    p.b.start_node(SOURCE_FILE.into());
    let start = if text.starts_with('\u{feff}') { 3 } else { 0 };
    p.token(BOM, 0..start);
    p.lines(&line::split_lines(text, start));
    p.b.finish_node();
    p.b.finish()
}

struct Parser<'a> {
    text: &'a str,
    b: GreenNodeBuilder<'static>,
}

/// A line whose node is still open, so that deeper lines can go in its block.
struct Open {
    level: usize,
    has_block: bool,
}

/// Prefixes after which a leading `\` stops a dialogue line reading as something else.
const LINE_ESCAPES: [&str; 13] = [
    "using", "if", "elif", "else", "while", "match", "when", "do", "set", "-", "~", "=>", "%",
];

/// Mutation keywords, longest first.
const MUTATIONS: [(&str, SyntaxKind); 5] = [
    ("do!", DO_BANG_KW),
    ("do", DO_KW),
    ("set", SET_KW),
    ("$>>", DOLLAR2_KW),
    ("$>", DOLLAR_KW),
];

impl Parser<'_> {
    fn token(&mut self, kind: SyntaxKind, range: Range<usize>) {
        if !range.is_empty() {
            self.b.token(kind.into(), &self.text[range]);
        }
    }

    /// Whitespace from `pos` (up to `end`) as one token; returns where it stops.
    fn ws(&mut self, pos: usize, end: usize) -> usize {
        let run =
            self.text[pos..end].len() - self.text[pos..end].trim_start_matches(is_blank).len();
        self.token(WHITESPACE, pos..pos + run);
        pos + run
    }

    fn lines(&mut self, lines: &[Line]) {
        let mut open: Vec<Open> = Vec::new();
        for line in lines {
            while open.last().is_some_and(|o| line.level <= o.level) {
                self.close(open.pop().expect("checked"));
            }
            if let Some(parent) = open.last_mut()
                && !parent.has_block
            {
                self.b.start_node(BLOCK.into());
                parent.has_block = true;
            }
            self.b.start_node(line_kind(line.ty).into());
            self.line(line);
            if line.ty.is_trivia() {
                self.b.finish_node();
            } else {
                open.push(Open {
                    level: line.level,
                    has_block: false,
                });
            }
        }
        while let Some(o) = open.pop() {
            self.close(o);
        }
    }

    fn close(&mut self, open: Open) {
        if open.has_block {
            self.b.finish_node();
        }
        self.b.finish_node();
    }

    fn line(&mut self, line: &Line) {
        self.token(INDENT, line.indent.clone());
        self.token(WHITESPACE, line.lead.clone());
        let end = line.content.end;
        let mut pos = line.content.start;
        if let Some(prefix) = &line.random {
            pos = self.random_prefix(pos, prefix, end);
        }
        match line.ty {
            LineType::Blank | LineType::Random => {}
            LineType::Comment => self.token(COMMENT, pos..end),
            LineType::Import => self.import(pos, end),
            LineType::Using => {
                self.token(USING_KW, pos..pos + "using".len());
                let pos = self.ws(pos + "using".len(), end);
                self.name(pos..end);
            }
            LineType::Title => {
                self.token(TILDE, pos..pos + 1);
                let pos = self.ws(pos + 1, end);
                self.token(TITLE_NAME, pos..end);
            }
            LineType::Condition => {
                let rest = &self.text[pos..end];
                let (kind, len) = if rest.starts_with("if") {
                    (IF_KW, 2)
                } else if rest.starts_with("elif") {
                    (ELIF_KW, 4)
                } else {
                    (ELSE_KW, 4)
                };
                self.token(kind, pos..pos + len);
                let mut pos = pos + len;
                if kind == ELSE_KW {
                    // `else if x`
                    let after = pos + self.text[pos..end].len()
                        - self.text[pos..end].trim_start_matches(is_blank).len();
                    let rest = &self.text[after..end];
                    if after > pos
                        && rest.starts_with("if")
                        && rest[2..].chars().next().is_none_or(is_blank)
                    {
                        self.ws(pos, end);
                        self.token(IF_KW, after..after + 2);
                        pos = after + 2;
                    }
                }
                self.expr_tail(pos, end, true);
            }
            LineType::While | LineType::Match | LineType::When => {
                let (kind, len) = match line.ty {
                    LineType::While => (WHILE_KW, "while".len()),
                    LineType::Match => (MATCH_KW, "match".len()),
                    _ => (WHEN_KW, "when".len()),
                };
                self.token(kind, pos..pos + len);
                self.expr_tail(pos + len, end, true);
            }
            LineType::Mutation => {
                let rest = &self.text[pos..end];
                let (word, kind) = MUTATIONS
                    .into_iter()
                    .find(|(w, _)| rest.starts_with(w))
                    .expect("typed as mutation");
                self.token(kind, pos..pos + word.len());
                self.expr_tail(pos + word.len(), end, false);
            }
            LineType::Goto => {
                let pos = self.jump(pos, end);
                self.inline(pos..end, false);
            }
            LineType::Response => {
                self.token(DASH, pos..pos + 1);
                let pos = self.ws(pos + 1, end);
                self.inline(pos..end, true);
            }
            LineType::Dialogue => self.dialogue(pos, end),
        }
        self.token(WHITESPACE, line.trail.clone());
        self.token(NEWLINE, line.newline.clone());
    }

    /// `%`, weight, `[if ...]` and the whitespace after them; returns where the line goes on.
    fn random_prefix(&mut self, start: usize, prefix: &RandomPrefix, end: usize) -> usize {
        self.b.start_node(RANDOM_PREFIX.into());
        self.token(PERCENT, start..start + 1);
        let mut pos = start + 1;
        if let Some(weight) = &prefix.weight {
            self.token(NUMBER, weight.clone());
            pos = weight.end;
        }
        if let Some(condition) = &prefix.condition {
            self.token(WHITESPACE, pos..condition.start);
            self.inline_condition(condition.clone());
            pos = condition.end;
        }
        let pos = self.ws(pos, end);
        self.b.finish_node();
        pos
    }

    fn import(&mut self, mut pos: usize, end: usize) {
        self.token(IMPORT_KW, pos..pos + "import".len());
        pos = self.ws(pos + "import".len(), end);
        if self.text[pos..end].starts_with('"')
            && let Some(len) = self.text[pos + 1..end].find('"')
        {
            self.token(STRING, pos..pos + len + 2);
            pos = self.ws(pos + len + 2, end);
        }
        let rest = &self.text[pos..end];
        if rest.starts_with("as") && rest[2..].chars().next().is_none_or(is_blank) {
            self.token(AS_KW, pos..pos + 2);
            pos = self.ws(pos + 2, end);
        }
        self.name(pos..end);
    }

    /// A name that should be an identifier: `IDENT` when it is one, else `ERROR`.
    fn name(&mut self, range: Range<usize>) {
        let text = &self.text[range.clone()];
        let valid =
            text.starts_with(expr::is_ident_start) && text.chars().all(expr::is_ident_continue);
        self.token(if valid { IDENT } else { ERROR }, range);
    }

    /// `[WS] EXPR [WS] [:]` to the end of a keyword line.
    fn expr_tail(&mut self, pos: usize, end: usize, colon: bool) {
        let colon = colon && self.text[pos..end].ends_with(':');
        let body_end = if colon { end - 1 } else { end };
        let pos = self.ws(pos, body_end);
        let expr_end = pos + self.text[pos..body_end].trim_end_matches(is_blank).len();
        self.expr(pos..expr_end);
        self.token(WHITESPACE, expr_end..body_end);
        if colon {
            self.token(COLON, body_end..end);
        }
    }

    fn expr(&mut self, range: Range<usize>) {
        if range.is_empty() {
            return;
        }
        self.b.start_node(EXPR.into());
        let mut pos = range.start;
        for (kind, len) in expr::lex(&self.text[range]) {
            self.token(kind, pos..pos + len);
            pos += len;
        }
        self.b.finish_node();
    }

    /// Whitespace, an expression, whitespace: the inside of `[if ...]` or `{{ ... }}`.
    fn padded_expr(&mut self, range: Range<usize>) {
        let pos = self.ws(range.start, range.end);
        let expr_end = pos + self.text[pos..range.end].trim_end_matches(is_blank).len();
        self.expr(pos..expr_end);
        self.token(WHITESPACE, expr_end..range.end);
    }

    /// `=> target` or `=>< target` at `pos`; returns where it ends.
    fn jump(&mut self, pos: usize, end: usize) -> usize {
        self.b.start_node(JUMP.into());
        let arrow = if self.text[pos..end].starts_with("=><") {
            (ARROW_RETURN, 3)
        } else {
            (ARROW, 2)
        };
        self.token(arrow.0, pos..pos + arrow.1);
        let pos = self.ws(pos + arrow.1, end);
        let target = &self.text[pos..end];
        let end = match replacement_len(target) {
            Some(len) => {
                self.replacement(pos..pos + len);
                pos + len
            }
            None => {
                let len = target
                    .find(|c| is_blank(c) || c == '[')
                    .unwrap_or(target.len());
                self.token(JUMP_TARGET, pos..pos + len);
                pos + len
            }
        };
        self.b.finish_node();
        end
    }

    fn dialogue(&mut self, mut pos: usize, end: usize) {
        let rest = &self.text[pos..end];
        if let Some(after) = rest.strip_prefix('\\')
            && LINE_ESCAPES.iter().any(|k| after.starts_with(k))
        {
            self.token(ESCAPE, pos..pos + 1);
            pos += 1;
        }
        if self.text[pos..end].starts_with("| ") {
            self.token(PIPE, pos..pos + 1);
            pos = self.ws(pos + 1, end);
        }
        self.inline(pos..end, true);
    }

    /// Dialogue or choice text with its inline parts, and the speaker when `speaker` is set.
    fn inline(&mut self, range: Range<usize>, speaker: bool) {
        let pieces = pieces(self.text, range);
        let mut rest = &pieces[..];
        if speaker && let Some((index, colon)) = speaker_split(self.text, &pieces) {
            self.b.start_node(SPEAKER.into());
            for piece in &pieces[..index] {
                self.piece(piece);
            }
            let text = pieces[index].1.clone();
            self.text_run(text.start..colon);
            self.token(COLON, colon..colon + 1);
            self.b.finish_node();
            self.text_run(colon + 1..text.end);
            rest = &pieces[index + 1..];
        }
        for piece in rest {
            self.piece(piece);
        }
    }

    /// Text with any whitespace at its ends split off.
    fn text_run(&mut self, range: Range<usize>) {
        let pos = self.ws(range.start, range.end);
        let text_end = pos + self.text[pos..range.end].trim_end_matches(is_blank).len();
        self.token(TEXT, pos..text_end);
        self.token(WHITESPACE, text_end..range.end);
    }

    fn piece(&mut self, (piece, range): &(Piece, Range<usize>)) {
        let r = range.clone();
        match piece {
            Piece::Text => self.text_run(r),
            Piece::Escape => self.token(ESCAPE, r),
            Piece::Tags => self.tag_group(r),
            Piece::StaticId => {
                self.b.start_node(STATIC_ID.into());
                self.token(L_BRACK, r.start..r.start + 1);
                self.token(ID_KW, r.start + 1..r.start + 3);
                self.token(COLON, r.start + 3..r.start + 4);
                self.token(ID_VALUE, r.start + 4..r.end - 1);
                self.token(R_BRACK, r.end - 1..r.end);
                self.b.finish_node();
            }
            Piece::Random => {
                self.b.start_node(INLINE_RANDOM.into());
                self.token(L_DBRACK, r.start..r.start + 2);
                let mut pos = r.start + 2;
                for option in self.text[pos..r.end - 2].split('|') {
                    if pos > r.start + 2 {
                        self.token(PIPE, pos - 1..pos);
                    }
                    self.token(TEXT, pos..pos + option.len());
                    pos += option.len() + 1;
                }
                self.token(R_DBRACK, r.end - 2..r.end);
                self.b.finish_node();
            }
            Piece::Condition => self.inline_condition(r),
            Piece::Mutation => {
                self.b.start_node(INLINE_MUTATION.into());
                self.token(L_BRACK, r.start..r.start + 1);
                let inner = &self.text[r.start + 1..r.end];
                let (word, kind) = MUTATIONS
                    .into_iter()
                    .find(|(w, _)| inner.starts_with(w))
                    .expect("lexed as mutation");
                self.token(kind, r.start + 1..r.start + 1 + word.len());
                self.padded_expr(r.start + 1 + word.len()..r.end - 1);
                self.token(R_BRACK, r.end - 1..r.end);
                self.b.finish_node();
            }
            Piece::BbCode => {
                self.b.start_node(BBCODE.into());
                self.token(L_BRACK, r.start..r.start + 1);
                let inner = &self.text[r.start + 1..r.end - 1];
                let name = inner.find(|c| !is_bbcode_char(c)).unwrap_or(inner.len());
                self.token(BBCODE_NAME, r.start + 1..r.start + 1 + name);
                let pos = self.ws(r.start + 1 + name, r.end - 1);
                let args_end = pos + self.text[pos..r.end - 1].trim_end_matches(is_blank).len();
                self.token(BBCODE_ARGS, pos..args_end);
                self.token(WHITESPACE, args_end..r.end - 1);
                self.token(R_BRACK, r.end - 1..r.end);
                self.b.finish_node();
            }
            Piece::Replacement => self.replacement(r),
            Piece::Jump => {
                self.jump(r.start, r.end);
            }
        }
    }

    fn inline_condition(&mut self, r: Range<usize>) {
        self.b.start_node(INLINE_CONDITION.into());
        self.token(L_BRACK, r.start..r.start + 1);
        self.token(IF_KW, r.start + 1..r.start + 3);
        self.padded_expr(r.start + 3..r.end - 1);
        self.token(R_BRACK, r.end - 1..r.end);
        self.b.finish_node();
    }

    fn replacement(&mut self, r: Range<usize>) {
        self.b.start_node(REPLACEMENT.into());
        self.token(L_DBRACE, r.start..r.start + 2);
        self.padded_expr(r.start + 2..r.end - 2);
        self.token(R_DBRACE, r.end - 2..r.end);
        self.b.finish_node();
    }

    /// `[#a, b=c]`: tags split on commas, each `name=value` split on its first `=`.
    fn tag_group(&mut self, r: Range<usize>) {
        self.b.start_node(TAG_GROUP.into());
        self.token(L_BRACK, r.start..r.start + 1);
        self.token(HASH, r.start + 1..r.start + 2);
        let inner = r.start + 2..r.end - 1;
        let mut pos = inner.start;
        for part in self.text[inner.clone()].split(',') {
            if pos > inner.start {
                self.token(COMMA, pos - 1..pos);
            }
            self.tag(pos..pos + part.len());
            pos += part.len() + 1;
        }
        self.token(R_BRACK, r.end - 1..r.end);
        self.b.finish_node();
    }

    fn tag(&mut self, r: Range<usize>) {
        let mut pos = self.ws(r.start, r.end);
        if self.text[pos..r.end].starts_with('#') {
            self.token(HASH, pos..pos + 1);
            pos += 1;
        }
        let end = pos + self.text[pos..r.end].trim_end_matches(is_blank).len();
        if pos < end {
            self.b.start_node(TAG.into());
            match self.text[pos..end].find('=') {
                Some(eq) => {
                    let eq = pos + eq;
                    let name_end = pos + self.text[pos..eq].trim_end_matches(is_blank).len();
                    self.token(TAG_NAME, pos..name_end);
                    self.token(WHITESPACE, name_end..eq);
                    self.token(EQ, eq..eq + 1);
                    let value = self.ws(eq + 1, end);
                    self.token(TAG_VALUE, value..end);
                }
                None => self.token(TAG_NAME, pos..end),
            }
            self.b.finish_node();
        }
        self.token(WHITESPACE, end..r.end);
    }
}

fn line_kind(ty: LineType) -> SyntaxKind {
    match ty {
        LineType::Blank => BLANK_LINE,
        LineType::Comment => COMMENT_LINE,
        LineType::Import => IMPORT_LINE,
        LineType::Using => USING_LINE,
        LineType::Title => TITLE_LINE,
        LineType::Condition => CONDITION_LINE,
        LineType::While => WHILE_LINE,
        LineType::Match => MATCH_LINE,
        LineType::When => WHEN_LINE,
        LineType::Mutation => MUTATION_LINE,
        LineType::Goto => GOTO_LINE,
        LineType::Response => RESPONSE_LINE,
        LineType::Random => RANDOM_LINE,
        LineType::Dialogue => DIALOGUE_LINE,
    }
}

/// A stretch of dialogue text, or one inline construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Piece {
    Text,
    Escape,
    Tags,
    StaticId,
    Random,
    Condition,
    Mutation,
    BbCode,
    Replacement,
    Jump,
}

/// Split dialogue text into text runs and inline constructs (absolute ranges).
fn pieces(text: &str, range: Range<usize>) -> Vec<(Piece, Range<usize>)> {
    let s = &text[range.clone()];
    let base = range.start;
    let mut pieces = Vec::new();
    let mut text_start = 0;
    let mut i = 0;
    while i < s.len() {
        match construct_at(s, i) {
            Some((piece, len)) => {
                if text_start < i {
                    pieces.push((Piece::Text, base + text_start..base + i));
                }
                pieces.push((piece, base + i..base + i + len));
                i += len;
                text_start = i;
            }
            None => i += s[i..].chars().next().expect("in bounds").len_utf8(),
        }
    }
    if text_start < s.len() {
        pieces.push((Piece::Text, base + text_start..base + s.len()));
    }
    pieces
}

fn construct_at(s: &str, i: usize) -> Option<(Piece, usize)> {
    let rest = &s[i..];
    match rest.as_bytes()[0] {
        b'\\' if rest[1..].starts_with(['[', ']', ':', 'n']) => Some((Piece::Escape, 2)),
        b'[' => bracket_at(rest),
        b'{' => replacement_len(rest).map(|len| (Piece::Replacement, len)),
        // Dialogue Manager only sees a jump after a space: ` => title`.
        b'=' if s[..i].ends_with(is_blank)
            && (rest.starts_with("=> ") || rest.starts_with("=>< ")) =>
        {
            Some((Piece::Jump, jump_len(rest)))
        }
        _ => None,
    }
}

fn bracket_at(rest: &str) -> Option<(Piece, usize)> {
    if rest.starts_with("[[")
        && let Some(len) = rest[2..].find("]]")
    {
        return Some((Piece::Random, len + 4));
    }
    // Tags and static ids end at the first `]`, as Dialogue Manager's regexes do.
    if rest.starts_with("[#") {
        return rest.find(']').map(|len| (Piece::Tags, len + 1));
    }
    if rest.starts_with("[ID:") {
        return rest.find(']').map(|len| (Piece::StaticId, len + 1));
    }
    let close = balanced_close(rest)?;
    let inner = &rest[1..close];
    let piece = if inner.starts_with("if ") {
        Piece::Condition
    } else if MUTATIONS
        .iter()
        .any(|(w, _)| inner.starts_with(w) && inner[w.len()..].starts_with(' '))
    {
        Piece::Mutation
    } else if inner.starts_with(is_bbcode_char) {
        Piece::BbCode
    } else {
        return None;
    };
    Some((piece, close + 1))
}

/// Index of the `]` that closes the `[` at the start, skipping `\[` and `\]`.
fn balanced_close(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if matches!(bytes.get(i + 1), Some(b'[' | b']')) => i += 1,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// What Dialogue Manager reads as the name of a BBCode tag.
fn is_bbcode_char(c: char) -> bool {
    c.is_lowercase() || c == '/'
}

/// `{{ ... }}`, closing at the first `}}` but taking any `}` right after it into the
/// expression, as Dialogue Manager does.
fn replacement_len(s: &str) -> Option<usize> {
    if !s.starts_with("{{") {
        return None;
    }
    let mut close = 2 + s[2..].find("}}")?;
    while s[close + 2..].starts_with('}') {
        close += 1;
    }
    Some(close + 2)
}

/// Length of `=> target` / `=>< target` at the start of `s`.
fn jump_len(s: &str) -> usize {
    let arrow = if s.starts_with("=><") { 3 } else { 2 };
    let pos = arrow + s[arrow..].len() - s[arrow..].trim_start_matches(is_blank).len();
    let target = &s[pos..];
    pos + replacement_len(target).unwrap_or_else(|| {
        target
            .find(|c| is_blank(c) || c == '[')
            .unwrap_or(target.len())
    })
}

/// The first `: ` in the text before any jump: Dialogue Manager splits the speaker there.
/// Returns the index of the text piece and the position of the `:`.
fn speaker_split(text: &str, pieces: &[(Piece, Range<usize>)]) -> Option<(usize, usize)> {
    pieces
        .iter()
        .take_while(|(piece, _)| *piece != Piece::Jump)
        .enumerate()
        .filter(|(_, (piece, _))| *piece == Piece::Text)
        .find_map(|(index, (_, range))| {
            text[range.clone()]
                .find(": ")
                .map(|at| (index, range.start + at))
        })
}
