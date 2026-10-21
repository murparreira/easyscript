//! Yarn Spinner 2 (<https://docs.yarnspinner.dev>): each node (`title:` ... `---` ...
//! `===`) becomes a title. Lines keep their speaker; `#line:` ids become `[ID:]`, other
//! hashtags tags; `-> options` become choices (`<<if>>` on one, its condition);
//! `<<if>>` / `<<elseif>>` / `<<else>>` / `<<endif>>` become `if` blocks; `<<set>>`,
//! `<<jump>>`, `<<detour>>`, `<<return>>` and `<<stop>>` their Dialogue Manager forms;
//! `{$x}` becomes `{{x}}`. Other commands are kept as comments.

use std::sync::LazyLock;

use regex::Regex;

use super::{Imported, Out, Titles, narration, speaker_and_text, tags};

static COMMAND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^<<\s*(\w+)\s*(.*?)\s*>>$").unwrap());
static TRAILING_IF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.*?)\s*<<\s*if\s+(.*?)\s*>>\s*$").unwrap());
static INLINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{([^{}]*)\}").unwrap());
static VARIABLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$([A-Za-z_]\w*)").unwrap());
static HASHTAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:\s+#[^\s#]+)+\s*$").unwrap());
static OLD_OPTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[\[(?:(.+?)\|)?(.+?)\]\]$").unwrap());

struct Node<'a> {
    title: String,
    tags: Option<String>,
    line: usize,
    body: Vec<(usize, &'a str)>,
}

/// What a line is nested in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Open {
    /// An option's body; the option's indentation in the source.
    Option(usize),
    If,
}

pub fn convert(text: &str) -> Imported {
    let mut out = Out::default();
    let nodes = parse(text, &mut out);
    let mut titles = Titles::default();
    for node in &nodes {
        titles.of(&node.title);
    }
    for node in &nodes {
        out.title(&titles.of(&node.title));
        if let Some(tags) = &node.tags {
            out.comment(0, &format!("Yarn tags: {tags}"));
        }
        body(node, &mut titles, &mut out);
    }
    out.finish()
}

fn body(node: &Node, titles: &mut Titles, out: &mut Out) {
    let mut open: Vec<Open> = Vec::new();
    let mut ends = false;
    for &(n, raw) in &node.body {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let indent = raw.len() - raw.trim_start().len();
        // Out of the options this line is not indented under.
        while let Some(Open::Option(at)) = open.last() {
            if indent > *at {
                break;
            }
            open.pop();
        }
        let depth = open.len();
        ends = false;

        if let Some(comment) = line.strip_prefix("//") {
            out.comment(depth, comment.trim());
            continue;
        }
        if let Some(option) = line.strip_prefix("->") {
            let (words, line_tags, id) = hashtags(option);
            let (words, condition) = match TRAILING_IF.captures(&words) {
                Some(c) => (c[1].to_owned(), Some(expression(&c[2]))),
                None => (words.clone(), None),
            };
            let condition = condition.map(|c| format!(" [if {c}]")).unwrap_or_default();
            let id = id.map(|id| format!(" [ID:{id}]")).unwrap_or_default();
            out.line(
                depth,
                &format!(
                    "- {}{condition}{}{id}",
                    text(&words, true),
                    tags(&line_tags)
                ),
            );
            open.push(Open::Option(indent));
            continue;
        }
        if let Some(c) = OLD_OPTION.captures(line) {
            // Yarn 1: `[[Text|Node]]`.
            let target = c[2].trim();
            let words = c.get(1).map_or(target, |m| m.as_str());
            let to = jump_target(target, titles, n, out);
            out.line(depth, &format!("- {} => {to}", text(words, true)));
            continue;
        }
        if let Some(c) = COMMAND.captures(line) {
            let (name, args) = (&c[1], c[2].trim());
            match name {
                "if" => {
                    out.line(depth, &format!("if {}", expression(args)));
                    open.push(Open::If);
                }
                "elseif" | "else" | "endif" => {
                    while let Some(top) = open.pop() {
                        if top == Open::If {
                            break;
                        }
                    }
                    let depth = open.len();
                    match name {
                        "elseif" => {
                            out.line(depth, &format!("elif {}", expression(args)));
                            open.push(Open::If);
                        }
                        "else" => {
                            out.line(depth, "else");
                            open.push(Open::If);
                        }
                        _ => {}
                    }
                }
                "set" => out.line(depth, &format!("set {}", assignment(args))),
                "jump" => {
                    let to = jump_target(args, titles, n, out);
                    out.line(depth, &format!("=> {to}"));
                    ends = depth == 0;
                }
                "detour" => {
                    let to = jump_target(args, titles, n, out);
                    out.line(depth, &format!("=>< {to}"));
                }
                "return" => {
                    out.line(depth, "=> END");
                    ends = depth == 0;
                }
                "stop" => {
                    out.line(depth, "=> END!");
                    ends = depth == 0;
                }
                "declare" => out.unconverted(
                    depth,
                    n,
                    line,
                    "a variable declaration (the game's state must have it)",
                ),
                "wait" => out.unconverted(depth, n, line, "`wait` (use [wait=...] in a line)"),
                _ => out.unconverted(depth, n, line, &format!("the command `{name}`")),
            }
            continue;
        }
        let (words, line_tags, id) = hashtags(line);
        if let Some(c) = TRAILING_IF.captures(&words) {
            // A line shown only when a condition holds.
            out.line(depth, &format!("if {}", expression(&c[2])));
            dialogue(&c[1], &line_tags, id, depth + 1, out);
            continue;
        }
        if words.contains("<<") {
            out.unconverted(depth, n, line, "a command inside a line");
            continue;
        }
        dialogue(&words, &line_tags, id, depth, out);
    }
    if !ends {
        // Yarn stops at the end of a node; Dialogue Manager would run into the next title.
        out.line(0, "=> END");
    }
}

fn dialogue(words: &str, line_tags: &[String], id: Option<String>, depth: usize, out: &mut Out) {
    let id = id.map(|id| format!(" [ID:{id}]")).unwrap_or_default();
    let line_tags = tags(line_tags);
    match speaker_and_text(words) {
        Some((speaker, said)) => out.line(
            depth,
            &format!("{speaker}: {}{line_tags}{id}", text(said, false)),
        ),
        None => out.line(
            depth,
            &format!("{}{line_tags}{id}", narration(&text(words, false))),
        ),
    }
}

/// A line's words with `{$x}` as `{{x}}`; a choice's colon escaped.
fn text(words: &str, choice: bool) -> String {
    let words = INLINE.replace_all(words.trim(), |c: &regex::Captures| {
        format!("{{{{{}}}}}", expression(&c[1]))
    });
    if choice {
        words.replace(": ", "\\: ")
    } else {
        words.into_owned()
    }
}

/// Hashtags at the end of a line: the words, the tags, and the `#line:` id.
fn hashtags(line: &str) -> (String, Vec<String>, Option<String>) {
    let Some(m) = HASHTAGS.find(line) else {
        return (line.trim().to_owned(), Vec::new(), None);
    };
    let mut tags = Vec::new();
    let mut id = None;
    for tag in m.as_str().split_whitespace() {
        let tag = tag.trim_start_matches('#');
        match tag.strip_prefix("line:") {
            Some(line_id) => id = Some(line_id.to_owned()),
            None => tags.push(tag.replace(':', "=")),
        }
    }
    (line[..m.start()].trim().to_owned(), tags, id)
}

/// A Yarn expression in Dialogue Manager's: `$x` as `x`, Yarn's word operators as
/// symbols.
fn expression(expr: &str) -> String {
    let expr = VARIABLE.replace_all(expr.trim(), "$1");
    let mut out = Vec::new();
    for word in expr.split(' ') {
        out.push(match word {
            "is" | "eq" => "==",
            "neq" => "!=",
            "gt" => ">",
            "lt" => "<",
            "gte" => ">=",
            "lte" => "<=",
            "xor" => "!=",
            w => w,
        });
    }
    out.join(" ")
}

/// `$x to 1` (or `$x = 1`) as `x = 1`.
fn assignment(args: &str) -> String {
    let args = match args.split_once(" to ") {
        Some((name, value)) => format!("{name} = {value}"),
        None => args.to_owned(),
    };
    expression(&args)
}

fn jump_target(name: &str, titles: &Titles, line: usize, out: &mut Out) -> String {
    let name = name.trim();
    match titles.get(name) {
        Some(id) => id.to_owned(),
        None if name.starts_with('{') => {
            let expr = expression(name.trim_matches(['{', '}']));
            format!("{{{{{expr}}}}}")
        }
        None => {
            out.warn(
                line,
                format!("no node is titled `{name}`: it ends the conversation"),
            );
            "END!".into()
        }
    }
}

fn parse<'a>(text: &'a str, out: &mut Out) -> Vec<Node<'a>> {
    let mut nodes = Vec::new();
    let mut header = true;
    let mut current: Option<Node> = None;
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let trimmed = line.trim();
        if header {
            if trimmed == "---" {
                header = false;
                if current.is_none() {
                    out.warn(n, "a node with no title: titled `untitled`");
                    current = Some(Node {
                        title: "untitled".into(),
                        tags: None,
                        line: n,
                        body: Vec::new(),
                    });
                }
            } else if let Some((key, value)) = trimmed.split_once(':') {
                let value = value.trim();
                match key.trim() {
                    "title" => {
                        current = Some(Node {
                            title: value.to_owned(),
                            tags: None,
                            line: n,
                            body: Vec::new(),
                        })
                    }
                    "tags" if !value.is_empty() => {
                        if let Some(node) = &mut current {
                            node.tags = Some(value.to_owned());
                        }
                    }
                    _ => {}
                }
            }
        } else if trimmed == "===" {
            header = true;
            nodes.extend(current.take());
        } else if let Some(node) = &mut current {
            node.body.push((n, line));
        }
    }
    if let Some(node) = current {
        out.warn(node.line, format!("node `{}` has no `===`", node.title));
        nodes.push(node);
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expressions() {
        assert_eq!(
            expression("$gold gt 5 and $name is \"Bo\""),
            "gold > 5 and name == \"Bo\""
        );
        assert_eq!(assignment("$gold to $gold + 1"), "gold = gold + 1");
    }

    #[test]
    fn line_tags() {
        assert_eq!(
            hashtags("Hi there. #line:a1b2 #happy #mood:sad"),
            (
                "Hi there.".into(),
                vec!["happy".into(), "mood=sad".into()],
                Some("a1b2".into())
            )
        );
    }
}
