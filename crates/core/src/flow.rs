//! How the story moves between labels: every jump, which way it goes (always, from a
//! choice, under a condition, there and back, falling through), and where the game starts.

use std::collections::HashSet;

use regex::Regex;
use serde::Serialize;
use ts_rs::TS;

use crate::model::*;
use crate::project::Project;

#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
pub struct Flow {
    pub edges: Vec<FlowEdge>,
    /// Labels the game starts by name (its code names them, or an entry point pattern
    /// matches), as `file:label`.
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct FlowEdge {
    /// `file:label`, the file being its index in the script.
    pub from: String,
    pub to: String,
    pub kind: FlowKind,
    /// For a `flag` edge: the flag the first label sets and the second reads.
    pub flag: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum FlowKind {
    /// A jump every time the label plays this far.
    Jump,
    /// A jump the player picks with a choice.
    Choice,
    /// A jump under a condition (or a random pick).
    Conditional,
    /// `=><`: there, then back.
    Return,
    /// The label ends without a jump and the next one plays.
    FallThrough,
    /// The first label sets a flag the second one reads.
    Flag,
}

/// The key of a label in the flow: `file:label`.
pub fn key(file: FileId, label: &str) -> String {
    format!("{file}:{label}")
}

pub fn flow(project: &Project, script: &Script) -> Flow {
    let mut edges = Vec::new();
    for (file_id, file) in script.files.iter().enumerate() {
        let titles: HashSet<&str> = file
            .sections
            .iter()
            .flat_map(|s| &s.labels)
            .map(|l| l.id.as_str())
            .collect();
        let labels: Vec<&Label> = file.sections.iter().flat_map(|s| &s.labels).collect();
        for (i, label) in labels.iter().enumerate() {
            let from = key(file_id, &label.id);
            let mut add = |target: &str, kind: FlowKind| {
                if titles.contains(target) {
                    let edge = FlowEdge {
                        from: from.clone(),
                        to: key(file_id, target),
                        kind,
                        flag: None,
                    };
                    if !edges.contains(&edge) {
                        edges.push(edge);
                    }
                }
            };
            jumps(&label.body, Context::Always, &mut add);
            if falls_through(&label.body)
                && let Some(next) = labels.get(i + 1)
            {
                add(&next.id, FlowKind::FallThrough);
            }
        }
    }
    // Flags: from each label that sets one to each label that reads it.
    let mut setters: Vec<(&str, String)> = Vec::new();
    let mut readers: Vec<(&str, String)> = Vec::new();
    for reference in &script.flags {
        let Some(label) = &reference.label else {
            continue;
        };
        let k = key(reference.span.file, label);
        let list = match reference.access {
            FlagAccess::Set => &mut setters,
            FlagAccess::Read => &mut readers,
        };
        if !list.contains(&(reference.flag.as_str(), k.clone())) {
            list.push((reference.flag.as_str(), k));
        }
    }
    for (flag, from) in &setters {
        for (_, to) in readers.iter().filter(|(f, to)| f == flag && to != from) {
            edges.push(FlowEdge {
                from: from.clone(),
                to: to.clone(),
                kind: FlowKind::Flag,
                flag: Some((*flag).to_owned()),
            });
        }
    }

    let named = CodeNames::read(&project.code_files());
    let entries = script
        .files
        .iter()
        .enumerate()
        .flat_map(|(file_id, file)| {
            file.sections
                .iter()
                .flat_map(|s| &s.labels)
                .map(move |l| (file_id, l))
        })
        .filter(|(_, l)| named.names(&l.id) || project.compiled.entry_points.is_match(&l.id))
        .map(|(file_id, l)| key(file_id, &l.id))
        .fold(Vec::new(), |mut all: Vec<String>, k| {
            if !all.contains(&k) {
                all.push(k);
            }
            all
        });
    Flow { edges, entries }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Context {
    Always,
    Choice,
    Conditional,
}

/// One jump to a label, as an edge of the kind its context gives it.
fn emit(jump: &Jump, context: Context, add: &mut impl FnMut(&str, FlowKind)) {
    let JumpTarget::Label(name) = &jump.target else {
        return;
    };
    let kind = match (jump.returns, context) {
        (true, _) => FlowKind::Return,
        (false, Context::Always) => FlowKind::Jump,
        (false, Context::Choice) => FlowKind::Choice,
        (false, Context::Conditional) => FlowKind::Conditional,
    };
    add(name, kind);
}

/// Every jump in these nodes, with how it is reached.
fn jumps(nodes: &[Node], context: Context, add: &mut impl FnMut(&str, FlowKind)) {
    for node in nodes {
        match &node.kind {
            NodeKind::Jump(jump) => emit(jump, context, add),
            NodeKind::Line(line) => {
                if let Some(jump) = &line.jump {
                    emit(jump, context, add);
                }
            }
            NodeKind::Menu { choices } => {
                for choice in choices {
                    let inner = if choice.condition.is_some() {
                        Context::Conditional
                    } else {
                        Context::Choice
                    };
                    let inner = if context == Context::Conditional {
                        Context::Conditional
                    } else {
                        inner
                    };
                    if let Some(jump) = &choice.jump {
                        emit(jump, inner, add);
                    }
                    jumps(&choice.body, inner, add);
                }
            }
            NodeKind::Branch { arms } | NodeKind::Match { arms, .. } => {
                for arm in arms {
                    jumps(&arm.body, Context::Conditional, add);
                }
            }
            NodeKind::While { body, .. } => jumps(body, Context::Conditional, add),
            NodeKind::Random { options } => {
                for option in options {
                    jumps(&option.body, Context::Conditional, add);
                }
            }
            NodeKind::Effect(_) | NodeKind::Anchor { .. } => {}
        }
    }
}

/// Can the flow run off the end of these nodes into what follows?
pub(crate) fn falls_through(nodes: &[Node]) -> bool {
    !matches!(
        nodes.last().map(|n| &n.kind),
        Some(NodeKind::Jump(Jump { returns: false, .. }))
    )
}

/// The strings in quotes in the game's code: names it may start a label by.
pub(crate) struct CodeNames {
    exact: HashSet<String>,
    /// Strings with format placeholders (`"urso_generico_%d"`, `"{npc}_talk"`) as patterns.
    templates: Vec<Regex>,
}

impl CodeNames {
    pub(crate) fn read(files: &[(String, String)]) -> Self {
        let quoted = Regex::new(r#""([^"\\\r\n]*)"|'([^'\\\r\n]*)'"#).expect("valid regex");
        let placeholder =
            Regex::new(r"%[-+ 0#]*\d*(?:\.\d+)?[sdifxXo]|\{[A-Za-z0-9_]*\}").expect("valid regex");
        let mut names = Self {
            exact: HashSet::new(),
            templates: Vec::new(),
        };
        let strings = files.iter().flat_map(|(_, text)| {
            quoted
                .captures_iter(text)
                .filter_map(|c| c.get(1).or_else(|| c.get(2)))
        });
        for string in strings.map(|m| m.as_str()) {
            if !placeholder.is_match(string) {
                names.exact.insert(string.to_owned());
                continue;
            }
            // Only templates that could be a name: identifier characters around the placeholders.
            let literal = placeholder.replace_all(string, "");
            if literal.len() < 3 || !literal.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let parts: Vec<String> = placeholder.split(string).map(regex::escape).collect();
            if let Ok(template) = Regex::new(&format!("^{}$", parts.join(".+"))) {
                names.templates.push(template);
            }
        }
        names
    }

    pub(crate) fn names(&self, label: &str) -> bool {
        self.exact.contains(label) || self.templates.iter().any(|t| t.is_match(label))
    }
}
