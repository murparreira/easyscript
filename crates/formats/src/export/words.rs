//! The words an export writes around the script's own text, in English or Portuguese.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The language of an export's own words (headings, stage directions, column names).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, rename = "ExportLanguage")]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    En,
    #[serde(rename = "pt-BR")]
    PtBr,
}

impl Language {
    /// `en`, `pt-BR` (or `pt`, `pt_BR`).
    pub fn parse(tag: &str) -> Option<Self> {
        let tag = tag.to_ascii_lowercase().replace('_', "-");
        if tag == "en" || tag.starts_with("en-") {
            Some(Language::En)
        } else if tag == "pt" || tag.starts_with("pt-") {
            Some(Language::PtBr)
        } else {
            None
        }
    }

    pub(crate) fn words(self) -> &'static Words {
        match self {
            Language::En => &EN,
            Language::PtBr => &PT_BR,
        }
    }
}

pub(crate) struct Words {
    // Voice scripts.
    pub voice_script: &'static str,
    pub line: &'static str,
    pub lines: &'static str,
    pub mood: &'static str,
    pub moods: &'static str,
    pub no_mood: &'static str,
    pub thought: &'static str,
    pub after: &'static str,
    pub narration: &'static str,
    pub choice: &'static str,
    pub columns: [&'static str; 9],
    pub yes: &'static str,
    // Screenplays.
    pub credit: &'static str,
    pub chooses: &'static str,
    pub only_if: &'static str,
    pub end_of_choices: &'static str,
    pub when_is: &'static str,
    pub if_: &'static str,
    pub otherwise: &'static str,
    pub end_of_block: &'static str,
    pub while_: &'static str,
    pub at_random: &'static str,
    pub weight: &'static str,
    pub end: &'static str,
    pub end_of_conversation: &'static str,
    pub thinking: &'static str,
    pub shaking: &'static str,
}

static EN: Words = Words {
    voice_script: "{name}: voice script",
    line: "line",
    lines: "lines",
    mood: "mood",
    moods: "moods",
    no_mood: "No mood given",
    thought: "(thought)",
    after: "after {who}",
    narration: "narration",
    choice: "(choice)",
    columns: [
        "character",
        "mood",
        "thought",
        "text",
        "context",
        "id",
        "label",
        "file",
        "line",
    ],
    yes: "yes",
    credit: "exported by dialogue-maker",
    chooses: "The player chooses:",
    only_if: "only if {condition}",
    end_of_choices: "(end of choices)",
    when_is: "When {subject} is {value}:",
    if_: "If {condition}:",
    otherwise: "Otherwise:",
    end_of_block: "(end)",
    while_: "While {condition}:",
    at_random: "One of these, at random:",
    weight: "weight {weight}",
    end: "End.",
    end_of_conversation: "End of the conversation.",
    thinking: "thinking",
    shaking: "shaking",
};

static PT_BR: Words = Words {
    voice_script: "{name}: roteiro de voz",
    line: "fala",
    lines: "falas",
    mood: "tom",
    moods: "tons",
    no_mood: "Sem tom indicado",
    thought: "(pensamento)",
    after: "depois de {who}",
    narration: "narração",
    choice: "(escolha)",
    columns: [
        "personagem",
        "tom",
        "pensamento",
        "texto",
        "contexto",
        "id",
        "label",
        "arquivo",
        "linha",
    ],
    yes: "sim",
    credit: "exportado pelo dialogue-maker",
    chooses: "O jogador escolhe:",
    only_if: "só se {condition}",
    end_of_choices: "(fim das escolhas)",
    when_is: "Quando {subject} for {value}:",
    if_: "Se {condition}:",
    otherwise: "Senão:",
    end_of_block: "(fim)",
    while_: "Enquanto {condition}:",
    at_random: "Uma destas, ao acaso:",
    weight: "peso {weight}",
    end: "Fim.",
    end_of_conversation: "Fim da conversa.",
    thinking: "pensando",
    shaking: "tremendo",
};

/// A phrase with `{name}` filled in.
pub(crate) fn fill(phrase: &str, values: &[(&str, &str)]) -> String {
    let mut out = phrase.to_owned();
    for (name, value) in values {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}
