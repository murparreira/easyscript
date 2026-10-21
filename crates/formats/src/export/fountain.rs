//! The script as a screenplay, in Fountain (<https://fountain.io>), for screenwriting apps
//! and print. Each label is a scene; spoken lines are dialogue, with the mood as a
//! parenthetical; narration is action. What a screenplay has no form for (choices,
//! branches, jumps) is written as forced action lines starting with `—` or `→`; game code
//! goes in notes (`[[ ]]`).

use std::fmt::Write as _;

use dm_core::model::{JumpTarget, Line, Node, NodeKind, Segment};
use dm_core::{Project, Script};

use super::words::{Language, Words, fill};
use super::{Source, plain};

pub fn to_fountain(project: &Project, script: &Script, language: Language) -> String {
    let source = Source::new(project);
    let mut w = Writer {
        source: &source,
        words: language.words(),
        mood: source.tag_shown_as("voice"),
        thought: source.tag_shown_as("thought"),
        shake: source.tag_shown_as("shake"),
        out: String::new(),
    };
    if let Some(name) = &project.config.name {
        let _ = writeln!(w.out, "Title: {name}");
    }
    let _ = writeln!(w.out, "Credit: {}\n", w.words.credit);
    let places = &project.config.places;
    for file in &script.files {
        let _ = writeln!(w.out, "# {}\n", file.path);
        if !file.preamble.is_empty() {
            w.nodes(&file.preamble);
        }
        for section in &file.sections {
            if let Some(title) = &section.title {
                let _ = writeln!(w.out, "## {title}\n");
            }
            for label in &section.labels {
                let place = label
                    .place
                    .as_deref()
                    .and_then(|p| places.iter().find(|pl| pl.id == p))
                    .map(|p| format!(" - {}", p.name.to_uppercase()))
                    .unwrap_or_default();
                let _ = writeln!(w.out, ".{}{place}\n", label.id.to_uppercase());
                for note in &label.notes {
                    let _ = writeln!(w.out, "[[{}]]\n", note.text);
                }
                w.nodes(&label.body);
            }
        }
    }
    while w.out.ends_with("\n\n") {
        w.out.pop();
    }
    w.out
}

struct Writer<'a> {
    source: &'a Source<'a>,
    words: &'static Words,
    mood: Option<&'a str>,
    thought: Option<&'a str>,
    shake: Option<&'a str>,
    out: String,
}

impl Writer<'_> {
    fn action(&mut self, text: &str) {
        let _ = writeln!(self.out, "!{text}\n");
    }

    fn note(&mut self, text: &str) {
        let _ = writeln!(self.out, "[[{text}]]\n");
    }

    fn nodes(&mut self, nodes: &[Node]) {
        for node in nodes {
            for note in &node.notes {
                self.note(&note.text);
            }
            self.node(node);
        }
    }

    fn node(&mut self, node: &Node) {
        match &node.kind {
            NodeKind::Line(line) => self.line(line),
            NodeKind::Menu { choices } => {
                self.action(&format!("— {}", self.words.chooses));
                for choice in choices {
                    let condition = choice
                        .condition
                        .as_deref()
                        .map(|c| format!(" ({})", fill(self.words.only_if, &[("condition", c)])))
                        .unwrap_or_default();
                    let text = Segment::plain(&choice.text);
                    self.action(&format!("— \"{}\"{condition}", text.trim()));
                    self.nodes(&choice.body);
                    if let Some(jump) = &choice.jump {
                        self.jump(&jump.target);
                    }
                }
                self.action(&format!("— {}", self.words.end_of_choices));
            }
            NodeKind::Branch { arms } | NodeKind::Match { arms, .. } => {
                let subject = match &node.kind {
                    NodeKind::Match { subject, .. } => Some(subject.as_str()),
                    _ => None,
                };
                for arm in arms {
                    let head = match (&arm.condition, subject) {
                        (Some(c), Some(s)) => format!(
                            "— {}",
                            fill(self.words.when_is, &[("subject", s), ("value", c)])
                        ),
                        (Some(c), None) => {
                            format!("— {}", fill(self.words.if_, &[("condition", c)]))
                        }
                        (None, _) => format!("— {}", self.words.otherwise),
                    };
                    self.action(&head);
                    self.nodes(&arm.body);
                }
                self.action(&format!("— {}", self.words.end_of_block));
            }
            NodeKind::While { condition, body } => {
                self.action(&format!(
                    "— {}",
                    fill(self.words.while_, &[("condition", condition)])
                ));
                self.nodes(body);
                self.action(&format!("— {}", self.words.end_of_block));
            }
            NodeKind::Random { options } => {
                self.action(&format!("— {}", self.words.at_random));
                for (n, option) in options.iter().enumerate() {
                    let weight = option
                        .weight
                        .as_deref()
                        .map(|w| format!(" ({})", fill(self.words.weight, &[("weight", w)])))
                        .unwrap_or_default();
                    let condition = option
                        .condition
                        .as_deref()
                        .map(|c| format!(" ({})", fill(self.words.only_if, &[("condition", c)])))
                        .unwrap_or_default();
                    self.action(&format!("— {}.{weight}{condition}", n + 1));
                    self.nodes(&option.body);
                }
                self.action(&format!("— {}", self.words.end_of_block));
            }
            NodeKind::Effect(effect) => self.note(&format!("{} {}", effect.keyword, effect.expr)),
            NodeKind::Jump(jump) => self.jump(&jump.target),
            NodeKind::Anchor { id } => self.note(&format!("~ {id}")),
        }
    }

    fn jump(&mut self, target: &JumpTarget) {
        let text = match target {
            JumpTarget::Label(label) => format!("→ {}", label.to_uppercase()),
            JumpTarget::End => format!("→ {}", self.words.end),
            JumpTarget::EndConversation => format!("→ {}", self.words.end_of_conversation),
            JumpTarget::Expression(e) => format!("→ {{{{{e}}}}}"),
        };
        self.action(&text);
    }

    fn line(&mut self, line: &Line) {
        let text = plain(line);
        let Some(speaker) = &line.speaker else {
            for paragraph in text.lines() {
                self.action(paragraph);
            }
            if let Some(jump) = &line.jump {
                self.jump(&jump.target);
            }
            return;
        };
        let name = self
            .source
            .character(speaker)
            .map_or(speaker.as_str(), |c| c.name.as_str());
        let dual = if line.concurrent { " ^" } else { "" };
        let _ = writeln!(self.out, "@{}{dual}", name.to_uppercase());
        let mut parenthetical = Vec::new();
        if let Some(mood) = self.source.tag(&line.tags, self.mood).flatten() {
            parenthetical.push(mood.to_owned());
        }
        if self
            .source
            .tag(&line.tags, self.thought)
            .is_some_and(|v| v != Some("false"))
        {
            parenthetical.push(self.words.thinking.to_owned());
        }
        if self
            .source
            .tag(&line.tags, self.shake)
            .is_some_and(|v| v != Some("false"))
        {
            parenthetical.push(self.words.shaking.to_owned());
        }
        if !parenthetical.is_empty() {
            let _ = writeln!(self.out, "({})", parenthetical.join(", "));
        }
        // A blank line would end the dialogue: keep paragraphs together.
        let text = text
            .lines()
            .map(|l| if l.trim().is_empty() { "  " } else { l })
            .collect::<Vec<_>>()
            .join("\n");
        let _ = writeln!(self.out, "{text}\n");
        if let Some(jump) = &line.jump {
            self.jump(&jump.target);
        }
    }
}
