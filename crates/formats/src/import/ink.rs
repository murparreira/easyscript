//! Ink (<https://github.com/inkle/ink/blob/master/Documentation/WritingWithInk.md>):
//! knots and stitches become titles (`knot_stitch`); the weave (choices `*` `+`,
//! gathers `-`, nested by their count) becomes indented choices; diverts become jumps
//! (`-> DONE` ends the title, `-> END` the conversation; tunnels `-> x ->` jump and come
//! back); `~` logic becomes `set` or `do`; conditional blocks become `if` or `match`;
//! inline `{x}`, `{cond: a|b}` and `{~a|b}` become `{{x}}`, `[if]` and `[[a|b]]`.
//!
//! What Ink has and Dialogue Manager does not is kept as a comment with a warning:
//! functions, threads, fallback choices, glue. Choices offered once only (`*`) are offered
//! every time, and sequences (`{a|b}`) pick at random.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use super::{Imported, Out, Titles, narration, speaker_and_text, tags};

static KNOT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^={2,}\s*(function\s+)?([A-Za-z_]\w*)\s*(\([^)]*\))?\s*=*\s*$").unwrap()
});
static STITCH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^=\s*([A-Za-z_]\w*)\s*(\([^)]*\))?\s*$").unwrap());
static LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\(\s*([A-Za-z_]\w*)\s*\)\s*").unwrap());
static DIVERT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*->\s*([A-Za-z_][\w.]*)\s*(->)?\s*$").unwrap());
static TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*#\s*([^#]*?)\s*(?:#|$)").unwrap());

pub fn convert(text: &str) -> Imported {
    let lines = strip_comments(text);
    let names = Names::collect(&lines);
    let mut c = Converter {
        out: Out::default(),
        names,
        knot: None,
        stitch: None,
        frames: vec![Frame::root(0)],
        skipping: false,
        top_written: false,
        warned_once_only: false,
        warned_glue: false,
    };
    for (i, line) in lines.iter().enumerate() {
        c.line(i + 1, line, lines.get(i + 1..).unwrap_or_default());
    }
    c.close_all(lines.len());
    c.out.finish()
}

/// The names diverts can go to, and their titles.
struct Names {
    /// `knot`, `knot.stitch`, `knot.label`, `knot.stitch.label`: title.
    by_path: HashMap<String, String>,
    /// The title for what comes before the first knot.
    top: String,
}

impl Names {
    fn collect(lines: &[String]) -> Self {
        let mut titles = Titles::default();
        let mut paths = Vec::new();
        let (mut knot, mut stitch): (Option<String>, Option<String>) = (None, None);
        for line in lines {
            let line = line.trim();
            if let Some(k) = KNOT.captures(line) {
                if k.get(1).is_none() {
                    knot = Some(k[2].to_owned());
                    stitch = None;
                    paths.push(k[2].to_owned());
                } else {
                    knot = None;
                }
            } else if let Some(s) = STITCH.captures(line) {
                if let Some(k) = &knot {
                    stitch = Some(s[1].to_owned());
                    paths.push(format!("{k}.{}", &s[1]));
                }
            } else if let Some(rest) = weave_rest(line)
                && let Some(l) = LABEL.captures(rest)
            {
                let scope = match (&knot, &stitch) {
                    (Some(k), Some(s)) => format!("{k}.{s}."),
                    (Some(k), None) => format!("{k}."),
                    _ => String::new(),
                };
                paths.push(format!("{scope}{}", &l[1]));
            }
        }
        let mut by_path = HashMap::new();
        for path in &paths {
            by_path.insert(path.clone(), titles.of(&path.replace('.', "_")));
        }
        let top = if by_path.contains_key("start") {
            titles.of("begin")
        } else {
            titles.of("start")
        };
        Self { by_path, top }
    }

    /// The title a divert goes to, from inside `knot` and `stitch`.
    fn resolve(&self, target: &str, knot: Option<&str>, stitch: Option<&str>) -> Option<&str> {
        let mut tries = Vec::new();
        if let (Some(k), Some(s)) = (knot, stitch) {
            tries.push(format!("{k}.{s}.{target}"));
        }
        if let Some(k) = knot {
            tries.push(format!("{k}.{target}"));
        }
        tries.push(target.to_owned());
        tries
            .iter()
            .find_map(|t| self.by_path.get(t))
            .map(String::as_str)
    }
}

/// What lines are nested in.
struct Frame {
    /// The indentation of the frame's own lines.
    indent: usize,
    /// How deep in the weave (choices and gathers) the lines are.
    weave: usize,
    block: Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Block {
    /// The knot itself.
    Root,
    /// `{ cond:` ... `- else:` ... `}`.
    If,
    /// `{` then `- cond:` arms.
    Chain { arms: usize },
    /// `{ subject:` then `- value:` arms.
    Switch,
}

impl Frame {
    fn root(indent: usize) -> Self {
        Self {
            indent,
            weave: 0,
            block: Block::Root,
        }
    }
}

struct Converter {
    out: Out,
    names: Names,
    knot: Option<String>,
    stitch: Option<String>,
    frames: Vec<Frame>,
    /// Inside a function: left out.
    skipping: bool,
    /// The title for what comes before the first knot is written (once there is some).
    top_written: bool,
    warned_once_only: bool,
    warned_glue: bool,
}

impl Converter {
    fn depth(&self) -> usize {
        let frame = self.frames.last().expect("the root");
        frame.indent + frame.weave
    }

    fn frame(&mut self) -> &mut Frame {
        self.frames.last_mut().expect("the root")
    }

    fn close_all(&mut self, n: usize) {
        if self.frames.len() > 1 {
            self.out.warn(n, "a `{` block is not closed");
        }
        self.frames.truncate(1);
        self.frames[0].weave = 0;
    }

    fn line(&mut self, n: usize, raw: &str, after: &[String]) {
        let line = raw.trim();
        if line.is_empty() {
            return;
        }
        if let Some(k) = KNOT.captures(line) {
            self.close_all(n);
            self.stitch = None;
            if k.get(1).is_some() {
                self.skipping = true;
                self.knot = None;
                self.out.blank();
                self.out.unconverted(
                    0,
                    n,
                    line,
                    &format!("the function `{}` (and its lines)", &k[2]),
                );
                return;
            }
            self.skipping = false;
            self.knot = Some(k[2].to_owned());
            if k.get(3).is_some() {
                self.out.warn(
                    n,
                    format!("the knot `{}` takes parameters: they are left out", &k[2]),
                );
            }
            let id = self.names.by_path[&k[2]].clone();
            self.out.title(&id);
            return;
        }
        if self.skipping {
            self.out.comment(0, line);
            return;
        }
        if let Some(s) = STITCH.captures(line)
            && let Some(knot) = self.knot.clone()
        {
            self.close_all(n);
            self.stitch = Some(s[1].to_owned());
            let id = self.names.by_path[&format!("{knot}.{}", &s[1])].clone();
            self.out.title(&id);
            return;
        }
        let depth = self.depth();
        for keyword in ["VAR ", "CONST ", "LIST ", "INCLUDE "] {
            if line.starts_with(keyword) {
                let why = if keyword == "INCLUDE " {
                    "an INCLUDE (import that file too)"
                } else {
                    "a declaration (the game's state must have it)"
                };
                self.out.unconverted(depth, n, line, why);
                return;
            }
        }
        if self.knot.is_none() && !self.top_written {
            self.top_written = true;
            let top = self.names.top.clone();
            self.out.title(&top);
        }
        if let Some(logic) = line.strip_prefix('~') {
            self.logic(n, logic.trim(), depth);
            return;
        }
        if line == "}" {
            if self.frames.len() > 1 {
                self.frames.pop();
            } else {
                self.out.warn(n, "a `}` with no `{`");
            }
            return;
        }
        if self.block_open(n, line, after) {
            return;
        }
        if line.starts_with('-') && !line.starts_with("->") && self.arm(n, line) {
            return;
        }
        if line.starts_with('*') || line.starts_with('+') {
            self.choice(n, line);
            return;
        }
        if let Some(rest) = gather(line) {
            let (count, rest) = rest;
            let frame = self.frame();
            frame.weave = count.saturating_sub(1).min(frame.weave);
            let depth = self.depth();
            let rest = self.label_anchor(rest, depth);
            if !rest.is_empty() {
                self.content(n, rest, depth);
            }
            return;
        }
        if line.starts_with('#') {
            self.out.comment(
                depth,
                &format!("tags: {}", line.trim_start_matches('#').trim()),
            );
            return;
        }
        self.content(n, line, depth);
    }

    /// `~ x = 1`, `~ x++`, `~ temp x = 1`, `~ f()`.
    fn logic(&mut self, n: usize, logic: &str, depth: usize) {
        if let Some(name) = logic.strip_suffix("++") {
            self.out.line(depth, &format!("set {} += 1", name.trim()));
        } else if let Some(name) = logic.strip_suffix("--") {
            self.out.line(depth, &format!("set {} -= 1", name.trim()));
        } else if let Some(rest) = logic.strip_prefix("temp ") {
            self.out.line(depth, &format!("set {}", expression(rest)));
            self.out
                .warn(n, "a temporary variable: the game's state must have it");
        } else if logic.starts_with("return") {
            self.out
                .unconverted(depth, n, logic, "a `return` outside a function");
        } else if is_assignment(logic) {
            self.out.line(depth, &format!("set {}", expression(logic)));
        } else {
            self.out.line(depth, &format!("do {}", expression(logic)));
        }
    }

    /// `{ cond:`, `{ subject:` with `- value:` arms, or `{` with `- cond:` arms.
    fn block_open(&mut self, n: usize, line: &str, after: &[String]) -> bool {
        if !line.starts_with('{') || line.contains('}') {
            return false;
        }
        let inside = line[1..].trim();
        let depth = self.depth();
        let next_is_arm = after
            .iter()
            .map(|l| l.trim())
            .find(|l| !l.is_empty())
            .is_some_and(|l| l.starts_with('-') && !l.starts_with("->"));
        let block = if inside.is_empty() {
            Block::Chain { arms: 0 }
        } else if let Some(condition) = inside.strip_suffix(':') {
            if next_is_arm {
                self.out
                    .line(depth, &format!("match {}", expression(condition)));
                Block::Switch
            } else {
                self.out
                    .line(depth, &format!("if {}", expression(condition)));
                Block::If
            }
        } else {
            self.out
                .unconverted(depth, n, line, "a block Dialogue Manager has no form for");
            return true;
        };
        // Arms go one in from the `if` (two from `match`, under its `when`s).
        let indent = depth + if block == Block::Switch { 2 } else { 1 };
        self.frames.push(Frame {
            indent,
            weave: 0,
            block,
        });
        true
    }

    /// `- cond:` (or `- else:`) inside a block: an arm. False when the line is a gather.
    fn arm(&mut self, n: usize, line: &str) -> bool {
        let frame = self.frames.last().expect("the root");
        if frame.block == Block::Root {
            return false;
        }
        let rest = line[1..].trim();
        let Some((head, body)) = split_arm(rest) else {
            return false;
        };
        let parent = frame.indent - 1;
        let keyword = match (&frame.block, head == "else") {
            (_, true) => "else".to_owned(),
            (Block::If, false) => return false,
            (Block::Chain { arms: 0 }, false) => format!("if {}", expression(head)),
            (Block::Chain { .. }, false) => format!("elif {}", expression(head)),
            (Block::Switch, false) => format!("when {}", expression(head)),
            (Block::Root, _) => unreachable!("checked above"),
        };
        self.out.line(parent, &keyword);
        let frame = self.frame();
        frame.weave = 0;
        if let Block::Chain { arms } = &mut frame.block {
            *arms += 1;
        }
        let body = body.trim();
        if !body.is_empty() {
            let depth = self.depth();
            self.content(n, body, depth);
        }
        true
    }

    fn choice(&mut self, n: usize, line: &str) {
        let (count, sticky, rest) = bullets(line);
        let frame = self.frame();
        frame.weave = (count - 1).min(frame.weave);
        let depth = self.depth();
        if !sticky && !self.warned_once_only {
            self.warned_once_only = true;
            self.out
                .warn(n, "choices offered once only (`*`) are offered every time");
        }
        let mut rest = rest.trim();
        let label = LABEL.captures(rest).map(|l| {
            let len = l[0].len();
            let name = l[1].to_owned();
            (name, len)
        });
        if let Some((_, len)) = &label {
            rest = rest[*len..].trim_start();
        }
        // Conditions: `{a} {b}` before the words.
        let mut conditions = Vec::new();
        while rest.starts_with('{') {
            let Some(end) = rest.find('}') else { break };
            conditions.push(expression(&rest[1..end]));
            rest = rest[end + 1..].trim_start();
        }
        let (rest, line_tags) = split_tags(rest);
        let (words, divert) = split_divert(&rest);
        if words.trim().is_empty() {
            self.out.unconverted(depth, n, line, "a fallback choice");
            return;
        }
        // `start[choice only]output only`.
        let (shown, echo) = match (words.find('['), words.find(']')) {
            (Some(open), Some(close)) if open < close => (
                format!("{}{}", &words[..open], &words[open + 1..close]),
                format!("{}{}", &words[..open], &words[close + 1..]),
            ),
            _ => (words.clone(), words.clone()),
        };
        let condition = match conditions.len() {
            0 => String::new(),
            1 => format!(" [if {}]", conditions[0]),
            _ => format!(" [if ({})]", conditions.join(") and (")),
        };
        let shown =
            inline(shown.trim(), n, &mut self.out, &mut self.warned_glue).replace(": ", "\\: ");
        self.out
            .line(depth, &format!("- {shown}{condition}{}", tags(&line_tags)));
        self.frame().weave = count;
        let inner = self.depth();
        if let Some((name, _)) = label {
            self.anchor(&name, inner);
        }
        let echo = echo.trim();
        if !echo.is_empty() {
            self.say(n, echo, &[], inner);
        }
        if let Some((target, tunnel)) = divert {
            self.divert(n, &target, tunnel, inner);
        }
    }

    /// A gather's or choice's `(label)`: an anchor to jump to.
    fn label_anchor<'l>(&mut self, rest: &'l str, depth: usize) -> &'l str {
        match LABEL.captures(rest) {
            Some(l) => {
                let len = l[0].len();
                self.anchor(&l[1], depth);
                rest[len..].trim_start()
            }
            None => rest,
        }
    }

    fn anchor(&mut self, name: &str, depth: usize) {
        if let Some(id) = self
            .names
            .resolve(name, self.knot.as_deref(), self.stitch.as_deref())
        {
            let id = id.to_owned();
            self.out.line(depth, &format!("~ {id}"));
        }
    }

    /// A line of text, maybe ending in a divert or tags.
    fn content(&mut self, n: usize, line: &str, depth: usize) {
        if line.starts_with("<-") {
            self.out.unconverted(depth, n, line, "a thread");
            return;
        }
        if line.trim() == "->->" {
            self.out.line(depth, "=> END");
            return;
        }
        let (rest, line_tags) = split_tags(line);
        let (words, divert) = split_divert(&rest);
        if !words.trim().is_empty() {
            self.say(n, words.trim(), &line_tags, depth);
        }
        if let Some((target, tunnel)) = divert {
            self.divert(n, &target, tunnel, depth);
        }
    }

    fn say(&mut self, n: usize, words: &str, line_tags: &[String], depth: usize) {
        let line_tags = tags(line_tags);
        match speaker_and_text(words) {
            Some((speaker, said)) => {
                let said = inline(said, n, &mut self.out, &mut self.warned_glue);
                self.out
                    .line(depth, &format!("{speaker}: {said}{line_tags}"));
            }
            None => {
                let said = inline(words, n, &mut self.out, &mut self.warned_glue);
                self.out
                    .line(depth, &format!("{}{line_tags}", narration(&said)));
            }
        }
    }

    fn divert(&mut self, n: usize, target: &str, tunnel: bool, depth: usize) {
        let to = match target {
            "DONE" => "END".to_owned(),
            "END" => "END!".to_owned(),
            _ => match self
                .names
                .resolve(target, self.knot.as_deref(), self.stitch.as_deref())
            {
                Some(id) => id.to_owned(),
                None => {
                    self.out.warn(
                        n,
                        format!("nothing is named `{target}`: the divert ends the conversation"),
                    );
                    "END!".to_owned()
                }
            },
        };
        let arrow = if tunnel { "=><" } else { "=>" };
        self.out.line(depth, &format!("{arrow} {to}"));
    }
}

/// After a line's choice or gather marks: the rest, when it is one.
fn weave_rest(line: &str) -> Option<&str> {
    if line.starts_with('*') || line.starts_with('+') {
        Some(bullets(line).2)
    } else {
        gather(line).map(|(_, rest)| rest)
    }
}

/// A gather: its depth and the rest of the line.
fn gather(line: &str) -> Option<(usize, &str)> {
    if !line.starts_with('-') || line.starts_with("->") {
        return None;
    }
    let mut count = 0;
    let mut rest = line;
    while let Some(r) = rest.strip_prefix('-') {
        if r.starts_with('>') {
            break;
        }
        count += 1;
        rest = r.trim_start();
    }
    Some((count, rest))
}

/// `* * text` / `++ text`: how many, sticky or not, the rest.
fn bullets(line: &str) -> (usize, bool, &str) {
    let sticky = line.starts_with('+');
    let mut count = 0;
    let mut rest = line;
    while let Some(r) = rest.strip_prefix(['*', '+']) {
        count += 1;
        rest = r.trim_start();
    }
    (count, sticky, rest)
}

/// `cond: body` or `else: body`, at the first `:` outside quotes and parentheses.
fn split_arm(rest: &str) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    let mut quoted = false;
    for (i, c) in rest.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            ':' if !quoted && depth == 0 => return Some((rest[..i].trim(), &rest[i + 1..])),
            _ => {}
        }
    }
    None
}

/// `text #a #b`: the text and the tags.
fn split_tags(line: &str) -> (String, Vec<String>) {
    let Some(at) = line.find('#') else {
        return (line.to_owned(), Vec::new());
    };
    let found: Vec<String> = TAGS
        .captures_iter(&line[at..])
        .map(|c| c[1].replace(':', "=").replace(' ', "_"))
        .filter(|t| !t.is_empty())
        .collect();
    (line[..at].trim_end().to_owned(), found)
}

/// `text -> target` or `text -> target ->` (a tunnel): the text and where it goes.
fn split_divert(line: &str) -> (String, Option<(String, bool)>) {
    match DIVERT.captures(line) {
        Some(c) => {
            let at = c.get(0).expect("a match").start();
            (
                line[..at].to_owned(),
                Some((c[1].to_owned(), c.get(2).is_some())),
            )
        }
        None => (line.to_owned(), None),
    }
}

fn is_assignment(logic: &str) -> bool {
    let Some(at) = logic.find('=') else {
        return false;
    };
    let before = logic[..at].trim_end();
    let next = logic[at + 1..].chars().next();
    next != Some('=')
        && !before.ends_with(['!', '<', '>', '='])
        && before
            .trim_end_matches(['+', '-', '*', '/'])
            .trim()
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
}

/// An Ink expression in Dialogue Manager's: `&&`, `||`, `!` and `mod` as words.
fn expression(expr: &str) -> String {
    let expr = expr.trim();
    let mut out = String::new();
    let mut chars = expr.chars().peekable();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                out.push(c);
            }
            '&' if !quoted && chars.peek() == Some(&'&') => {
                chars.next();
                out.push_str("and");
            }
            '|' if !quoted && chars.peek() == Some(&'|') => {
                chars.next();
                out.push_str("or");
            }
            '!' if !quoted && chars.peek() != Some(&'=') => out.push_str("not "),
            _ => out.push(c),
        }
    }
    out.replace(" mod ", " % ")
}

/// Inline Ink in a line: `{x}`, `{cond: a|b}`, `{~a|b}`, `{a|b}`, glue.
fn inline(text: &str, n: usize, out: &mut Out, warned_glue: &mut bool) -> String {
    let mut text = text.to_owned();
    if text.contains("<>") {
        text = text.replace(" <> ", " ").replace("<>", "");
        if !*warned_glue {
            *warned_glue = true;
            out.warn(n, "glue (`<>`) is left out: the lines stay apart");
        }
    }
    let mut result = String::new();
    let mut rest = text.as_str();
    while let Some(open) = rest.find('{') {
        result.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            result.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let inner = &after[..close];
        if inner.contains('{') {
            out.warn(n, "nested `{}` in a line: kept as written");
            result.push_str(&rest[open..open + 1 + close + 1]);
        } else {
            result.push_str(&inline_one(inner, n, out));
        }
        rest = &after[close + 1..];
    }
    result.push_str(rest);
    result
}

fn inline_one(inner: &str, n: usize, out: &mut Out) -> String {
    if let Some(options) = inner.strip_prefix('~') {
        return format!("[[{options}]]");
    }
    if let Some((head, body)) = split_arm(inner)
        && !head.contains('|')
    {
        return match body.split_once('|') {
            Some((yes, no)) => format!(
                "[if {}]{}[else]{}[/if]",
                expression(head),
                yes.trim(),
                no.trim()
            ),
            None => format!("[if {}]{}[/if]", expression(head), body.trim()),
        };
    }
    if inner.contains('|') {
        let options = inner.trim_start_matches(['&', '!']);
        out.warn(
            n,
            "a sequence (`{a|b}`) picks at random in Dialogue Manager",
        );
        return format!("[[{options}]]");
    }
    format!("{{{{{}}}}}", expression(inner))
}

/// The lines with `//` and `/* */` comments taken out (not inside quotes).
fn strip_comments(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        let mut out = String::new();
        let mut chars = line.chars().peekable();
        let mut quoted = false;
        while let Some(c) = chars.next() {
            if in_block {
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    in_block = false;
                }
                continue;
            }
            match c {
                '"' => {
                    quoted = !quoted;
                    out.push(c);
                }
                '/' if !quoted && chars.peek() == Some(&'/') => break,
                '/' if !quoted && chars.peek() == Some(&'*') => {
                    chars.next();
                    in_block = true;
                }
                _ => out.push(c),
            }
        }
        lines.push(out);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expressions() {
        assert_eq!(expression("a && !b || c != 2"), "a and not b or c != 2");
        assert!(is_assignment("x = 1"));
        assert!(is_assignment("x += 1"));
        assert!(!is_assignment("x == 1"));
        assert!(!is_assignment("f(x = 1)"));
    }

    #[test]
    fn inline_forms() {
        let mut out = Out::default();
        let mut glue = false;
        assert_eq!(
            inline(
                "You have {gold} coins, {gold > 5: rich|poor}. {~Hi|Hello}",
                1,
                &mut out,
                &mut glue
            ),
            "You have {{gold}} coins, [if gold > 5]rich[else]poor[/if]. [[Hi|Hello]]"
        );
    }

    #[test]
    fn comments() {
        assert_eq!(
            strip_comments("a // b\nc /* d\ne */ f \"//g\""),
            vec!["a ", "c ", " f \"//g\""]
        );
    }
}
