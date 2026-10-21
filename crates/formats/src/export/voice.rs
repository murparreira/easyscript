//! Voice-actor scripts: each character's lines, grouped by the mood they are to be read
//! in (the tag the project file shows as `voice`), each with the line before it for
//! context. As Markdown to read or print, or as CSV for a recording sheet.

use std::fmt::Write as _;

use dm_core::model::{Label, Node, NodeKind, Segment};
use dm_core::{Project, Script};

use super::words::{Language, fill};
use super::{Source, plain};

/// A line to record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Take {
    /// The character's id, or the speaker as written when no character claims it.
    pub character: String,
    /// The character's name, or the speaker.
    pub name: String,
    pub speaker: String,
    /// The mood to read it in; none when the line names none.
    pub mood: Option<String>,
    /// Said in thought (the tag shown as `thought`).
    pub thought: bool,
    pub text: String,
    /// What comes right before: who says it, and the words.
    pub context: Option<(Said, String)>,
    /// The static line id, which names the audio file in Dialogue Manager.
    pub id: Option<String>,
    pub label: String,
    pub path: String,
    pub line: usize,
}

/// Who says the line before a take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Said {
    Speaker(String),
    Narration,
    /// The choice the player picked.
    Choice,
}

/// Every spoken line, in script order.
pub fn takes(project: &Project, script: &Script) -> Vec<Take> {
    let source = Source::new(project);
    let mood_tag = source.tag_shown_as("voice");
    let thought_tag = source.tag_shown_as("thought");
    let mut out = Vec::new();
    for label in script.labels() {
        let mut before: Option<(Said, String)> = None;
        collect(
            &source,
            label,
            &label.body,
            &mut before,
            (mood_tag, thought_tag),
            &mut out,
        );
    }
    out
}

fn collect(
    source: &Source,
    label: &Label,
    nodes: &[Node],
    before: &mut Option<(Said, String)>,
    tags: (Option<&str>, Option<&str>),
    out: &mut Vec<Take>,
) {
    for node in nodes {
        match &node.kind {
            NodeKind::Line(line) => {
                let text = plain(line);
                let who = line.speaker.clone().map_or(Said::Narration, Said::Speaker);
                if let Some(speaker) = &line.speaker {
                    let character = source.character(speaker);
                    out.push(Take {
                        character: character.map_or_else(|| speaker.clone(), |c| c.id.clone()),
                        name: character.map_or_else(|| speaker.clone(), |c| c.name.clone()),
                        speaker: speaker.clone(),
                        mood: source.tag(&line.tags, tags.0).flatten().map(str::to_owned),
                        thought: source
                            .tag(&line.tags, tags.1)
                            .is_some_and(|v| v != Some("false")),
                        text: text.clone(),
                        context: before.clone(),
                        id: line.id.clone(),
                        label: label.id.clone(),
                        path: source.path(node.span).to_owned(),
                        line: source.line(node.span),
                    });
                }
                *before = Some((who, text));
            }
            NodeKind::Menu { choices } => {
                let after = before.clone();
                for choice in choices {
                    let mut inside =
                        Some((Said::Choice, Segment::plain(&choice.text).trim().to_owned()));
                    collect(source, label, &choice.body, &mut inside, tags, out);
                }
                *before = after;
            }
            NodeKind::Branch { arms } | NodeKind::Match { arms, .. } => {
                let after = before.clone();
                for arm in arms {
                    let mut inside = after.clone();
                    collect(source, label, &arm.body, &mut inside, tags, out);
                }
                *before = after;
            }
            NodeKind::While { body, .. } => collect(source, label, body, before, tags, out),
            NodeKind::Random { options } => {
                let after = before.clone();
                for option in options {
                    let mut inside = after.clone();
                    collect(source, label, &option.body, &mut inside, tags, out);
                }
                *before = after;
            }
            NodeKind::Effect(_) | NodeKind::Jump(_) | NodeKind::Anchor { .. } => {}
        }
    }
}

/// A character's lines, by mood.
type ByMood<'t> = Vec<(Option<&'t str>, Vec<&'t Take>)>;

/// Characters in the project file's order, then other speakers by name; each with their
/// moods in the order the project file lists them, then others by name, then no mood.
fn grouped<'t>(project: &Project, takes: &'t [Take]) -> Vec<(&'t str, ByMood<'t>)> {
    let config = &project.config;
    let rank = |character: &str| {
        config
            .characters
            .iter()
            .position(|c| c.id == character)
            .unwrap_or(usize::MAX)
    };
    let mut characters: Vec<&str> = Vec::new();
    for take in takes {
        if !characters.contains(&take.character.as_str()) {
            characters.push(&take.character);
        }
    }
    characters.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a.cmp(b)));
    characters
        .into_iter()
        .map(|character| {
            let voices = config
                .characters
                .iter()
                .find(|c| c.id == character)
                .map(|c| c.voices.as_slice())
                .unwrap_or_default();
            let mine: Vec<&Take> = takes.iter().filter(|t| t.character == character).collect();
            let mut moods: Vec<Option<&str>> = Vec::new();
            for take in &mine {
                if !moods.contains(&take.mood.as_deref()) {
                    moods.push(take.mood.as_deref());
                }
            }
            let rank = |mood: &Option<&str>| match mood {
                Some(m) => (0, voices.iter().position(|v| v == m).unwrap_or(usize::MAX)),
                None => (1, 0),
            };
            moods.sort_by(|a, b| rank(a).cmp(&rank(b)).then(a.cmp(b)));
            let groups = moods
                .into_iter()
                .map(|mood| {
                    let lines = mine
                        .iter()
                        .copied()
                        .filter(|t| t.mood.as_deref() == mood)
                        .collect();
                    (mood, lines)
                })
                .collect();
            (character, groups)
        })
        .collect()
}

/// The script for `character` (an id or a speaker), or for everyone, as Markdown.
pub fn to_markdown(
    project: &Project,
    script: &Script,
    character: Option<&str>,
    language: Language,
) -> String {
    let words = language.words();
    let all = takes(project, script);
    let takes: Vec<Take> = all
        .into_iter()
        .filter(|t| character.is_none_or(|c| t.character == c || t.speaker == c))
        .collect();
    let title = project.config.name.as_deref();
    let mut out = String::new();
    for (i, (_, moods)) in grouped(project, &takes).into_iter().enumerate() {
        let name = &moods[0].1[0].name;
        let count: usize = moods.iter().map(|(_, l)| l.len()).sum();
        if i > 0 {
            out.push('\n');
        }
        let _ = writeln!(out, "# {}\n", fill(words.voice_script, &[("name", name)]));
        let _ = writeln!(
            out,
            "{}{count} {}, {} {}.\n",
            title.map(|t| format!("{t}. ")).unwrap_or_default(),
            if count == 1 { words.line } else { words.lines },
            moods.len(),
            if moods.len() == 1 {
                words.mood
            } else {
                words.moods
            },
        );
        for (mood, lines) in moods {
            let _ = writeln!(
                out,
                "## {} ({})\n",
                mood.unwrap_or(words.no_mood),
                lines.len()
            );
            for (n, take) in lines.iter().enumerate() {
                let thought = if take.thought {
                    format!("{} ", words.thought)
                } else {
                    String::new()
                };
                let _ = writeln!(out, "{}. {thought}**{}**", n + 1, escape(&take.text));
                if let Some((who, said)) = &take.context {
                    let who = match who {
                        Said::Speaker(s) => s.as_str(),
                        Said::Narration => words.narration,
                        Said::Choice => words.choice,
                    };
                    let after = fill(words.after, &[("who", who)]);
                    let _ = writeln!(out, "   - {after}: *{}*", escape(said));
                }
                let id = take
                    .id
                    .as_deref()
                    .map(|id| format!(" · ID `{id}`"))
                    .unwrap_or_default();
                let _ = writeln!(
                    out,
                    "   - `{}` · {}:{}{id}",
                    take.label, take.path, take.line
                );
            }
            out.push('\n');
        }
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

/// Every spoken line as CSV: one row per line, to sort and fill in at a recording session.
pub fn to_csv(project: &Project, script: &Script, language: Language) -> String {
    let words = language.words();
    let takes = takes(project, script);
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(words.columns).expect("in memory");
    for (_, moods) in grouped(project, &takes) {
        for (_, lines) in moods {
            for take in lines {
                let context = take
                    .context
                    .as_ref()
                    .map(|(who, said)| match who {
                        Said::Speaker(s) => format!("{s}: {said}"),
                        Said::Narration => said.clone(),
                        Said::Choice => format!("{}: {said}", words.choice),
                    })
                    .unwrap_or_default();
                writer
                    .write_record([
                        take.name.as_str(),
                        take.mood.as_deref().unwrap_or(""),
                        if take.thought { words.yes } else { "" },
                        &take.text,
                        &context,
                        take.id.as_deref().unwrap_or(""),
                        &take.label,
                        &take.path,
                        &take.line.to_string(),
                    ])
                    .expect("in memory");
            }
        }
    }
    String::from_utf8(writer.into_inner().expect("in memory")).expect("written from strings")
}

/// Markdown's emphasis characters, kept literal.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '*' | '_' | '`' | '\\') {
            out.push('\\');
        }
        if c == '\n' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}
