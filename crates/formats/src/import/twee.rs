//! Twine 2's Twee 3 (<https://github.com/iftechfoundation/twine-specs>): each passage
//! (`:: Name [tags] {metadata}`) becomes a title, its lines narration (or `Name: text`
//! lines, spoken), its links choices. A passage with no links ends there. Variables
//! printed in the text (`$gold`) become `{{gold}}`, and `<<set>>` / `(set:)` become `set`;
//! other story-format macros are kept as comments.

use std::sync::LazyLock;

use regex::Regex;

use super::{Imported, Out, Titles, narration, speaker_and_text};

struct Passage<'a> {
    name: String,
    tags: Vec<String>,
    /// The header's line, from 1.
    line: usize,
    /// Lines with their numbers.
    body: Vec<(usize, &'a str)>,
}

/// Passages that are the story's settings, not its text.
const SPECIAL: [&str; 2] = ["StoryTitle", "StoryData"];

/// Tags of passages that hold code or styles.
const NOT_TEXT: [&str; 4] = ["script", "stylesheet", "widget", "Twine.private"];

static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[(.+?)\]\](\[.*?\])?").unwrap());
static VARIABLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^|[^\w$])\$([A-Za-z_][\w.]*)").unwrap());
static SET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:<<set\s+(.+?)\s*>>|\(set:\s*(.+?)\s*\))\s*$").unwrap());
static MACRO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<<|>>|\([a-z][\w-]*:").unwrap());

pub fn convert(text: &str) -> Imported {
    let passages = parse(text);
    let mut out = Out::default();
    let mut titles = Titles::default();

    let setting = |name: &str| passages.iter().find(|p| p.name == name);
    if let Some(title) = setting("StoryTitle") {
        let name: Vec<&str> = title.body.iter().map(|(_, l)| l.trim()).collect();
        out.comment(
            0,
            &format!("Imported from Twine: {}", name.join(" ").trim()),
        );
    }
    let story: Vec<&Passage> = passages
        .iter()
        .filter(|p| !SPECIAL.contains(&p.name.as_str()))
        .collect();
    for passage in &story {
        titles.of(&passage.name);
    }
    if let Some(data) = setting("StoryData") {
        let json: String = data
            .body
            .iter()
            .map(|(_, l)| *l)
            .collect::<Vec<_>>()
            .join("\n");
        let start = serde_json::from_str::<serde_json::Value>(&json)
            .ok()
            .and_then(|v| v["start"].as_str().map(str::to_owned));
        if let Some(id) = start.as_deref().and_then(|s| titles.get(s)) {
            out.comment(0, &format!("The story starts at ~ {id}."));
        }
    }
    out.blank();

    for passage in story {
        if let Some(tag) = passage.tags.iter().find(|t| NOT_TEXT.contains(&t.as_str())) {
            out.warn(
                passage.line,
                format!("passage `{}` is a {tag}: left out", passage.name),
            );
            continue;
        }
        let id = titles.of(&passage.name);
        out.title(&id);
        if !passage.tags.is_empty() {
            out.comment(0, &format!("Twine tags: {}", passage.tags.join(" ")));
        }
        let mut links: Vec<(usize, String, String)> = Vec::new();
        for &(n, line) in &passage.body {
            if line.trim().is_empty() {
                continue;
            }
            if let Some(set) = SET.captures(line) {
                let expr = set.get(1).or(set.get(2)).map_or("", |m| m.as_str());
                out.line(0, &format!("set {}", assignment(expr)));
                continue;
            }
            // Links: taken out of the text, offered as choices after it.
            let mut rest = String::new();
            let mut last = 0;
            for link in LINK.captures_iter(line) {
                let whole = link.get(0).expect("a match");
                let (label, target) = parse_link(&link[1]);
                if link.get(2).is_some() {
                    out.warn(
                        n,
                        format!("the link to `{target}` sets variables: left out"),
                    );
                }
                rest.push_str(&line[last..whole.start()]);
                rest.push_str(&label);
                last = whole.end();
                links.push((n, label, target));
            }
            rest.push_str(&line[last..]);
            let only_links = LINK.replace_all(line, "").trim().is_empty();
            if only_links {
                continue;
            }
            if MACRO.is_match(&rest) {
                out.unconverted(0, n, line, "a story-format macro");
                continue;
            }
            let rest = VARIABLE.replace_all(&rest, "$1{{$2}}");
            match speaker_and_text(&rest) {
                Some((speaker, words)) => out.line(0, &format!("{speaker}: {words}")),
                None => out.line(0, &narration(&rest)),
            }
        }
        if links.is_empty() {
            out.line(0, "=> END");
        }
        for (n, label, target) in links {
            match titles.get(&target) {
                Some(id) => {
                    let id = id.to_owned();
                    out.line(0, &format!("- {} => {id}", choice_text(&label)));
                }
                None => {
                    out.warn(
                        n,
                        format!("no passage is named `{target}`: the choice ends the story"),
                    );
                    out.line(0, &format!("- {} => END", choice_text(&label)));
                }
            }
        }
    }
    out.finish()
}

/// `$x to 5` (SugarCube, Harlowe) as `x = 5`.
fn assignment(expr: &str) -> String {
    let expr = expr.replace(" to ", " = ");
    VARIABLE.replace_all(&expr, "$1$2").trim().to_owned()
}

/// A choice's words: `[` would open markup, a colon and space a speaker.
fn choice_text(label: &str) -> String {
    let label = VARIABLE.replace_all(label.trim(), "$1{{$2}}");
    label.replace(": ", "\\: ")
}

/// `[[Text|Target]]`, `[[Text->Target]]`, `[[Target<-Text]]`, `[[Target]]`: (text, target).
fn parse_link(inner: &str) -> (String, String) {
    if let Some((text, target)) = inner.rsplit_once('|') {
        return (text.trim().into(), target.trim().into());
    }
    if let Some((text, target)) = inner.rsplit_once("->") {
        return (text.trim().into(), target.trim().into());
    }
    if let Some((target, text)) = inner.split_once("<-") {
        return (text.trim().into(), target.trim().into());
    }
    (inner.trim().into(), inner.trim().into())
}

fn parse(text: &str) -> Vec<Passage<'_>> {
    let mut passages: Vec<Passage> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if let Some(header) = line.strip_prefix("::") {
            let (name, tags) = header_parts(header);
            passages.push(Passage {
                name,
                tags,
                line: i + 1,
                body: Vec::new(),
            });
        } else if let Some(passage) = passages.last_mut() {
            passage.body.push((i + 1, line));
        }
    }
    // Blank lines between passages belong to none.
    for passage in &mut passages {
        while passage
            .body
            .last()
            .is_some_and(|(_, l)| l.trim().is_empty())
        {
            passage.body.pop();
        }
    }
    passages
}

/// A header's name and tags (`Name [a b] {"position":"1,2"}`), escapes undone.
fn header_parts(header: &str) -> (String, Vec<String>) {
    let mut rest = header.trim();
    // Metadata: a JSON object at the end.
    if rest.ends_with('}')
        && let Some(at) = unescaped_find(rest, '{')
    {
        rest = rest[..at].trim_end();
    }
    let mut tags = Vec::new();
    if rest.ends_with(']')
        && let Some(at) = unescaped_find(rest, '[')
    {
        tags = rest[at + 1..rest.len() - 1]
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        rest = rest[..at].trim_end();
    }
    let mut name = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                name.push(next);
            }
        } else {
            name.push(c);
        }
    }
    (name, tags)
}

/// The first `c` not escaped with a backslash.
fn unescaped_find(text: &str, c: char) -> Option<usize> {
    let mut escaped = false;
    for (i, ch) in text.char_indices() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == c {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links() {
        assert_eq!(parse_link("Go|North"), ("Go".into(), "North".into()));
        assert_eq!(parse_link("Go->North"), ("Go".into(), "North".into()));
        assert_eq!(parse_link("North<-Go"), ("Go".into(), "North".into()));
        assert_eq!(parse_link("North"), ("North".into(), "North".into()));
    }

    #[test]
    fn headers() {
        assert_eq!(
            header_parts(r#" A room [dark cold] {"position":"1,2"}"#),
            ("A room".into(), vec!["dark".into(), "cold".into()])
        );
        assert_eq!(
            header_parts(r" Odd \[name\]"),
            ("Odd [name]".into(), vec![])
        );
    }
}
