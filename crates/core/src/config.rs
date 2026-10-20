//! The project file, `dialogue-maker.toml`: what a project knows about itself that its
//! dialogue files cannot say. Every field is optional; an empty file (or none) works.
//!
//! Paths of sources, portraits and assets are relative to the project root. Paths of
//! documents are relative to the project file's own folder.

use std::collections::BTreeMap;
use std::fmt;

use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// The project file's name, looked for at the project root.
pub const FILE_NAME: &str = "dialogue-maker.toml";

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// The project's name, for titles.
    pub name: Option<String>,
    pub sources: Sources,
    /// Places in the order a player meets them.
    pub places: Vec<Place>,
    /// Characters in the order they should be listed.
    pub characters: Vec<Character>,
    /// Line tags (`[#name=value]`) by name.
    pub tags: BTreeMap<String, TagSpec>,
    /// Conditions the views can filter on.
    pub variants: Vec<Variant>,
    /// Comments that say who wrote what.
    pub marks: Vec<MarkSpec>,
    /// A regex for the comment that opens a section of labels (`# === PORTEIRO ===`).
    /// Its first capture group, or the group named `title`, is the section's title.
    pub section_header: Option<String>,
    pub flags: Flags,
    pub documents: Vec<Document>,
    pub checks: Checks,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Sources {
    /// Globs of the dialogue files.
    pub dialogue: Vec<String>,
    /// Globs of files to leave out.
    pub exclude: Vec<String>,
    /// Globs of the game's code and scenes: a label named there in quotes is an entry point.
    pub code: Vec<String>,
}

impl Default for Sources {
    fn default() -> Self {
        Self {
            dialogue: vec!["**/*.dialogue".into()],
            // Godot plugins, Dialogue Manager's own examples among them.
            exclude: vec!["addons/**".into()],
            code: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Place {
    pub id: String,
    pub name: String,
    /// A district, a chapter: places with the same group are shown together.
    pub group: Option<String>,
    /// Dialogue files played here.
    pub files: Vec<String>,
    /// Only these labels of the files (when a place takes part of a file).
    pub labels: Vec<String>,
    /// All labels of the files but these.
    pub exclude_labels: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Character {
    pub id: String,
    /// The name to show.
    pub name: String,
    /// Speaker names the dialogue files use for this character (`Vizinho`).
    pub speakers: Vec<String>,
    /// The face shown for the character's lines when no tag picks another.
    pub portrait: Option<String>,
    /// Who they are, in a sentence.
    pub role: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TagSpec {
    pub kind: TagKind,
    /// Allowed values of an `enum` tag.
    pub values: Vec<String>,
    /// Where an `asset` tag's file is, with `{value}` for the value
    /// (`assets/reactions/{value}.png`).
    pub path: Option<String>,
    /// How the views show it: `thought`, `shake`, `voice`, `face`, or nothing.
    pub show: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TagKind {
    /// Present or not: `[#pensamento]`.
    #[default]
    Flag,
    /// One of `values`: `[#voz=feliz]`.
    Enum,
    /// Names a file: `[#reaction=detetive_serio_1]`.
    Asset,
    /// Any value.
    Text,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Variant {
    pub name: String,
    /// The condition, as written in the dialogue (`Globals.modo_assistido()`).
    pub condition: String,
    /// What the views call the branch where the condition holds.
    #[serde(rename = "true")]
    pub if_true: String,
    /// ... and where it does not.
    #[serde(rename = "false")]
    pub if_false: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MarkSpec {
    /// Text in a comment that marks the next line or label (`✏ produção`). Anything after
    /// it, in parentheses or not, is the mark's note.
    pub marker: String,
    /// Who the mark means (`production`).
    pub who: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Flags {
    /// Functions whose first string argument is a flag they set (`set_game_flag`).
    pub set: Vec<String>,
    /// Functions whose first string argument is a flag they read (`has_game_flag`).
    pub read: Vec<String>,
    /// `match` subjects whose `when` values are flags read (`Globals.pista_atual()`).
    pub match_subjects: Vec<String>,
    /// What each flag means.
    pub meanings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Document {
    /// A Markdown file, relative to the project file. `[[label]]` links into the script.
    pub path: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Checks {
    /// Globs of labels the game starts by a name it builds at run time
    /// (`cavalo_generico_p*`), so no file names them whole.
    pub entry_points: Vec<String>,
    /// Check codes not to report.
    pub ignore: Vec<String>,
}

/// A project file that does not read.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        toml::from_str(text).map_err(|e| ConfigError(e.to_string()))
    }

    pub fn character_of_speaker(&self, speaker: &str) -> Option<&Character> {
        self.characters
            .iter()
            .find(|c| c.speakers.iter().any(|s| s == speaker))
    }

    /// The patterns in the file, compiled; fails on a bad glob or regex.
    pub fn compile(&self) -> Result<Compiled, ConfigError> {
        let section_header = self
            .section_header
            .as_deref()
            .map(|p| Regex::new(p).map_err(|e| ConfigError(format!("section_header: {e}"))))
            .transpose()?;
        Ok(Compiled {
            dialogue: globs("sources.dialogue", &self.sources.dialogue)?,
            exclude: globs("sources.exclude", &self.sources.exclude)?,
            code: globs("sources.code", &self.sources.code)?,
            entry_points: globs("checks.entry_points", &self.checks.entry_points)?,
            section_header,
        })
    }
}

/// The patterns of a project file, compiled.
#[derive(Debug, Clone)]
pub struct Compiled {
    pub dialogue: GlobSet,
    pub exclude: GlobSet,
    pub code: GlobSet,
    pub entry_points: GlobSet,
    pub section_header: Option<Regex>,
}

impl Compiled {
    /// The title of a section header comment, if this comment is one.
    pub fn section_title(&self, comment: &str) -> Option<String> {
        let captures = self.section_header.as_ref()?.captures(comment)?;
        let title = captures
            .name("title")
            .or_else(|| captures.get(1))
            .map_or("", |m| m.as_str());
        Some(title.trim().to_owned())
    }
}

fn globs(field: &str, patterns: &[String]) -> Result<GlobSet, ConfigError> {
    let mut set = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|e| ConfigError(format!("{field}: {e}")))?;
        set.add(glob);
    }
    set.build()
        .map_err(|e| ConfigError(format!("{field}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_has_defaults() {
        let config = Config::parse("").unwrap();
        assert_eq!(config.sources.dialogue, ["**/*.dialogue"]);
        let compiled = config.compile().unwrap();
        assert!(compiled.dialogue.is_match("src/a/b.dialogue"));
        assert!(
            compiled
                .exclude
                .is_match("addons/dialogue_manager/x.dialogue")
        );
    }

    #[test]
    fn full_file() {
        let config = Config::parse(
            r#"
            name = "Lighthouse"
            section_header = '^# =+ (?P<title>.+?) =+$'

            [sources]
            dialogue = ["dialogue/*.dialogue"]

            [[places]]
            id = "pier"
            name = "The Pier"
            files = ["dialogue/pier.dialogue"]

            [[characters]]
            id = "keeper"
            name = "The Keeper"
            speakers = ["Keeper", "Old Man"]

            [tags.mood]
            kind = "enum"
            values = ["calm", "angry"]

            [tags.reaction]
            kind = "asset"
            path = "faces/{value}.png"

            [[variants]]
            name = "Mode"
            condition = "Globals.easy()"
            true = "Easy"
            false = "Hard"

            [[marks]]
            marker = "✏ prod"
            who = "production"

            [flags]
            set = ["set_flag"]
            read = ["has_flag"]
            meanings = { oil = "The oil was found." }
            "#,
        )
        .unwrap();
        assert_eq!(config.character_of_speaker("Old Man").unwrap().id, "keeper");
        assert_eq!(config.tags["mood"].kind, TagKind::Enum);
        assert_eq!(config.variants[0].if_false, "Hard");
        let compiled = config.compile().unwrap();
        assert_eq!(
            compiled.section_title("# === THE KEEPER ===").as_deref(),
            Some("THE KEEPER")
        );
        assert_eq!(compiled.section_title("# a note"), None);
    }

    #[test]
    fn typos_are_errors() {
        let error = Config::parse("[[characters]]\nid = \"a\"\nspeaker = [\"A\"]\n").unwrap_err();
        assert!(error.0.contains("speaker"), "{error}");
        let error = Config::parse("section_header = \"(\"")
            .unwrap()
            .compile()
            .unwrap_err();
        assert!(error.0.starts_with("section_header"), "{error}");
    }
}
