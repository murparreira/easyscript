//! Other tools' scripts as Dialogue Manager files: Twine's Twee, Yarn Spinner, Ink. Each
//! converts what has a match in Dialogue Manager and keeps the rest as comments, with a
//! warning naming the line, so nothing is dropped without a word. A Dialogue Manager file
//! comes in as it is.
//!
//! The result is a new file's text; writing it is the caller's (never over a file that
//! exists).

pub mod ink;
pub mod twee;
pub mod yarn;

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The formats there are importers for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename = "ImportSource")]
pub enum Source {
    /// Twine 2's Twee 3 (`.twee`, `.tw`).
    Twee,
    /// Yarn Spinner 2 (`.yarn`).
    Yarn,
    /// Ink (`.ink`).
    Ink,
    /// A Dialogue Manager file from elsewhere (`.dialogue`), as it is.
    Dialogue,
}

impl Source {
    /// The format of a file, by its extension.
    pub fn of(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        match extension.as_str() {
            "twee" | "tw" | "tw3" => Some(Source::Twee),
            "yarn" => Some(Source::Yarn),
            "ink" => Some(Source::Ink),
            "dialogue" => Some(Source::Dialogue),
            _ => None,
        }
    }
}

/// A converted script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Imported {
    /// The Dialogue Manager file.
    pub text: String,
    /// What did not convert, or converted only in part.
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, rename = "ImportWarning")]
pub struct Warning {
    /// The line in the source file, from 1.
    pub line: usize,
    pub message: String,
}

/// Convert a script.
pub fn import(source: Source, text: &str) -> Imported {
    match source {
        Source::Twee => twee::convert(text),
        Source::Yarn => yarn::convert(text),
        Source::Ink => ink::convert(text),
        Source::Dialogue => Imported {
            text: text.to_owned(),
            warnings: es_syntax::parse(text)
                .errors()
                .map(|token| Warning {
                    line: text[..usize::from(token.text_range().start())]
                        .matches('\n')
                        .count()
                        + 1,
                    message: format!(
                        "`{}` does not read as Dialogue Manager: kept as it is",
                        token.text().trim()
                    ),
                })
                .collect(),
        },
    }
}

/// The Dialogue Manager file being written.
#[derive(Default)]
pub(crate) struct Out {
    text: String,
    warnings: Vec<Warning>,
}

impl Out {
    pub fn title(&mut self, id: &str) {
        if !self.text.is_empty() && !self.text.ends_with("\n\n") {
            self.text.push('\n');
        }
        let _ = writeln!(self.text, "~ {id}");
    }

    /// A line at a depth of nesting (a tab each).
    pub fn line(&mut self, depth: usize, text: &str) {
        for _ in 0..depth {
            self.text.push('\t');
        }
        self.text.push_str(text);
        self.text.push('\n');
    }

    pub fn comment(&mut self, depth: usize, text: &str) {
        for line in text.lines() {
            self.line(depth, &format!("# {}", line.trim_end()));
        }
    }

    pub fn blank(&mut self) {
        if !self.text.is_empty() && !self.text.ends_with("\n\n") {
            self.text.push('\n');
        }
    }

    pub fn warn(&mut self, line: usize, message: impl Into<String>) {
        self.warnings.push(Warning {
            line,
            message: message.into(),
        });
    }

    /// Something kept as a comment, with a warning.
    pub fn unconverted(&mut self, depth: usize, line: usize, source: &str, why: &str) {
        self.comment(depth, source.trim());
        self.warn(line, format!("{why}: kept as a comment"));
    }

    pub fn finish(mut self) -> Imported {
        while self.text.ends_with("\n\n") {
            self.text.pop();
        }
        Imported {
            text: self.text,
            warnings: self.warnings,
        }
    }
}

/// Titles for the source's names: what Dialogue Manager accepts (letters, digits and `_`,
/// not starting with a digit), each name its own title.
#[derive(Default)]
pub(crate) struct Titles {
    by_name: HashMap<String, String>,
    used: HashSet<String>,
}

impl Titles {
    /// The title for a name, made the first time it is asked for.
    pub fn of(&mut self, name: &str) -> String {
        if let Some(id) = self.by_name.get(name) {
            return id.clone();
        }
        let base = title_id(name);
        let mut id = base.clone();
        let mut n = 2;
        while !self.used.insert(id.clone()) || reserved(&id) {
            id = format!("{base}_{n}");
            n += 1;
        }
        self.by_name.insert(name.to_owned(), id.clone());
        id
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.by_name.get(name).map(String::as_str)
    }
}

fn reserved(id: &str) -> bool {
    matches!(id, "END" | "END!")
}

/// A name as a title: accents folded, anything else not allowed as `_`.
pub(crate) fn title_id(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        let c = fold(c);
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_').to_owned();
    match out.chars().next() {
        None => "untitled".into(),
        Some(c) if c.is_ascii_digit() => format!("_{out}"),
        Some(_) => out,
    }
}

/// Latin letters with accents as the letter alone.
fn fold(c: char) -> char {
    const FROM: &str = "ÀÁÂÃÄÅàáâãäåÇçÈÉÊËèéêëÌÍÎÏìíîïÑñÒÓÔÕÖØòóôõöøÙÚÛÜùúûüÝýÿ";
    const TO: &str = "AAAAAAaaaaaaCcEEEEeeeeIIIIiiiiNnOOOOOOooooooUUUUuuuuYyy";
    FROM.chars()
        .position(|f| f == c)
        .and_then(|i| TO.chars().nth(i))
        .unwrap_or(c)
}

/// Words that open a line Dialogue Manager reads as something other than dialogue.
const KEYWORDS: [&str; 12] = [
    "if ", "elif ", "else ", "match ", "when ", "while ", "do ", "do! ", "set ", "import ",
    "using ", "return ",
];

/// Narration (a line with no speaker) as Dialogue Manager reads it back as narration: a
/// colon followed by a space would make what is before it a speaker, and a line opening
/// with a keyword or a marker would be read as code.
pub(crate) fn narration(text: &str) -> String {
    let text = text.trim().replace(": ", "\\: ");
    let code = KEYWORDS
        .iter()
        .any(|k| text.starts_with(k) || text == k.trim())
        || ["- ", "~ ", "=> ", "=>< ", "| ", "% ", "#", "$>"]
            .iter()
            .any(|m| text.starts_with(m));
    if code {
        // An empty expression first: shown as nothing, read as dialogue.
        format!("{{{{\"\"}}}}{text}")
    } else {
        text
    }
}

/// A spoken line, `Speaker: text`, when the line looks like one.
pub(crate) fn speaker_and_text(line: &str) -> Option<(&str, &str)> {
    let (speaker, text) = line.split_once(": ")?;
    let speaker = speaker.trim();
    // A name: a few words, each capitalised (`Old Man`, not `The gate creaks`).
    let ok = !speaker.is_empty()
        && speaker.chars().count() <= 40
        && !speaker.contains([
            '[', ']', '{', '}', '<', '>', '"', '*', '#', ',', '.', '!', '?',
        ])
        && speaker.split_whitespace().count() <= 3
        && speaker
            .split_whitespace()
            .all(|w| w.chars().next().is_some_and(|c| !c.is_lowercase()));
    ok.then_some((speaker, text.trim()))
}

/// Tags as Dialogue Manager writes them: `[#a, b=c]`.
pub(crate) fn tags(tags: &[String]) -> String {
    if tags.is_empty() {
        String::new()
    } else {
        format!(" [#{}]", tags.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles() {
        let mut t = Titles::default();
        assert_eq!(t.of("Começo da história"), "Comeco_da_historia");
        assert_eq!(t.of("2nd floor"), "_2nd_floor");
        assert_eq!(t.of("a-b"), "a_b");
        assert_eq!(t.of("a b"), "a_b_2", "two names, two titles");
        assert_eq!(t.of("a-b"), "a_b", "the same name, the same title");
        assert_eq!(t.of("END"), "END_2");
        assert_eq!(t.of("!!!"), "untitled");
    }

    #[test]
    fn narration_stays_narration() {
        assert_eq!(narration("Note: it rains."), "Note\\: it rains.");
        assert_eq!(narration("do you see?"), "{{\"\"}}do you see?");
        assert_eq!(narration("- a list"), "{{\"\"}}- a list");
        assert_eq!(narration("Plain."), "Plain.");
    }
}
