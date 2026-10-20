//! The script model: what the views show and the checks read. Built from the syntax trees
//! (see [`crate::lower`]); every part keeps the span it came from, so an edit can go back
//! to the exact bytes.

use serde::Serialize;
use ts_rs::TS;

/// Index of a file in [`crate::Project::files`].
pub type FileId = usize;

/// Where something is: a file and a byte range in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, TS)]
#[ts(export)]
pub struct Span {
    pub file: FileId,
    pub start: usize,
    pub end: usize,
}

/// All the dialogue of a project.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
pub struct Script {
    pub files: Vec<ScriptFile>,
    /// Every place a flag is set or read, in file order.
    pub flags: Vec<FlagRef>,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
pub struct ScriptFile {
    /// Relative to the project root, with `/`.
    pub path: String,
    /// `import "path" as alias`.
    pub imports: Vec<Import>,
    /// Comments at the top, before the first section or label.
    pub notes: Vec<Note>,
    /// Lines before the first label (they play when the file starts from the top).
    pub preamble: Vec<Node>,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Import {
    pub path: String,
    pub alias: String,
    pub span: Span,
}

/// Labels under one section header (or before any).
#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
pub struct Section {
    pub title: Option<String>,
    pub notes: Vec<Note>,
    pub labels: Vec<Label>,
}

/// A title (`~ name`) and the lines up to the next one.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Label {
    pub id: String,
    /// The `~ name` line.
    pub span: Span,
    /// The place it plays in, from the project file.
    pub place: Option<String>,
    pub notes: Vec<Note>,
    pub marks: Vec<Mark>,
    pub body: Vec<Node>,
}

/// A comment kept with what follows it.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Note {
    /// Without the `#` (or `##`) and surrounding blanks.
    pub text: String,
    pub span: Span,
}

/// Who wrote what: a comment with a marker from the project file.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Mark {
    pub who: String,
    pub note: Option<String>,
    pub span: Span,
}

/// One step of a label: a line, a menu, a branch...
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Node {
    pub span: Span,
    pub notes: Vec<Note>,
    pub marks: Vec<Mark>,
    #[serde(flatten)]
    pub kind: NodeKind,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeKind {
    Line(Line),
    /// Choices shown together.
    Menu {
        choices: Vec<Choice>,
    },
    /// `if` / `elif` / `else`.
    Branch {
        arms: Vec<Arm>,
    },
    /// `match subject` with `when` arms (and maybe an `else`).
    Match {
        subject: String,
        arms: Vec<Arm>,
    },
    While {
        condition: String,
        body: Vec<Node>,
    },
    /// One of these, at random (`%` lines).
    Random {
        options: Vec<RandomOption>,
    },
    /// `do`, `set`...
    Effect(Effect),
    Jump(Jump),
    /// A title nested inside a block: Dialogue Manager accepts it as a jump target.
    Anchor {
        id: String,
    },
}

/// A line of dialogue, spoken or narrated.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
pub struct Line {
    pub speaker: Option<String>,
    pub text: Vec<Segment>,
    pub tags: Vec<Tag>,
    /// A jump at the end of the line.
    pub jump: Option<Jump>,
    /// `[ID:...]`
    pub id: Option<String>,
    /// `| line`: shown with the line before.
    pub concurrent: bool,
}

/// A response the player can pick.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Choice {
    pub span: Span,
    pub notes: Vec<Note>,
    pub marks: Vec<Mark>,
    pub speaker: Option<String>,
    pub text: Vec<Segment>,
    pub tags: Vec<Tag>,
    /// `[if ...]`: shown only when this holds.
    pub condition: Option<String>,
    pub jump: Option<Jump>,
    pub id: Option<String>,
    pub body: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Arm {
    pub keyword: ArmKeyword,
    /// The condition (or `when` value); none on `else`.
    pub condition: Option<String>,
    pub span: Span,
    pub notes: Vec<Note>,
    pub marks: Vec<Mark>,
    pub body: Vec<Node>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum ArmKeyword {
    If,
    Elif,
    Else,
    When,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct RandomOption {
    pub weight: Option<String>,
    pub condition: Option<String>,
    pub span: Span,
    pub body: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Effect {
    /// `do`, `do!`, `set`, `$>` or `$>>`.
    pub keyword: String,
    pub expr: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Jump {
    pub target: JumpTarget,
    /// `=><`: jump, then come back.
    pub returns: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum JumpTarget {
    /// A title, maybe `alias/title` from an import.
    Label(String),
    /// `END`: the end of this title's flow.
    End,
    /// `END!`: the end of the conversation.
    EndConversation,
    /// `{{ expression }}`
    Expression(String),
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct Tag {
    pub name: String,
    pub value: Option<String>,
    pub span: Span,
}

/// A piece of a line's text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Segment {
    Text {
        text: String,
    },
    /// `[[a|b|c]]`: one of these, at random.
    Random {
        options: Vec<String>,
    },
    /// `{{ expression }}`
    Expr {
        expr: String,
    },
    /// BBCode and timing tags: `[wave]`, `[/wave]`, `[wait=1]`...
    Markup {
        name: String,
        args: Option<String>,
    },
    /// `[if condition]` ... `[else]` ... `[/if]`
    If {
        condition: String,
    },
    Else,
    EndIf,
    /// `[do ...]` in the middle of a line.
    Effect {
        expr: String,
    },
}

/// A flag set or read somewhere in the dialogue.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct FlagRef {
    pub flag: String,
    pub access: FlagAccess,
    /// The label it is in, if any.
    pub label: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum FlagAccess {
    Set,
    Read,
}

impl Script {
    /// All labels, in file and section order.
    pub fn labels(&self) -> impl Iterator<Item = &Label> {
        self.files
            .iter()
            .flat_map(|f| f.sections.iter().flat_map(|s| &s.labels))
    }
}

impl Node {
    /// This node and every node nested in it, depth first, in order.
    pub fn walk<'a>(&'a self, visit: &mut impl FnMut(&'a Node)) {
        visit(self);
        let mut each = |nodes: &'a [Node]| nodes.iter().for_each(|n| n.walk(visit));
        match &self.kind {
            NodeKind::Menu { choices } => choices.iter().for_each(|c| each(&c.body)),
            NodeKind::Branch { arms } | NodeKind::Match { arms, .. } => {
                arms.iter().for_each(|a| each(&a.body))
            }
            NodeKind::While { body, .. } => each(body),
            NodeKind::Random { options } => options.iter().for_each(|o| each(&o.body)),
            NodeKind::Line(_)
            | NodeKind::Effect(_)
            | NodeKind::Jump(_)
            | NodeKind::Anchor { .. } => {}
        }
    }
}

/// Walk a sequence of nodes and everything nested in them.
pub fn walk<'a>(nodes: &'a [Node], visit: &mut impl FnMut(&'a Node)) {
    nodes.iter().for_each(|n| n.walk(visit));
}

impl Segment {
    /// The segments as plain text: random options joined with `/`, markup left out.
    pub fn plain(segments: &[Segment]) -> String {
        let mut out = String::new();
        for segment in segments {
            match segment {
                Segment::Text { text } => out.push_str(text),
                Segment::Random { options } => out.push_str(&options.join("/")),
                Segment::Expr { expr } => {
                    out.push_str("{{");
                    out.push_str(expr);
                    out.push_str("}}");
                }
                _ => {}
            }
        }
        out
    }
}
