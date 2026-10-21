//! Problems a script has that can be found without running the game.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::Path;

use es_syntax::ast::Line as AstLine;
use regex::Regex;
use serde::Serialize;
use ts_rs::TS;

use crate::config::TagKind;
use crate::flow::{CodeNames, falls_through};
use crate::model::*;
use crate::project::Project;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Diagnostic {
    pub severity: Severity,
    /// A short stable name for the kind of problem (`unknown-jump-target`).
    pub code: &'static str,
    /// In English.
    pub message: String,
    /// The values the message names, by name (`speaker`, `count`...), so a view can say it
    /// in another language. Which names each code has: see `app/ui/src/lib/diagnostics.ts`.
    pub args: BTreeMap<String, String>,
    pub location: Option<Location>,
}

/// A place in a file: a dialogue file, the project file or a document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[ts(export)]
pub struct Location {
    pub path: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(at) = &self.location {
            write!(f, "{}:{}:{}: ", at.path, at.line, at.column)?;
        }
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{severity}[{}]: {}", self.code, self.message)
    }
}

/// Run every check. Diagnostics come sorted by file and position.
pub fn check(project: &Project, script: &Script) -> Vec<Diagnostic> {
    let mut checker = Checker {
        project,
        script,
        out: Vec::new(),
    };
    checker.unreadable();
    checker.syntax();
    checker.jumps();
    checker.speakers();
    checker.portraits();
    checker.voices();
    checker.tags();
    checker.unreachable();
    checker.places();
    checker.documents();
    let mut out = checker.out;
    out.retain(|d| {
        !project
            .config
            .checks
            .ignore
            .iter()
            .any(|code| code == d.code)
    });
    out.sort_by(|a, b| (&a.location, a.severity).cmp(&(&b.location, b.severity)));
    out
}

struct Checker<'a> {
    project: &'a Project,
    script: &'a Script,
    out: Vec<Diagnostic>,
}

impl Checker<'_> {
    fn at(&self, span: Span) -> Option<Location> {
        let file = self.project.files.get(span.file)?;
        let (line, column) = file.line_col(span.start);
        Some(Location {
            path: file.path.clone(),
            line,
            column,
        })
    }

    /// Where `needle` first appears in the project file, if it does.
    fn in_config(&self, needle: &str) -> Option<Location> {
        let path = self.project.config_path.as_ref()?;
        let text = fs::read_to_string(path).ok()?;
        let offset = text.find(needle)?;
        Some(location(self.shown(path), &text, offset))
    }

    /// A path as the user should see it: relative to the project root when it is inside.
    fn shown(&self, path: &Path) -> String {
        let path = path.strip_prefix(&self.project.root).unwrap_or(path);
        path.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }

    fn push(
        &mut self,
        severity: Severity,
        code: &'static str,
        message: String,
        args: Args,
        location: Option<Location>,
    ) {
        self.out.push(Diagnostic {
            severity,
            code,
            message,
            args: args.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
            location,
        });
    }

    fn unreadable(&mut self) {
        for (path, why) in &self.project.unreadable {
            let location = Some(Location {
                path: path.clone(),
                line: 1,
                column: 1,
            });
            self.push(
                Severity::Error,
                "unreadable-file",
                format!("cannot read this file ({why}); it is left alone"),
                vec![("why", why.clone())],
                location,
            );
        }
    }

    fn syntax(&mut self) {
        for (id, file) in self.project.files.iter().enumerate() {
            for token in file.parse.errors() {
                let span = Span {
                    file: id,
                    start: token.text_range().start().into(),
                    end: token.text_range().end().into(),
                };
                self.push(
                    Severity::Error,
                    "syntax",
                    format!("cannot read `{}`", token.text()),
                    vec![("text", token.text().to_owned())],
                    self.at(span),
                );
            }
            let mut seen = HashSet::new();
            for line in file.parse.tree().all_lines() {
                if let AstLine::Title(title) = &line
                    && let Some(name) = title.name()
                    && !seen.insert(name.text().to_owned())
                {
                    let span = Span {
                        file: id,
                        start: name.text_range().start().into(),
                        end: name.text_range().end().into(),
                    };
                    self.push(
                        Severity::Error,
                        "duplicate-label",
                        format!("`{}` is already a title in this file", name.text()),
                        vec![("label", name.text().to_owned())],
                        self.at(span),
                    );
                }
            }
        }
    }

    /// Titles a jump in each file can reach: its own, and `alias/title` from its imports.
    fn jump_targets(&self) -> Vec<HashSet<String>> {
        let own: Vec<HashSet<String>> = self
            .project
            .files
            .iter()
            .map(|f| titles(f.parse.tree().all_lines()))
            .collect();
        self.script
            .files
            .iter()
            .enumerate()
            .map(|(id, file)| {
                let mut known = own[id].clone();
                for import in &file.imports {
                    let path = import.path.strip_prefix("res://").unwrap_or(&import.path);
                    if let Some(other) = self.project.file_index(path) {
                        known.extend(own[other].iter().map(|t| format!("{}/{t}", import.alias)));
                    }
                }
                known
            })
            .collect()
    }

    fn jumps(&mut self) {
        let known = self.jump_targets();
        let mut bad = Vec::new();
        for_each_jump(self.script, |jump| {
            if let JumpTarget::Label(name) = &jump.target
                && !known[jump.span.file].contains(name)
            {
                bad.push((name.clone(), jump.span));
            }
        });
        for (name, span) in bad {
            let message = if name.is_empty() {
                "jump with no target".to_owned()
            } else {
                format!("no title `{name}` to jump to")
            };
            self.push(
                Severity::Error,
                "unknown-jump-target",
                message,
                vec![("label", name)],
                self.at(span),
            );
        }
    }

    fn speakers(&mut self) {
        let config = &self.project.config;
        if config.characters.is_empty() {
            return;
        }
        let mut unknown: BTreeMap<String, (Span, usize)> = BTreeMap::new();
        for_each_text_line(self.script, |speaker, _, span| {
            if let Some(speaker) = speaker
                && !speaker.contains("{{")
                && config.character_of_speaker(speaker).is_none()
            {
                unknown.entry(speaker.to_owned()).or_insert((span, 0)).1 += 1;
            }
        });
        for (speaker, (span, count)) in unknown {
            let message = format!("speaker `{speaker}` is no character's ({})", lines(count));
            let args = vec![("speaker", speaker), ("count", count.to_string())];
            self.push(
                Severity::Warning,
                "unknown-speaker",
                message,
                args,
                self.at(span),
            );
        }
    }

    fn portraits(&mut self) {
        for character in &self.project.config.characters {
            if let Some(portrait) = &character.portrait
                && !self.project.resolve(portrait).is_file()
            {
                let message = format!("{}'s portrait `{portrait}` does not exist", character.id);
                self.push(
                    Severity::Error,
                    "missing-portrait",
                    message,
                    vec![
                        ("character", character.id.clone()),
                        ("path", portrait.clone()),
                    ],
                    self.in_config(portrait),
                );
            }
        }
    }

    /// Lines asking for a voice mood their speaker has no recording in.
    fn voices(&mut self) {
        let config = &self.project.config;
        let Some((voice, _)) = config
            .tags
            .iter()
            .find(|(_, spec)| spec.show.as_deref() == Some("voice"))
        else {
            return;
        };
        let mut missing: BTreeMap<(String, String), (Span, usize)> = BTreeMap::new();
        for_each_text_line(self.script, |speaker, tags, span| {
            let Some(character) = speaker.and_then(|s| config.character_of_speaker(s)) else {
                return;
            };
            if character.voices.is_empty() {
                return;
            }
            for tag in tags.iter().filter(|t| t.name == *voice) {
                if let Some(mood) = &tag.value
                    && !character.voices.contains(mood)
                {
                    missing
                        .entry((character.id.clone(), mood.clone()))
                        .or_insert((span, 0))
                        .1 += 1;
                }
            }
        });
        for ((id, mood), (span, count)) in missing {
            let message = format!("{id} has no `{mood}` voice recorded ({})", lines(count));
            let args = vec![
                ("character", id),
                ("mood", mood),
                ("count", count.to_string()),
            ];
            self.push(
                Severity::Warning,
                "missing-voice",
                message,
                args,
                self.at(span),
            );
        }
    }

    fn tags(&mut self) {
        let schema = &self.project.config.tags;
        if schema.is_empty() {
            return;
        }
        let mut unknown: BTreeMap<String, (Span, usize)> = BTreeMap::new();
        let mut missing: BTreeMap<String, (Span, usize)> = BTreeMap::new();
        let mut bad = Vec::new();
        for_each_text_line(self.script, |_, tags, _| {
            for tag in tags {
                let Some(spec) = schema.get(&tag.name) else {
                    unknown.entry(tag.name.clone()).or_insert((tag.span, 0)).1 += 1;
                    continue;
                };
                let value = tag.value.as_deref();
                let problem = match (spec.kind, value) {
                    (TagKind::Flag, Some(v)) => Some((
                        format!("`{}` takes no value, has `{v}`", tag.name),
                        vec![
                            ("problem", "has-value".to_owned()),
                            ("tag", tag.name.clone()),
                            ("value", (*v).to_owned()),
                        ],
                    )),
                    (TagKind::Enum | TagKind::Asset | TagKind::Text, None) => Some((
                        format!("`{}` needs a value", tag.name),
                        vec![
                            ("problem", "needs-value".to_owned()),
                            ("tag", tag.name.clone()),
                        ],
                    )),
                    (TagKind::Enum, Some(v)) if !spec.values.iter().any(|x| x == v) => Some((
                        format!(
                            "`{v}` is not one of {}'s values ({})",
                            tag.name,
                            spec.values.join(", ")
                        ),
                        vec![
                            ("problem", "not-a-value".to_owned()),
                            ("tag", tag.name.clone()),
                            ("value", (*v).to_owned()),
                            ("values", spec.values.join(", ")),
                        ],
                    )),
                    (TagKind::Asset, Some(v)) => {
                        let path = spec
                            .path
                            .as_deref()
                            .unwrap_or("{value}")
                            .replace("{value}", v);
                        if !self.project.resolve(&path).is_file() {
                            missing.entry(path).or_insert((tag.span, 0)).1 += 1;
                        }
                        None
                    }
                    _ => None,
                };
                if let Some((message, args)) = problem {
                    bad.push((message, args, tag.span));
                }
            }
        });
        for (name, (span, count)) in unknown {
            self.push(
                Severity::Warning,
                "unknown-tag",
                format!("tag `{name}` is not in the project file ({})", lines(count)),
                vec![("tag", name), ("count", count.to_string())],
                self.at(span),
            );
        }
        for (message, args, span) in bad {
            self.push(
                Severity::Warning,
                "bad-tag-value",
                message,
                args,
                self.at(span),
            );
        }
        for (path, (span, count)) in missing {
            self.push(
                Severity::Error,
                "missing-asset",
                format!("`{path}` does not exist ({})", lines(count)),
                vec![("path", path), ("count", count.to_string())],
                self.at(span),
            );
        }
    }

    /// Labels nothing leads to: no jump, no fall-through from the label above, no code that
    /// names them, no entry point pattern. Needs `sources.code` to know what the game calls.
    fn unreachable(&mut self) {
        let config = &self.project.config;
        if config.sources.code.is_empty() && config.checks.entry_points.is_empty() {
            return;
        }
        let named = CodeNames::read(&self.project.code_files());
        let mut jumped: HashSet<(FileId, String)> = HashSet::new();
        for_each_jump(self.script, |jump| {
            if let JumpTarget::Label(name) = &jump.target {
                jumped.insert((jump.span.file, name.clone()));
            }
        });
        let mut unreachable = Vec::new();
        for (id, file) in self.script.files.iter().enumerate() {
            let mut falls_in = !file.preamble.is_empty() && falls_through(&file.preamble);
            for label in file.sections.iter().flat_map(|s| &s.labels) {
                let reached = falls_in
                    || jumped.contains(&(id, label.id.clone()))
                    || named.names(&label.id)
                    || self.project.compiled.entry_points.is_match(&label.id);
                if !reached {
                    unreachable.push((label.id.clone(), label.span));
                }
                falls_in = falls_through(&label.body);
            }
        }
        for (id, span) in unreachable {
            let message =
                format!("nothing leads to `{id}`: no jump, no code naming it, not an entry point");
            self.push(
                Severity::Warning,
                "unreachable-label",
                message,
                vec![("label", id)],
                self.at(span),
            );
        }
    }

    fn places(&mut self) {
        let mut labels_by_file: HashMap<&str, HashSet<&str>> = HashMap::new();
        for file in &self.script.files {
            labels_by_file.entry(&file.path).or_default().extend(
                file.sections
                    .iter()
                    .flat_map(|s| &s.labels)
                    .map(|l| l.id.as_str()),
            );
        }
        for place in &self.project.config.places {
            for path in &place.files {
                if !labels_by_file.contains_key(path.as_str()) {
                    let message = format!(
                        "place `{}`: `{path}` is not one of the dialogue files",
                        place.id
                    );
                    self.push(
                        Severity::Error,
                        "unknown-place-file",
                        message,
                        vec![("place", place.id.clone()), ("path", path.clone())],
                        self.in_config(path),
                    );
                }
            }
            let known = |label: &str| {
                place.files.iter().any(|f| {
                    labels_by_file
                        .get(f.as_str())
                        .is_some_and(|l| l.contains(label))
                })
            };
            for label in place.labels.iter().chain(&place.exclude_labels) {
                if !known(label) {
                    let message = format!("place `{}`: no label `{label}` in its files", place.id);
                    self.push(
                        Severity::Warning,
                        "unknown-place-label",
                        message,
                        vec![("place", place.id.clone()), ("label", label.clone())],
                        self.in_config(label),
                    );
                }
            }
        }
    }

    /// `[[label]]` links in the project's documents.
    fn documents(&mut self) {
        let labels: HashSet<&str> = self.script.labels().map(|l| l.id.as_str()).collect();
        let link = Regex::new(r"\[\[([^\[\]|]+?)\]\]").expect("valid regex");
        for document in &self.project.config.documents {
            let path = self.project.resolve_document(&document.path);
            let Ok(text) = fs::read_to_string(&path) else {
                let message = format!("document `{}` cannot be read", document.path);
                self.push(
                    Severity::Error,
                    "missing-document",
                    message,
                    vec![("path", document.path.clone())],
                    self.in_config(&document.path),
                );
                continue;
            };
            for found in link.captures_iter(&text) {
                let target = found.get(1).expect("one group");
                if labels.contains(target.as_str().trim()) {
                    continue;
                }
                let location = location(self.shown(&path), &text, target.start() - 2);
                self.push(
                    Severity::Error,
                    "broken-link",
                    format!("no label `{}`", target.as_str()),
                    vec![("label", target.as_str().to_owned())],
                    Some(location),
                );
            }
        }
    }
}

/// A diagnostic's values, by name.
type Args = Vec<(&'static str, String)>;

/// Line and column of a byte offset in `text`.
fn location(path: String, text: &str, offset: usize) -> Location {
    let before = &text[..offset];
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    Location {
        path,
        line: before.matches('\n').count() + 1,
        column: before[line_start..].chars().count() + 1,
    }
}

fn lines(count: usize) -> String {
    if count == 1 {
        "1 line".to_owned()
    } else {
        format!("{count} lines")
    }
}

fn titles(lines: impl Iterator<Item = AstLine>) -> HashSet<String> {
    lines
        .filter_map(|line| match line {
            AstLine::Title(title) => title.name().map(|n| n.text().to_owned()),
            _ => None,
        })
        .collect()
}

fn for_each_node(script: &Script, mut visit: impl FnMut(&Node)) {
    for file in &script.files {
        walk(&file.preamble, &mut visit);
        for label in file.sections.iter().flat_map(|s| &s.labels) {
            walk(&label.body, &mut visit);
        }
    }
}

fn for_each_jump(script: &Script, mut visit: impl FnMut(&Jump)) {
    for_each_node(script, |node| match &node.kind {
        NodeKind::Jump(jump) => visit(jump),
        NodeKind::Line(line) => line.jump.iter().for_each(&mut visit),
        NodeKind::Menu { choices } => choices
            .iter()
            .filter_map(|c| c.jump.as_ref())
            .for_each(&mut visit),
        _ => {}
    });
}

/// Speaker, tags and span of every dialogue line and choice.
fn for_each_text_line(script: &Script, mut visit: impl FnMut(Option<&str>, &[Tag], Span)) {
    for_each_node(script, |node| match &node.kind {
        NodeKind::Line(line) => visit(line.speaker.as_deref(), &line.tags, node.span),
        NodeKind::Menu { choices } => choices
            .iter()
            .for_each(|c| visit(c.speaker.as_deref(), &c.tags, c.span)),
        _ => {}
    });
}
