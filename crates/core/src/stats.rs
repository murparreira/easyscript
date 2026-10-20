//! Counts over a script.

use std::collections::{BTreeSet, HashMap};

use serde::Serialize;
use ts_rs::TS;

use crate::model::*;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Stats {
    pub files: usize,
    pub labels: usize,
    /// Dialogue lines (a line with lines nested under it counts once).
    pub lines: usize,
    pub choices: usize,
    /// Groups of choices shown together.
    pub menus: usize,
    /// `if` chains and `match`es.
    pub branches: usize,
    pub jumps: usize,
    pub effects: usize,
    /// Words in lines and choices.
    pub words: usize,
    /// Lines per speaker, most first; narration is the empty name.
    pub speakers: Vec<Count>,
    /// Uses of each tag, most first.
    pub tags: Vec<Count>,
    /// Marks per author, most first.
    pub marks: Vec<Count>,
    pub flags_set: usize,
    pub flags_read: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Count {
    pub name: String,
    pub count: usize,
}

pub fn stats(script: &Script) -> Stats {
    let mut stats = Stats {
        files: script.files.len(),
        ..Stats::default()
    };
    let mut speakers: HashMap<String, usize> = HashMap::new();
    let mut tags: HashMap<String, usize> = HashMap::new();
    let mut marks: HashMap<String, usize> = HashMap::new();
    let mut count_marks = |list: &[Mark]| {
        list.iter()
            .for_each(|m| *marks.entry(m.who.clone()).or_default() += 1)
    };

    for file in &script.files {
        let labels = file.sections.iter().flat_map(|s| &s.labels);
        let bodies = std::iter::once(&file.preamble).chain(labels.clone().map(|l| &l.body));
        for label in labels {
            stats.labels += 1;
            count_marks(&label.marks);
        }
        for body in bodies {
            walk(body, &mut |node: &Node| {
                count_marks(&node.marks);
                match &node.kind {
                    NodeKind::Line(line) => {
                        stats.lines += 1;
                        stats.words += words(&line.text);
                        *speakers
                            .entry(line.speaker.clone().unwrap_or_default())
                            .or_default() += 1;
                        line.tags
                            .iter()
                            .for_each(|t| *tags.entry(t.name.clone()).or_default() += 1);
                        stats.jumps += usize::from(line.jump.is_some());
                    }
                    NodeKind::Menu { choices } => {
                        stats.menus += 1;
                        stats.choices += choices.len();
                        for choice in choices {
                            count_marks(&choice.marks);
                            stats.words += words(&choice.text);
                            choice
                                .tags
                                .iter()
                                .for_each(|t| *tags.entry(t.name.clone()).or_default() += 1);
                            stats.jumps += usize::from(choice.jump.is_some());
                        }
                    }
                    NodeKind::Branch { .. } | NodeKind::Match { .. } => stats.branches += 1,
                    NodeKind::Jump(_) => stats.jumps += 1,
                    NodeKind::Effect(_) => stats.effects += 1,
                    NodeKind::While { .. } | NodeKind::Random { .. } | NodeKind::Anchor { .. } => {}
                }
            });
        }
    }
    let distinct = |access| {
        script
            .flags
            .iter()
            .filter(|f| f.access == access)
            .map(|f| &f.flag)
            .collect::<BTreeSet<_>>()
            .len()
    };
    stats.flags_set = distinct(FlagAccess::Set);
    stats.flags_read = distinct(FlagAccess::Read);
    stats.speakers = sorted(speakers);
    stats.tags = sorted(tags);
    stats.marks = sorted(marks);
    stats
}

fn words(text: &[Segment]) -> usize {
    Segment::plain(text).split_whitespace().count()
}

fn sorted(counts: HashMap<String, usize>) -> Vec<Count> {
    let mut out: Vec<Count> = counts
        .into_iter()
        .map(|(name, count)| Count { name, count })
        .collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    out
}
