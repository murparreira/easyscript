//! A project's script in other formats. Each takes the project (for its files and project
//! file) and its script (see [`es_core::lower`]), and returns the text to write.

pub mod archive;
pub mod fountain;
pub mod json;
pub mod translation;
pub mod voice;
mod words;

pub use words::Language;

use es_core::config::Character;
use es_core::model::{Line, Segment, Span, Tag};
use es_core::{Project, Script};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The exports there are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, rename = "ExportFormat")]
pub enum Format {
    /// The whole script as JSON, in a documented format.
    Json,
    /// A translation table: Dialogue Manager's CSV, which Godot imports.
    Translation,
    /// Each character's lines by mood, to read or print (Markdown).
    Voice,
    /// Every spoken line as a recording sheet (CSV).
    VoiceCsv,
    /// A screenplay (Fountain).
    Fountain,
}

impl Format {
    pub const ALL: [Format; 5] = [
        Format::Json,
        Format::Translation,
        Format::Voice,
        Format::VoiceCsv,
        Format::Fountain,
    ];

    /// The file name extension, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Translation | Format::VoiceCsv => "csv",
            Format::Voice => "md",
            Format::Fountain => "fountain",
        }
    }
}

/// What an export can be asked for; each format reads the fields that concern it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export, rename = "ExportOptions")]
pub struct Options {
    /// Translation: the locale the script is written in (`en` when none).
    pub locale: Option<String>,
    /// Translation: columns for the translations.
    pub extra_locales: Vec<String>,
    /// Voice: one character (an id or a speaker) instead of everyone.
    pub character: Option<String>,
    /// The language of the export's own words (voice scripts, screenplays).
    pub language: Language,
}

/// The export as text. `existing` is what is at the destination now: a translation
/// table keeps the translations in it.
pub fn render(
    format: Format,
    project: &Project,
    script: &Script,
    options: &Options,
    existing: Option<&str>,
) -> Result<String, translation::TranslationError> {
    Ok(match format {
        Format::Json => json::to_string(project, script),
        Format::Translation => {
            let mut table = translation::Options {
                extra_locales: options.extra_locales.clone(),
                ..Default::default()
            };
            if let Some(locale) = options.locale.clone().filter(|l| !l.trim().is_empty()) {
                table.locale = locale;
            }
            if let Some(existing) = existing {
                table.delimiter = translation::sniff_delimiter(existing);
            }
            translation::to_csv(project, script, &table, existing)?
        }
        Format::Voice => voice::to_markdown(
            project,
            script,
            options.character.as_deref(),
            options.language,
        ),
        Format::VoiceCsv => voice::to_csv(project, script, options.language),
        Format::Fountain => fountain::to_fountain(project, script, options.language),
    })
}

/// What every export reads.
pub(crate) struct Source<'a> {
    pub project: &'a Project,
}

impl<'a> Source<'a> {
    pub fn new(project: &'a Project) -> Self {
        Self { project }
    }

    pub fn path(&self, span: Span) -> &'a str {
        &self.project.files[span.file].path
    }

    /// The 1-based line a span starts on.
    pub fn line(&self, span: Span) -> usize {
        self.project.files[span.file].line_col(span.start).0
    }

    pub fn character(&self, speaker: &str) -> Option<&'a Character> {
        self.project.config.character_of_speaker(speaker)
    }

    /// The name of the tag the project shows as `role` (`voice`, `thought`, `shake`).
    pub fn tag_shown_as(&self, role: &str) -> Option<&'a str> {
        self.project
            .config
            .tags
            .iter()
            .find(|(_, spec)| spec.show.as_deref() == Some(role))
            .map(|(name, _)| name.as_str())
    }

    /// The value of a tag on a line: `Some(None)` for a tag with no value.
    pub fn tag<'t>(&self, tags: &'t [Tag], name: Option<&str>) -> Option<Option<&'t str>> {
        let name = name?;
        tags.iter()
            .find(|t| t.name == name)
            .map(|t| t.value.as_deref())
    }
}

/// The text Dialogue Manager shows (and translates) for a line or choice: the words as
/// written, `\n` as a new line, an escaped `\:` as a colon.
pub(crate) fn shown_text(words: &str) -> String {
    words
        .replace("\\n", "\n")
        .replace("\\:", ":")
        .trim()
        .to_owned()
}

/// A line's words as plain text: markup left out, random options as `a/b`.
pub(crate) fn plain(line: &Line) -> String {
    Segment::plain(&line.text)
        .replace("\\n", "\n")
        .trim()
        .to_owned()
}
