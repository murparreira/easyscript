//! A translation table, in the CSV layout Dialogue Manager exports and Godot imports: a
//! `keys` column, then one column per locale, then `_character` and `_notes` when asked.
//! A line's key is its static id (`[ID:...]`), or else its text, as Dialogue Manager does.
//!
//! Given the table as it is now, the translations in it are kept: rows for lines still in
//! the script get the script's current text in the source column and keep the rest; rows
//! for lines no longer in the script stay, after the others, so no translator's work is
//! lost.

use std::collections::{HashMap, HashSet};

use es_core::model::{Choice, Line, Node, NodeKind, Span};
use es_core::{Project, Script};

use super::{Source, shown_text};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// The locale the script is written in: the column with its text (`en`).
    pub locale: String,
    /// Columns for the translations (`pt_BR`, `es`).
    pub extra_locales: Vec<String>,
    /// A `_character` column: who says the line, `(response)` for choices.
    pub character: bool,
    /// A `_notes` column: the `##` comments above the line, for translators.
    pub notes: bool,
    /// `,`, `;` or a tab.
    pub delimiter: u8,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            locale: "en".into(),
            extra_locales: Vec::new(),
            character: true,
            notes: true,
            delimiter: b',',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationError(pub String);

impl std::fmt::Display for TranslationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TranslationError {}

/// One translatable string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub text: String,
    pub character: String,
    pub notes: String,
}

/// Every line and choice of the script, once per key, in file order.
pub fn entries(project: &Project, script: &Script) -> Vec<Entry> {
    let source = Source::new(project);
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for file in &script.files {
        let mut found = Vec::new();
        let all = file.preamble.iter().chain(
            file.sections
                .iter()
                .flat_map(|s| &s.labels)
                .flat_map(|l| &l.body),
        );
        for node in all {
            collect(node, &mut found);
        }
        let Some(first) = found.first() else {
            continue;
        };
        let notes = doc_comments(
            &project.files[first.0.file].text,
            found.iter().map(|(span, _)| source.line(*span)).collect(),
        );
        for (span, mut entry) in found {
            entry.notes = notes.get(&source.line(span)).cloned().unwrap_or_default();
            if !entry.key.is_empty() && seen.insert(entry.key.clone()) {
                out.push(entry);
            }
        }
    }
    out
}

/// Lines and choices in file order: a choice before what it leads to.
fn collect(node: &Node, out: &mut Vec<(Span, Entry)>) {
    let each = |nodes: &[Node], out: &mut Vec<(Span, Entry)>| {
        nodes.iter().for_each(|n| collect(n, out));
    };
    match &node.kind {
        NodeKind::Line(line) => out.push((node.span, line_entry(line))),
        NodeKind::Menu { choices } => {
            for choice in choices {
                out.push((choice.span, choice_entry(choice)));
                each(&choice.body, out);
            }
        }
        NodeKind::Branch { arms } | NodeKind::Match { arms, .. } => {
            arms.iter().for_each(|a| each(&a.body, out));
        }
        NodeKind::While { body, .. } => each(body, out),
        NodeKind::Random { options } => options.iter().for_each(|o| each(&o.body, out)),
        NodeKind::Effect(_) | NodeKind::Jump(_) | NodeKind::Anchor { .. } => {}
    }
}

/// Dialogue Manager's notes for translators: the `##` comments since the last line or
/// choice, given to the next one (by line number).
fn doc_comments(text: &str, lines: HashSet<usize>) -> HashMap<usize, String> {
    let mut out = HashMap::new();
    let mut pending: Vec<&str> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(doc) = trimmed.strip_prefix("##") {
            pending.push(doc.trim());
        } else if lines.contains(&(i + 1)) {
            if !pending.is_empty() {
                out.insert(i + 1, pending.join("\n"));
            }
            pending.clear();
        }
    }
    out
}

fn line_entry(line: &Line) -> Entry {
    let text = shown_text(&line.words);
    Entry {
        key: line.id.clone().unwrap_or_else(|| text.clone()),
        text,
        character: line.speaker.clone().unwrap_or_default(),
        notes: String::new(),
    }
}

fn choice_entry(choice: &Choice) -> Entry {
    let text = shown_text(&choice.words);
    Entry {
        key: choice.id.clone().unwrap_or_else(|| text.clone()),
        text,
        character: "(response)".into(),
        notes: String::new(),
    }
}

/// The delimiter of a table: whichever of `,`, `;` and tab its header has most of.
pub fn sniff_delimiter(table: &str) -> u8 {
    let header = table.lines().next().unwrap_or_default();
    b",;\t"
        .iter()
        .copied()
        .max_by_key(|d| header.bytes().filter(|b| b == d).count())
        .filter(|d| header.as_bytes().contains(d))
        .unwrap_or(b',')
}

/// The table as CSV text, keeping the translations of `existing` (the table as it is now).
pub fn to_csv(
    project: &Project,
    script: &Script,
    options: &Options,
    existing: Option<&str>,
) -> Result<String, TranslationError> {
    let entries = entries(project, script);

    // The columns: the existing table's, then any asked for that it lacks.
    let mut header: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let Some(existing) = existing.filter(|e| !e.trim().is_empty()) {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(options.delimiter)
            .has_headers(false)
            .flexible(true)
            .from_reader(existing.as_bytes());
        for record in reader.records() {
            let record = record.map_err(|e| TranslationError(format!("the table: {e}")))?;
            let row: Vec<String> = record.iter().map(str::to_owned).collect();
            if header.is_empty() {
                header = row;
            } else if row.first().is_some_and(|k| !k.trim().is_empty()) {
                rows.push(row);
            }
        }
        if header.first().map(String::as_str) != Some("keys") {
            return Err(TranslationError(
                "the table's first column is not `keys`: not a translation table".into(),
            ));
        }
    } else {
        header.push("keys".into());
    }
    let mut wanted = vec![options.locale.clone()];
    wanted.extend(options.extra_locales.iter().cloned());
    if options.character {
        wanted.push("_character".into());
    }
    if options.notes {
        wanted.push("_notes".into());
    }
    for column in wanted {
        if !header.contains(&column) {
            header.push(column);
        }
    }
    let column = |name: &str| header.iter().position(|h| h == name);
    let text_at = column(&options.locale).expect("added above");
    let character_at = column("_character");
    let notes_at = column("_notes");

    let mut old: HashMap<String, Vec<String>> = HashMap::new();
    let mut old_order = Vec::new();
    for row in rows {
        old_order.push(row[0].clone());
        old.insert(row[0].clone(), row);
    }

    let mut writer = csv::WriterBuilder::new()
        .delimiter(options.delimiter)
        .flexible(false)
        .from_writer(Vec::new());
    let write = |writer: &mut csv::Writer<Vec<u8>>, row: &[String]| {
        writer
            .write_record(row)
            .map_err(|e| TranslationError(e.to_string()))
    };
    write(&mut writer, &header)?;
    for entry in &entries {
        let mut row = old
            .remove(&entry.key)
            .unwrap_or_else(|| vec![entry.key.clone()]);
        row.resize(header.len(), String::new());
        row[text_at] = entry.text.clone();
        if let Some(at) = character_at {
            row[at] = entry.character.clone();
        }
        if let Some(at) = notes_at {
            row[at] = entry.notes.clone();
        }
        write(&mut writer, &row)?;
    }
    // Lines no longer in the script: kept, after the others.
    for key in old_order {
        if let Some(mut row) = old.remove(&key) {
            row.resize(header.len(), String::new());
            write(&mut writer, &row)?;
        }
    }
    let bytes = writer
        .into_inner()
        .map_err(|e| TranslationError(e.to_string()))?;
    Ok(String::from_utf8(bytes).expect("written from strings"))
}
