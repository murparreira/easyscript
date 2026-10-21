//! The script as JSON, for other tools: a documented, versioned format (see
//! `docs/formats/script-json.md` and the JSON Schema next to it), not the app's own view
//! data, which changes with the app.

use std::collections::BTreeMap;

use dm_core::model::{self, ArmKeyword, JumpTarget, Node as ModelNode, NodeKind};
use dm_core::{Project, Script};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Source, plain};

/// The value of `format`.
pub const FORMAT: &str = "dialogue-maker/script";
/// The value of `version`. New fields can appear in any version; it goes up when a field
/// changes meaning or goes away.
pub const VERSION: u32 = 1;

/// A project's whole script.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(title = "dialogue-maker script")]
pub struct ScriptExport {
    /// Always `dialogue-maker/script`.
    pub format: String,
    /// The format's version: 1.
    pub version: u32,
    /// The project's name, from its project file.
    pub name: Option<String>,
    /// The characters of the project file, in its order.
    pub characters: Vec<Character>,
    /// The places of the project file, in the order a player meets them.
    pub places: Vec<Place>,
    /// Every flag set or read, by name.
    pub flags: Vec<Flag>,
    /// The dialogue files, in path order.
    pub files: Vec<File>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Character {
    pub id: String,
    /// The name to show.
    pub name: String,
    /// The speaker names the dialogue uses for them.
    pub speakers: Vec<String>,
    pub role: Option<String>,
    /// The portrait, relative to the project root.
    pub portrait: Option<String>,
    /// The voice moods recorded for them.
    pub voices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Place {
    pub id: String,
    pub name: String,
    /// A district, a chapter.
    pub group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Flag {
    pub name: String,
    /// What it means, from the project file.
    pub meaning: Option<String>,
    /// Labels that set it.
    pub set_in: Vec<String>,
    /// Labels that read it.
    pub read_in: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct File {
    /// Relative to the project root, with `/`.
    pub path: String,
    /// What plays when the file starts from the top, before its first label.
    pub preamble: Vec<Node>,
    pub labels: Vec<Label>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Label {
    /// The title (`~ id`): what jumps name.
    pub id: String,
    /// The line of the title in the file, from 1.
    pub line: usize,
    /// The place it plays in (an id from `places`).
    pub place: Option<String>,
    /// The section it is under (a header comment the project file names).
    pub section: Option<String>,
    /// Comments before the title.
    pub notes: Vec<String>,
    /// Authorship marks on the title.
    pub marks: Vec<Mark>,
    pub body: Vec<Node>,
}

/// Who wrote what: a comment the project file names as a mark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Mark {
    pub who: String,
    pub note: Option<String>,
}

/// One step of a label. Every node has `type` and `line` (in the file, from 1); `notes`
/// and `marks` are the comments right before it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Node {
    pub line: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<Mark>,
    #[serde(flatten)]
    pub kind: Kind,
}

/// What a node is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Kind {
    /// A line of dialogue, spoken (`speaker`) or narrated.
    Line(Line),
    /// Choices shown together.
    Menu {
        choices: Vec<Choice>,
    },
    /// `if` / `elif` / `else`: the first arm whose condition holds plays.
    Branch {
        arms: Vec<Arm>,
    },
    /// `match subject`, with `when` arms (an arm with no condition is the `else`).
    Match {
        subject: String,
        arms: Vec<Arm>,
    },
    /// Plays its body again while the condition holds.
    While {
        condition: String,
        body: Vec<Node>,
    },
    /// One of the options, at random (`%` lines).
    Random {
        options: Vec<RandomOption>,
    },
    /// Game code: `do`, `set`...
    Effect {
        /// `do`, `do!`, `set`, `$>` or `$>>`.
        keyword: String,
        expression: String,
    },
    Jump(Jump),
    /// A title inside a block: a jump target.
    Anchor {
        id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Line {
    /// The speaker as written; none for narration.
    pub speaker: Option<String>,
    /// The character id the speaker belongs to, from the project file.
    pub character: Option<String>,
    /// The words as written: Dialogue Manager's markup (`[wave]`, `{{name}}`, `[[a|b]]`,
    /// `[if ...]`) included.
    pub text: String,
    /// The words without markup, random options as `a/b`.
    pub plain: String,
    /// `[#name=value]` tags, in order. `value` is null for a tag with none.
    pub tags: Vec<Tag>,
    /// The static line id (`[ID:...]`), the translation key.
    pub id: Option<String>,
    /// Shown together with the line before (`| line`).
    #[serde(default)]
    pub concurrent: bool,
    /// A jump at the end of the line.
    pub jump: Option<Jump>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Choice {
    pub line: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    pub speaker: Option<String>,
    /// The words as written, markup included.
    pub text: String,
    /// The words without markup.
    pub plain: String,
    pub tags: Vec<Tag>,
    pub id: Option<String>,
    /// Offered only when this holds (`[if ...]`).
    pub condition: Option<String>,
    pub jump: Option<Jump>,
    /// What plays when it is picked.
    pub body: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Arm {
    pub line: usize,
    /// `if`, `elif`, `else` or `when`.
    pub keyword: String,
    /// The condition (or `when` value); none on `else`.
    pub condition: Option<String>,
    pub body: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RandomOption {
    pub line: usize,
    /// Relative weight (`%2`); none is 1.
    pub weight: Option<String>,
    pub condition: Option<String>,
    pub body: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Tag {
    pub name: String,
    pub value: Option<String>,
}

/// Where the dialogue goes next.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Jump {
    /// `label` (with `label`), `end` (this title's flow ends), `end_conversation`, or
    /// `expression` (a label computed at run time, with `expression`).
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    /// Jump, then come back (`=><`).
    #[serde(default)]
    pub returns: bool,
}

/// The script as [`ScriptExport`].
pub fn export(project: &Project, script: &Script) -> ScriptExport {
    let source = Source::new(project);
    let config = &project.config;

    let mut flags: BTreeMap<&str, Flag> = BTreeMap::new();
    for r in &script.flags {
        let flag = flags.entry(&r.flag).or_insert_with(|| Flag {
            name: r.flag.clone(),
            meaning: config.flags.meanings.get(&r.flag).cloned(),
            set_in: Vec::new(),
            read_in: Vec::new(),
        });
        let list = match r.access {
            model::FlagAccess::Set => &mut flag.set_in,
            model::FlagAccess::Read => &mut flag.read_in,
        };
        if let Some(label) = &r.label
            && !list.contains(label)
        {
            list.push(label.clone());
        }
    }

    ScriptExport {
        format: FORMAT.into(),
        version: VERSION,
        name: config.name.clone(),
        characters: config
            .characters
            .iter()
            .map(|c| Character {
                id: c.id.clone(),
                name: c.name.clone(),
                speakers: c.speakers.clone(),
                role: c.role.clone(),
                portrait: c.portrait.clone(),
                voices: c.voices.clone(),
            })
            .collect(),
        places: config
            .places
            .iter()
            .map(|p| Place {
                id: p.id.clone(),
                name: p.name.clone(),
                group: p.group.clone(),
            })
            .collect(),
        flags: flags.into_values().collect(),
        files: script
            .files
            .iter()
            .map(|f| File {
                path: f.path.clone(),
                preamble: nodes(&source, &f.preamble),
                labels: f
                    .sections
                    .iter()
                    .flat_map(|s| {
                        s.labels.iter().map(|l| Label {
                            id: l.id.clone(),
                            line: source.line(l.span),
                            place: l.place.clone(),
                            section: s.title.clone(),
                            notes: l.notes.iter().map(|n| n.text.clone()).collect(),
                            marks: marks(&l.marks),
                            body: nodes(&source, &l.body),
                        })
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// The script as JSON text.
pub fn to_string(project: &Project, script: &Script) -> String {
    let mut text = serde_json::to_string_pretty(&export(project, script)).expect("plain data");
    text.push('\n');
    text
}

/// The JSON Schema of the format.
pub fn schema() -> String {
    let mut text =
        serde_json::to_string_pretty(&schemars::schema_for!(ScriptExport)).expect("plain data");
    text.push('\n');
    text
}

fn marks(marks: &[model::Mark]) -> Vec<Mark> {
    marks
        .iter()
        .map(|m| Mark {
            who: m.who.clone(),
            note: m.note.clone(),
        })
        .collect()
}

fn tags(tags: &[model::Tag]) -> Vec<Tag> {
    tags.iter()
        .map(|t| Tag {
            name: t.name.clone(),
            value: t.value.clone(),
        })
        .collect()
}

fn jump(jump: &model::Jump) -> Jump {
    let (to, label, expression) = match &jump.target {
        JumpTarget::Label(l) => ("label", Some(l.clone()), None),
        JumpTarget::End => ("end", None, None),
        JumpTarget::EndConversation => ("end_conversation", None, None),
        JumpTarget::Expression(e) => ("expression", None, Some(e.clone())),
    };
    Jump {
        to: to.into(),
        label,
        expression,
        returns: jump.returns,
    }
}

fn nodes(source: &Source, nodes: &[ModelNode]) -> Vec<Node> {
    nodes.iter().map(|n| node(source, n)).collect()
}

fn arms(source: &Source, arms: &[model::Arm]) -> Vec<Arm> {
    arms.iter()
        .map(|a| Arm {
            line: source.line(a.span),
            keyword: match a.keyword {
                ArmKeyword::If => "if",
                ArmKeyword::Elif => "elif",
                ArmKeyword::Else => "else",
                ArmKeyword::When => "when",
            }
            .into(),
            condition: a.condition.clone(),
            body: nodes(source, &a.body),
        })
        .collect()
}

fn node(source: &Source, node: &ModelNode) -> Node {
    let kind = match &node.kind {
        NodeKind::Line(line) => Kind::Line(Line {
            speaker: line.speaker.clone(),
            character: line
                .speaker
                .as_deref()
                .and_then(|s| source.character(s))
                .map(|c| c.id.clone()),
            text: line.words.trim().to_owned(),
            plain: plain(line),
            tags: tags(&line.tags),
            id: line.id.clone(),
            concurrent: line.concurrent,
            jump: line.jump.as_ref().map(jump),
        }),
        NodeKind::Menu { choices } => Kind::Menu {
            choices: choices
                .iter()
                .map(|c| Choice {
                    line: source.line(c.span),
                    notes: c.notes.iter().map(|n| n.text.clone()).collect(),
                    speaker: c.speaker.clone(),
                    text: c.words.trim().to_owned(),
                    plain: model::Segment::plain(&c.text).trim().to_owned(),
                    tags: tags(&c.tags),
                    id: c.id.clone(),
                    condition: c.condition.clone(),
                    jump: c.jump.as_ref().map(jump),
                    body: nodes(source, &c.body),
                })
                .collect(),
        },
        NodeKind::Branch { arms: a } => Kind::Branch {
            arms: arms(source, a),
        },
        NodeKind::Match { subject, arms: a } => Kind::Match {
            subject: subject.clone(),
            arms: arms(source, a),
        },
        NodeKind::While { condition, body } => Kind::While {
            condition: condition.clone(),
            body: nodes(source, body),
        },
        NodeKind::Random { options } => Kind::Random {
            options: options
                .iter()
                .map(|o| RandomOption {
                    line: source.line(o.span),
                    weight: o.weight.clone(),
                    condition: o.condition.clone(),
                    body: nodes(source, &o.body),
                })
                .collect(),
        },
        NodeKind::Effect(e) => Kind::Effect {
            keyword: e.keyword.clone(),
            expression: e.expr.clone(),
        },
        NodeKind::Jump(j) => Kind::Jump(jump(j)),
        NodeKind::Anchor { id } => Kind::Anchor { id: id.clone() },
    };
    Node {
        line: source.line(node.span),
        notes: node.notes.iter().map(|n| n.text.clone()).collect(),
        marks: marks(&node.marks),
        kind,
    }
}
