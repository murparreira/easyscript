//! Edits to a dialogue file, made through its syntax tree.
//!
//! An edit names a line by where it starts (the `start` of a span in the model) and says
//! what to change. It finds that line in the tree and rewrites only that line's bytes, or
//! inserts, removes or moves whole lines together with the lines nested under them and the
//! comments written right above them. Everything else in the file stays byte for byte.
//!
//! The result is parsed again and checked against what was asked: text that would read as
//! a speaker, a tag or a keyword is written escaped, and an edit that would still change
//! more than it says is refused.

use std::fmt;
use std::ops::Range;

use es_syntax::SyntaxKind::{self, *};
use es_syntax::ast::{self, Line as AstLine};
use es_syntax::{AstNode, SyntaxElement, SyntaxNode};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(tag = "op", rename_all = "snake_case")]
#[ts(export)]
pub enum Edit {
    /// The words of a line or choice; its speaker, tags, jump and id stay.
    SetText {
        at: usize,
        text: String,
    },
    /// Who says it; none makes it narration.
    SetSpeaker {
        at: usize,
        speaker: Option<String>,
    },
    /// Add a tag, or change its value.
    SetTag {
        at: usize,
        name: String,
        value: Option<String>,
    },
    RemoveTag {
        at: usize,
        name: String,
    },
    /// The condition of an `if`, `elif`, `while`, `match` or `when` line, or of a choice
    /// (where none removes it).
    SetCondition {
        at: usize,
        condition: Option<String>,
    },
    /// A new line before or after another, at its indentation; or, `under` it, as the
    /// last line nested under a choice, an `if`, `elif`, `else`, `when` or `while`.
    InsertLine {
        at: usize,
        after: bool,
        #[serde(default)]
        under: bool,
        line: NewLine,
    },
    /// A line, the lines nested under it, and the comments right above it.
    RemoveLine {
        at: usize,
    },
    /// Swap a line (with what is nested under it and the comments above it) with the line
    /// above or below it at the same level.
    MoveLine {
        at: usize,
        up: bool,
    },
    /// A line of the game's code, `do ...` or `set ...`, placed as `InsertLine` places one.
    InsertCode {
        at: usize,
        after: bool,
        #[serde(default)]
        under: bool,
        code: String,
    },
    /// `if`, `elif` or `else` (with its condition, none for `else`) and a first line
    /// nested under it, placed as `InsertLine` places one: an `elif` or `else` goes after
    /// the last arm of its `if`.
    InsertCondition {
        at: usize,
        after: bool,
        #[serde(default)]
        under: bool,
        keyword: String,
        condition: Option<String>,
        line: NewLine,
    },
    /// Where a line or choice goes next (`=> title`, `=> END`, `=> END!`); none removes
    /// its jump.
    SetJump {
        at: usize,
        target: Option<String>,
    },
    /// A new label, `~ id`, that ends the conversation until lines are added: after the
    /// label whose title starts at `at`, or at the end of the file.
    AddLabel {
        at: Option<usize>,
        id: String,
    },
    /// A label: its title and its lines; and its notes, from `from` (where the comments
    /// the model gives the label start) when it has some above its title.
    RemoveLabel {
        at: usize,
        from: Option<usize>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct NewLine {
    /// A response (`- ...`) instead of a dialogue line.
    pub choice: bool,
    pub speaker: Option<String>,
    pub text: String,
    /// Tags to write on it.
    #[serde(default)]
    pub tags: Vec<NewTag>,
    /// For a choice: the condition that shows it.
    #[serde(default)]
    pub condition: Option<String>,
    /// Where it goes next (`=> title`); with no text and no choice, the line is only that.
    #[serde(default)]
    pub jump: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct NewTag {
    pub name: String,
    pub value: Option<String>,
}

/// The file after an edit, and where the line the edit was about starts now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    pub text: String,
    pub at: usize,
}

/// An edit that cannot be made as asked; the file is left as it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditError(pub String);

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EditError {}

fn fail<T>(message: impl Into<String>) -> Result<T, EditError> {
    Err(EditError(message.into()))
}

/// Make an edit to a file's text.
pub fn apply(text: &str, edit: &Edit) -> Result<Applied, EditError> {
    let root = es_syntax::parse(text).syntax();
    match edit {
        Edit::SetText { at, text: words } => set_text(text, &root, *at, words),
        Edit::SetSpeaker { at, speaker } => set_speaker(text, &root, *at, speaker.as_deref()),
        Edit::SetTag { at, name, value } => set_tag(text, &root, *at, name, value.as_deref()),
        Edit::RemoveTag { at, name } => remove_tag(text, &root, *at, name),
        Edit::SetCondition { at, condition } => {
            set_condition(text, &root, *at, condition.as_deref())
        }
        Edit::InsertLine {
            at,
            after,
            under,
            line,
        } => {
            // The line first, then its tags, condition and jump, each checked as its own edit.
            let mut applied = if *under {
                insert_under(text, &root, *at, line)?
            } else {
                insert_line(text, &root, *at, *after, line)?
            };
            if line.text.trim().is_empty() && !line.choice && line.jump.is_some() {
                return Ok(applied);
            }
            for tag in &line.tags {
                let root = es_syntax::parse(&applied.text).syntax();
                applied = set_tag(
                    &applied.text,
                    &root,
                    applied.at,
                    &tag.name,
                    tag.value.as_deref(),
                )?;
            }
            if line.condition.is_some() {
                let root = es_syntax::parse(&applied.text).syntax();
                applied =
                    set_condition(&applied.text, &root, applied.at, line.condition.as_deref())?;
            }
            if line.jump.is_some() {
                let root = es_syntax::parse(&applied.text).syntax();
                applied = set_jump(&applied.text, &root, applied.at, line.jump.as_deref())?;
            }
            Ok(applied)
        }
        Edit::InsertCode {
            at,
            after,
            under,
            code,
        } => {
            let code = code.trim();
            let Some(keyword) = ["do!", "do", "set"].into_iter().find(|k| {
                code.strip_prefix(k)
                    .is_some_and(|rest| rest.starts_with(' '))
            }) else {
                return fail("game code here starts with `do` or `set`");
            };
            if code.contains(['\n', '\r']) {
                return fail("game code here is one line");
            }
            let (pos, indent, parent) = position(&root, *at, *after, *under)?;
            place(
                text,
                pos,
                &format!("{indent}{code}"),
                newline_of(&root),
                |inserted, line| {
                    inserted.kind == MUTATION_LINE
                        && !inserted.unreadable
                        && line.text().to_string().trim().starts_with(keyword)
                },
                parent,
            )
            .map_err(|_| EditError(format!("`{code}` does not read as game code")))
        }
        Edit::InsertCondition {
            at,
            after,
            under,
            keyword,
            condition,
            line,
        } => {
            let condition = condition
                .as_deref()
                .map(str::trim)
                .filter(|c| !c.is_empty());
            let written = match (keyword.as_str(), condition) {
                ("if" | "elif", Some(c)) if !c.contains(['\n', '\r']) => format!("{keyword} {c}"),
                ("if" | "elif", _) => return fail("a condition is one line, and not empty"),
                ("else", None) => "else".to_owned(),
                ("else", Some(_)) => return fail("an `else` has no condition"),
                _ => return fail("conditions are `if`, `elif` or `else`"),
            };
            let (pos, indent, parent) = position(&root, *at, *after, *under)?;
            let applied = place(
                text,
                pos,
                &format!("{indent}{written}"),
                newline_of(&root),
                |inserted, line| {
                    // An `elif` or `else` follows an `if` or `elif` arm.
                    let follows = keyword == "if"
                        || std::iter::successors(line.prev_sibling(), SyntaxNode::prev_sibling)
                            .find(|l| !is_trivia(l.kind()))
                            .is_some_and(|l| {
                                l.kind() == CONDITION_LINE
                                    && l.children_with_tokens().all(|e| e.kind() != ELSE_KW)
                            });
                    inserted.kind == CONDITION_LINE
                        && !inserted.unreadable
                        && inserted.condition.as_deref() == condition
                        && follows
                },
                parent,
            )
            .map_err(|_| EditError(format!("`{written}` does not read as a condition here")))?;
            // Its first line, nested under it.
            let root = es_syntax::parse(&applied.text).syntax();
            let body = insert_under(&applied.text, &root, applied.at, line)?;
            Ok(Applied {
                text: body.text,
                at: applied.at,
            })
        }
        Edit::RemoveLine { at } => remove_line(text, &root, *at),
        Edit::MoveLine { at, up } => move_line(text, &root, *at, *up),
        Edit::SetJump { at, target } => set_jump(text, &root, *at, target.as_deref()),
        Edit::AddLabel { at, id } => add_label(text, &root, *at, id),
        Edit::RemoveLabel { at, from } => remove_label(text, &root, *at, *from),
    }
}

// ---------------------------------------------------------------- the edits

fn set_text(text: &str, root: &SyntaxNode, at: usize, words: &str) -> Result<Applied, EditError> {
    let line = text_line_at(root, at)?;
    let before = Summary::of(&line);
    let narration = line.kind() == DIALOGUE_LINE && before.speaker.is_none();
    let words = escape_words(words, narration)?;
    let region = words_range(&line);
    let (pre, post) = padding(&line, &region);
    // The words are not part of the summary: everything else about the line must stay.
    let same = |after: &Summary| *after == before;
    let first = format!("{pre}{words}{post}");
    let result = checked(&splice(text, region.clone(), &first), at, same);
    if result.is_ok() || !narration {
        return result.map_err(|_| refused_text());
    }
    // Narration that reads as a keyword line (`if`, `- `, `=> `...) is written escaped.
    checked(
        &splice(text, region, &format!("{pre}\\{words}{post}")),
        at,
        same,
    )
    .map_err(|_| refused_text())
}

fn refused_text() -> EditError {
    EditError(
        "this text would change more of the line than its words (a tag, a jump, the speaker)"
            .into(),
    )
}

fn set_speaker(
    text: &str,
    root: &SyntaxNode,
    at: usize,
    speaker: Option<&str>,
) -> Result<Applied, EditError> {
    let line = text_line_at(root, at)?;
    let before = Summary::of(&line);
    let name = speaker.map(str::trim).filter(|s| !s.is_empty());
    if let Some(name) = name
        && (name.contains(':') || name.contains(['[', ']', '#', '\n', '\r']))
    {
        return fail("a speaker's name cannot have `:`, `[`, `]`, `#` or a line break");
    }
    let node = line.children().find(|n| n.kind() == SPEAKER);
    let (range, with) = match (&node, name) {
        (Some(node), Some(name)) => (range_of(node), format!("{name}:")),
        (Some(node), None) => {
            let end = match node.next_sibling_or_token() {
                Some(next) if next.kind() == WHITESPACE => element_range(&next).end,
                _ => end(node),
            };
            (start(node)..end, String::new())
        }
        (None, Some(name)) => {
            let at = words_range(&line).start;
            (at..at, format!("{name}: "))
        }
        (None, None) => {
            return Ok(Applied {
                text: text.to_owned(),
                at,
            });
        }
    };
    checked(&splice(text, range, &with), at, |after| {
        after.speaker.as_deref() == name
            && Summary {
                speaker: before.speaker.clone(),
                ..after.clone()
            } == before
    })
    .map_err(|_| match name {
        None => {
            EditError("as narration, this line's text would read as someone's (it has `: `)".into())
        }
        Some(_) => EditError("this name does not read back as the speaker".into()),
    })
}

fn set_tag(
    text: &str,
    root: &SyntaxNode,
    at: usize,
    name: &str,
    value: Option<&str>,
) -> Result<Applied, EditError> {
    let line = text_line_at(root, at)?;
    let before = Summary::of(&line);
    let name = name.trim();
    let value = value.map(str::trim).filter(|v| !v.is_empty());
    if name.is_empty() || name.contains(|c: char| c.is_whitespace() || ",=[]#".contains(c)) {
        return fail(format!("`{name}` cannot be a tag's name"));
    }
    if value.is_some_and(|v| v.contains([',', ']', '[', '\n', '\r'])) {
        return fail("a tag's value cannot have `,`, `[`, `]` or a line break");
    }
    let written = match value {
        Some(v) => format!("{name}={v}"),
        None => name.to_owned(),
    };
    let groups: Vec<SyntaxNode> = line.children().filter(|n| n.kind() == TAG_GROUP).collect();
    let existing = groups
        .iter()
        .flat_map(|g| g.children())
        .find(|t| tag_name(t).as_deref() == Some(name));
    let (range, with) = if let Some(tag) = existing {
        (range_of(&tag), written.clone())
    } else if let Some(group) = groups.last() {
        // Inside the last group, before its `]`.
        let has_tags = group.children().any(|n| n.kind() == TAG);
        (
            end(group) - 1..end(group) - 1,
            if has_tags {
                format!(", {written}")
            } else {
                written.clone()
            },
        )
    } else if line.kind() == RESPONSE_LINE
        && let Some(condition) = trailing_condition(&line)
    {
        // A choice's `[if ...]` must stay last for Dialogue Manager to read it.
        (
            start(&condition)..start(&condition),
            format!("[#{written}] "),
        )
    } else {
        let at = content_end(&line);
        (at..at, format!(" [#{written}]"))
    };
    checked(&splice(text, range, &with), at, |after| {
        let mut expected = before.tags.clone();
        match expected.iter_mut().find(|(n, _)| n == name) {
            Some(tag) => tag.1 = value.map(str::to_owned),
            None => expected.push((name.to_owned(), value.map(str::to_owned))),
        }
        after.tags == expected
            && Summary {
                tags: before.tags.clone(),
                ..after.clone()
            } == before
    })
    .map_err(|_| EditError(format!("the tag `{written}` does not read back as written")))
}

fn remove_tag(text: &str, root: &SyntaxNode, at: usize, name: &str) -> Result<Applied, EditError> {
    let line = text_line_at(root, at)?;
    let before = Summary::of(&line);
    let mut ranges = Vec::new();
    for group in line.children().filter(|n| n.kind() == TAG_GROUP) {
        let tags: Vec<SyntaxNode> = group.children().filter(|n| n.kind() == TAG).collect();
        let gone: Vec<&SyntaxNode> = tags
            .iter()
            .filter(|t| tag_name(t).as_deref() == Some(name))
            .collect();
        if gone.is_empty() {
            continue;
        }
        if gone.len() == tags.len() {
            ranges.push(group_removal(&group));
            continue;
        }
        for tag in gone {
            let elements: Vec<SyntaxElement> = group.children_with_tokens().collect();
            let index = elements
                .iter()
                .position(|e| e.as_node() == Some(tag))
                .expect("a child of its group");
            let comma_before = elements[..index].iter().rposition(|e| e.kind() == COMMA);
            let range = match comma_before {
                // `a, b` → `a`: from the comma before.
                Some(comma)
                    if elements[comma + 1..index]
                        .iter()
                        .all(|e| e.kind() == WHITESPACE) =>
                {
                    element_range(&elements[comma]).start..end(tag)
                }
                // The first tag: up to the next one.
                _ => {
                    let next = elements[index + 1..]
                        .iter()
                        .find(|e| e.kind() == TAG)
                        .map(|e| element_range(e).start);
                    start(tag)..next.unwrap_or_else(|| end(tag))
                }
            };
            ranges.push(range);
        }
    }
    if ranges.is_empty() {
        return Ok(Applied {
            text: text.to_owned(),
            at,
        });
    }
    ranges.sort_by_key(|r| std::cmp::Reverse(r.start));
    let mut out = text.to_owned();
    for range in ranges {
        out.replace_range(range, "");
    }
    checked(&out, at, |after| {
        let expected: Vec<_> = before
            .tags
            .iter()
            .filter(|(n, _)| n != name)
            .cloned()
            .collect();
        after.tags == expected
            && Summary {
                tags: before.tags.clone(),
                ..after.clone()
            } == before
    })
    .map_err(|_| EditError(format!("removing `{name}` would change more of the line")))
}

/// A whole tag group, with the space that separates it from the text.
fn group_removal(group: &SyntaxNode) -> Range<usize> {
    let previous = group
        .prev_sibling_or_token()
        .filter(|e| e.kind() == WHITESPACE);
    let next = group.next_sibling_or_token();
    let at_end = next
        .as_ref()
        .is_none_or(|e| matches!(e.kind(), NEWLINE | WHITESPACE | BLOCK));
    match (previous, next) {
        (Some(previous), _) if at_end => element_range(&previous).start..end(group),
        (_, Some(next)) if next.kind() == WHITESPACE && !at_end => {
            start(group)..element_range(&next).end
        }
        (Some(previous), _) => element_range(&previous).start..end(group),
        _ => range_of(group),
    }
}

fn set_condition(
    text: &str,
    root: &SyntaxNode,
    at: usize,
    condition: Option<&str>,
) -> Result<Applied, EditError> {
    let line = line_at(root, at)?;
    let before = Summary::of(&line);
    let condition = condition.map(str::trim).filter(|c| !c.is_empty());
    if condition.is_some_and(|c| c.contains(['\n', '\r'])) {
        return fail("a condition is one line");
    }
    let (range, with) = match line.kind() {
        CONDITION_LINE | WHILE_LINE | MATCH_LINE | WHEN_LINE => {
            let Some(condition) = condition else {
                return fail("this line needs a condition; remove the line instead");
            };
            match line.children().find(|n| n.kind() == EXPR) {
                Some(expr) => (range_of(&expr), condition.to_owned()),
                None => {
                    let keyword = line
                        .children_with_tokens()
                        .filter(|e| {
                            matches!(e.kind(), IF_KW | ELIF_KW | WHILE_KW | MATCH_KW | WHEN_KW)
                        })
                        .last();
                    let Some(keyword) = keyword else {
                        return fail("an `else` has no condition");
                    };
                    let at = element_range(&keyword).end;
                    (at..at, format!(" {condition}"))
                }
            }
        }
        RESPONSE_LINE => match (
            trailing_condition(&line).or_else(|| first_child(&line, INLINE_CONDITION)),
            condition,
        ) {
            (Some(node), Some(condition)) => (range_of(&node), format!("[if {condition}]")),
            (Some(node), None) => {
                let start = match node.prev_sibling_or_token() {
                    Some(previous) if previous.kind() == WHITESPACE => {
                        element_range(&previous).start
                    }
                    _ => start(&node),
                };
                (start..end(&node), String::new())
            }
            (None, Some(condition)) => {
                let at = content_end(&line);
                (at..at, format!(" [if {condition}]"))
            }
            (None, None) => {
                return Ok(Applied {
                    text: text.to_owned(),
                    at,
                });
            }
        },
        _ => return fail("this line has no condition"),
    };
    checked(&splice(text, range, &with), at, |after| {
        after.condition.as_deref() == condition
            && !after.unreadable
            && Summary {
                condition: before.condition.clone(),
                ..after.clone()
            } == before
    })
    .map_err(|_| {
        EditError(format!(
            "`{}` does not read as a condition",
            condition.unwrap_or_default()
        ))
    })
}

fn insert_line(
    text: &str,
    root: &SyntaxNode,
    at: usize,
    after: bool,
    new: &NewLine,
) -> Result<Applied, EditError> {
    let (pos, indent, parent) = position(root, at, after, false)?;
    insert_at(text, root, pos, &indent, new, parent)
}

/// Where a new line goes, and its indentation: before or after the line at `at` (with
/// what is nested under it), or under it, after the lines nested there. The third value
/// is the line it must end up under, when it goes under one.
fn position(
    root: &SyntaxNode,
    at: usize,
    after: bool,
    under: bool,
) -> Result<(usize, String, Option<usize>), EditError> {
    let line = line_at(root, at)?;
    if !under {
        let indent = first_token(&line, INDENT).unwrap_or_default();
        let pos = if after { end(&line) } else { unit_start(&line) };
        return Ok((pos, indent, None));
    }
    if !matches!(
        line.kind(),
        RESPONSE_LINE | CONDITION_LINE | WHEN_LINE | WHILE_LINE
    ) {
        return fail("lines go under a choice or a condition only");
    }
    let nested = first_child(&line, BLOCK).and_then(|block| {
        block
            .children()
            .filter(|n| n.kind().is_line() && !is_trivia(n.kind()))
            .last()
    });
    let indent = match &nested {
        Some(last) => first_token(last, INDENT).unwrap_or_default(),
        None => format!(
            "{}{}",
            first_token(&line, INDENT).unwrap_or_default(),
            indent_unit(root)
        ),
    };
    let pos = nested.as_ref().map_or_else(|| end(&line), end);
    Ok((pos, indent, Some(at)))
}

/// A new line nested under a choice or a condition, after the lines there.
fn insert_under(
    text: &str,
    root: &SyntaxNode,
    at: usize,
    new: &NewLine,
) -> Result<Applied, EditError> {
    let (pos, indent, parent) = position(root, at, true, true)?;
    insert_at(text, root, pos, &indent, new, parent)
}

/// The indentation a file nests lines with: a tab, or its narrowest run of spaces.
fn indent_unit(root: &SyntaxNode) -> String {
    let indents: Vec<String> = root
        .descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|t| t.kind() == INDENT && !t.text().is_empty())
        .map(|t| t.text().to_owned())
        .collect();
    if indents.is_empty() || indents.iter().any(|i| i.starts_with('\t')) {
        return "\t".into();
    }
    let narrowest = indents.iter().map(String::len).min().unwrap_or(4);
    " ".repeat(narrowest)
}

/// Write a new line at `pos` (the start of a line, or the end of the file) with `indent`;
/// under the line starting at `parent` when there is one.
fn insert_at(
    text: &str,
    root: &SyntaxNode,
    pos: usize,
    indent: &str,
    new: &NewLine,
    parent: Option<usize>,
) -> Result<Applied, EditError> {
    let newline = newline_of(root);
    let speaker = new
        .speaker
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if speaker.is_some_and(|s| s.contains(':') || s.contains(['[', ']', '#', '\n', '\r'])) {
        return fail("a speaker's name cannot have `:`, `[`, `]`, `#` or a line break");
    }
    // A line that only jumps.
    if !new.choice && new.text.trim().is_empty() {
        let Some(target) = new.jump.as_deref() else {
            return fail("a line needs some words");
        };
        let target = jump_target(target)?;
        let content = format!("{indent}=> {target}");
        return place(
            text,
            pos,
            &content,
            newline,
            |inserted, _| {
                inserted.kind == GOTO_LINE
                    && inserted.jump.as_deref() == Some(&*format!("=> {target}"))
            },
            parent,
        )
        .map_err(|_| EditError("the jump does not read back as written".into()));
    }
    let narration = !new.choice && speaker.is_none();
    let words = escape_words(&new.text, narration)?;
    let kind = if new.choice {
        RESPONSE_LINE
    } else {
        DIALOGUE_LINE
    };
    let content = |words: &str| {
        let dash = if new.choice { "- " } else { "" };
        let name = speaker.map(|s| format!("{s}: ")).unwrap_or_default();
        format!("{indent}{dash}{name}{words}")
    };
    let attempt = |words: &str| {
        place(
            text,
            pos,
            &content(words),
            newline,
            |inserted, _| inserted.kind == kind && inserted.speaker.as_deref() == speaker,
            parent,
        )
    };
    match attempt(&words) {
        Ok(applied) => Ok(applied),
        Err(_) if narration => attempt(&format!("\\{words}")),
        Err(e) => Err(e),
    }
    .map_err(|_| EditError("the new line does not read back as written".into()))
}

/// Put `content` as a line at `pos` and check it; under `parent` when given.
fn place(
    text: &str,
    pos: usize,
    content: &str,
    newline: &str,
    ok: impl Fn(&Summary, &SyntaxNode) -> bool,
    parent: Option<usize>,
) -> Result<Applied, EditError> {
    let (out, at) = if pos == 0 || text[..pos].ends_with('\n') {
        (splice(text, pos..pos, &format!("{content}{newline}")), pos)
    } else {
        // The last line, with no newline after it: the file keeps ending without one.
        (
            splice(text, pos..pos, &format!("{newline}{content}")),
            pos + newline.len(),
        )
    };
    let root = es_syntax::parse(&out).syntax();
    let line = line_at(&root, at)?;
    let under = |line: &SyntaxNode| {
        line.parent()
            .filter(|b| b.kind() == BLOCK)
            .and_then(|b| b.parent())
            .map(|owner| start(&owner))
    };
    if ok(&Summary::of(&line), &line) && parent.is_none_or(|p| under(&line) == Some(p)) {
        Ok(Applied { text: out, at })
    } else {
        fail("the new line does not read back as written")
    }
}

/// A jump's target as written after `=>`: a title, `END` or `END!`.
fn jump_target(target: &str) -> Result<&str, EditError> {
    let target = target.trim();
    if target == "END" || target == "END!" || is_title(target) {
        Ok(target)
    } else {
        fail(format!("`{target}` is not a title to go to"))
    }
}

/// A title Dialogue Manager accepts: letters, digits and `_`, not starting with a digit.
pub fn is_title(id: &str) -> bool {
    id.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn set_jump(
    text: &str,
    root: &SyntaxNode,
    at: usize,
    target: Option<&str>,
) -> Result<Applied, EditError> {
    let line = line_at(root, at)?;
    let target = target.map(str::trim).filter(|t| !t.is_empty());
    if line.kind() == GOTO_LINE {
        let Some(target) = target else {
            return fail("this line only jumps; remove the line instead");
        };
        let target = jump_target(target)?;
        let Some(jump) = first_child(&line, JUMP) else {
            return fail("this line has no jump to change");
        };
        let written = format!("=> {target}");
        return checked(&splice(text, range_of(&jump), &written), at, |after| {
            after.jump.as_deref() == Some(&*written)
        })
        .map_err(|_| EditError(format!("`{written}` does not read back as a jump")));
    }
    let line = text_line_at(root, at)?;
    let before = Summary::of(&line);
    let target = target.map(jump_target).transpose()?;
    let written = target.map(|t| format!("=> {t}"));
    let same = |after: &Summary| {
        after.jump == written
            && Summary {
                jump: before.jump.clone(),
                ..after.clone()
            } == before
    };
    let jump = line.children().find(|n| n.kind() == JUMP);
    let attempts: Vec<(Range<usize>, String)> = match (&jump, &written) {
        (Some(jump), Some(written)) => vec![(range_of(jump), written.clone())],
        (Some(jump), None) => {
            let from = match jump.prev_sibling_or_token() {
                Some(previous) if previous.kind() == WHITESPACE => element_range(&previous).start,
                _ => start(jump),
            };
            vec![(from..end(jump), String::new())]
        }
        (None, Some(written)) => {
            let mut places = Vec::new();
            if line.kind() == RESPONSE_LINE
                && let Some(condition) = trailing_condition(&line)
            {
                places.push(start(&condition));
            }
            places.push(content_end(&line));
            let words = words_range(&line);
            places.push(words.end);
            places
                .into_iter()
                .map(|at| {
                    let spaced = if text[..at].ends_with([' ', '\t']) {
                        format!("{written} ")
                    } else {
                        format!(" {written}")
                    };
                    (at..at, spaced)
                })
                .collect()
        }
        (None, None) => {
            return Ok(Applied {
                text: text.to_owned(),
                at,
            });
        }
    };
    for (range, with) in attempts {
        if let Ok(applied) = checked(&splice(text, range, &with), at, same) {
            return Ok(applied);
        }
    }
    fail(match written {
        Some(written) => format!("`{written}` does not read back as this line's jump"),
        None => "removing the jump would change more of the line".into(),
    })
}

fn add_label(
    text: &str,
    root: &SyntaxNode,
    at: Option<usize>,
    id: &str,
) -> Result<Applied, EditError> {
    let id = id.trim();
    if !is_title(id) {
        return fail(format!(
            "`{id}` cannot be a title: letters, digits and `_`, not starting with a digit"
        ));
    }
    let titles: Vec<SyntaxNode> = root.children().filter(|n| n.kind() == TITLE_LINE).collect();
    if titles.iter().any(|t| title_name(t).as_deref() == Some(id)) {
        return fail(format!("there is a label `{id}` in this file already"));
    }
    let newline = newline_of(root);
    let content = format!("~ {id}{newline}=> END{newline}");
    let next = match at {
        Some(at) => {
            let title = line_at(root, at)?;
            if title.kind() != TITLE_LINE || title.parent().as_ref() != Some(root) {
                return fail("new labels go after a label's title");
            }
            titles.iter().find(|t| start(t) > at).cloned()
        }
        None => None,
    };
    let (out, new_at) = match next {
        // Before the next label and the comments above it, with a blank line after.
        Some(next) => {
            let pos = unit_start(&next);
            (splice(text, pos..pos, &format!("{content}{newline}")), pos)
        }
        // At the end, after a blank line.
        None => {
            let mut lead = String::new();
            if !text.is_empty() && !text.ends_with('\n') {
                lead.push_str(newline);
            }
            let ended = format!("{text}{lead}");
            if !ended.is_empty()
                && !ended.ends_with(&format!("{newline}{newline}"))
                && ended != newline
            {
                lead.push_str(newline);
            }
            let pos = text.len() + lead.len();
            (format!("{text}{lead}{content}"), pos)
        }
    };
    let new_root = es_syntax::parse(&out).syntax();
    let title = line_at(&new_root, new_at)?;
    if title.kind() == TITLE_LINE && title_name(&title).as_deref() == Some(id) {
        Ok(Applied {
            text: out,
            at: new_at,
        })
    } else {
        fail("the new label does not read back as written")
    }
}

/// The labels a file has, by title.
pub fn titles(text: &str) -> Vec<String> {
    es_syntax::parse(text)
        .syntax()
        .children()
        .filter(|n| n.kind() == TITLE_LINE)
        .filter_map(|t| title_name(&t))
        .collect()
}

fn title_name(title: &SyntaxNode) -> Option<String> {
    ast::TitleLine::cast(title.clone())?
        .name()
        .map(|t| t.text().to_owned())
}

fn remove_label(
    text: &str,
    root: &SyntaxNode,
    at: usize,
    from: Option<usize>,
) -> Result<Applied, EditError> {
    let title = line_at(root, at)?;
    if title.kind() != TITLE_LINE || title.parent().as_ref() != Some(root) {
        return fail("no label's title starts there");
    }
    let lines: Vec<SyntaxNode> = root.children().filter(|n| n.kind().is_line()).collect();
    let index = lines
        .iter()
        .position(|l| *l == title)
        .expect("a line of the file");
    // Its lines: up to the next title, without the comments and blanks at the end (they
    // are about the next label).
    let next = lines[index + 1..]
        .iter()
        .position(|l| l.kind() == TITLE_LINE)
        .map_or(lines.len(), |i| index + 1 + i);
    let last = lines[index..next]
        .iter()
        .rposition(|l| !is_trivia(l.kind()))
        .map(|i| index + i)
        .expect("the title itself");
    let mut range = start(&title)..end(&lines[last]);
    if let Some(from) = from {
        // Only whole comment and blank lines between the notes and the title.
        let above = lines[..index]
            .iter()
            .rev()
            .take_while(|l| start(l) >= from)
            .collect::<Vec<_>>();
        if from > at
            || above.iter().any(|l| !is_trivia(l.kind()))
            || above.last().is_none_or(|l| start(l) != from)
        {
            return fail("the label's notes are not right above its title");
        }
        range.start = from;
    }
    // The blank lines that separated it go too, on one side.
    let after_blank = lines[last + 1..next]
        .iter()
        .take_while(|l| l.kind() == BLANK_LINE)
        .last()
        .map(end);
    if lines[last + 1..].iter().all(|l| l.kind() == BLANK_LINE) {
        // The last thing in the file: the blanks before it go, and the file still ends with
        // a newline.
        let before = lines[..index]
            .iter()
            .rev()
            .skip_while(|l| start(l) >= range.start)
            .take_while(|l| l.kind() == BLANK_LINE)
            .last()
            .map(start);
        if let Some(before) = before {
            range.start = before;
        }
        let mut out = text[..range.start].to_owned();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(newline_of(root));
        }
        let at = range.start.min(out.len());
        return Ok(Applied { text: out, at });
    }
    if let Some(blank_end) = after_blank {
        range.end = blank_end;
    }
    let out = splice(text, range.clone(), "");
    Ok(Applied {
        text: out,
        at: range.start,
    })
}

fn remove_line(text: &str, root: &SyntaxNode, at: usize) -> Result<Applied, EditError> {
    let line = line_at(root, at)?;
    if line.kind() == TITLE_LINE {
        return fail("a label's title is not removed this way");
    }
    if let Some(owner) = line
        .parent()
        .filter(|p| p.kind() == BLOCK)
        .and_then(|b| b.parent())
        && matches!(
            owner.kind(),
            CONDITION_LINE | WHILE_LINE | MATCH_LINE | WHEN_LINE
        )
    {
        let others = line
            .parent()
            .into_iter()
            .flat_map(|b| b.children())
            .filter(|l| *l != line && !is_trivia(l.kind()))
            .count();
        if others == 0 {
            return fail("this is the only line under its condition; remove the condition instead");
        }
    }
    let range = unit_start(&line)..end(&line);
    let at = range.start;
    let out = splice(text, range, "");
    Ok(Applied { text: out, at })
}

fn move_line(text: &str, root: &SyntaxNode, at: usize, up: bool) -> Result<Applied, EditError> {
    let line = line_at(root, at)?;
    if line.kind() == TITLE_LINE {
        return fail("a label's title is not moved this way");
    }
    if is_trivia(line.kind()) {
        return fail("comments move with the line below them");
    }
    let siblings: Vec<SyntaxNode> = line
        .parent()
        .into_iter()
        .flat_map(|p| p.children())
        .filter(|n| n.kind().is_line() && !is_trivia(n.kind()))
        .collect();
    let index = siblings
        .iter()
        .position(|n| *n == line)
        .expect("a child of its parent");
    let other = if up {
        index.checked_sub(1)
    } else {
        Some(index + 1)
    };
    let Some(other) = other.and_then(|i| siblings.get(i)) else {
        return fail(if up {
            "already the first line here"
        } else {
            "already the last line here"
        });
    };
    if other.kind() == TITLE_LINE {
        return fail("lines do not move across a label's title");
    }
    let (first, second) = if up { (other, &line) } else { (&line, other) };
    let first_range = unit_start(first)..end(first);
    let second_range = unit_start(second)..end(second);
    let between = &text[first_range.end..second_range.start];
    let mut later = text[second_range.clone()].to_owned();
    let mut earlier = text[first_range.clone()].to_owned();
    let newline = newline_of(root);
    if !later.ends_with('\n') {
        // The later unit ends the file without a newline; after the swap the other one does.
        later.push_str(newline);
        if let Some(trimmed) = earlier.strip_suffix(newline) {
            earlier = trimmed.to_owned();
        }
    }
    let offset_in_unit = at - unit_start(&line);
    let new_at = if up {
        first_range.start + offset_in_unit
    } else {
        first_range.start + later.len() + between.len() + offset_in_unit
    };
    let out = splice(
        text,
        first_range.start..second_range.end,
        &format!("{later}{between}{earlier}"),
    );
    let kind = line.kind();
    checked(&out, new_at, |moved| moved.kind == kind)
        .map_err(|_| EditError("the line does not move cleanly here".into()))
}

// ---------------------------------------------------------------- finding things

fn line_at(root: &SyntaxNode, at: usize) -> Result<SyntaxNode, EditError> {
    root.descendants()
        .find(|n| n.kind().is_line() && start(n) == at)
        .ok_or_else(|| {
            EditError(format!(
                "no line starts at byte {at}; the file may have changed"
            ))
        })
}

fn text_line_at(root: &SyntaxNode, at: usize) -> Result<SyntaxNode, EditError> {
    let line = line_at(root, at)?;
    match line.kind() {
        DIALOGUE_LINE | RESPONSE_LINE => Ok(line),
        _ => fail("only a line of dialogue or a choice has text, a speaker and tags"),
    }
}

fn start(node: &SyntaxNode) -> usize {
    node.text_range().start().into()
}

fn end(node: &SyntaxNode) -> usize {
    node.text_range().end().into()
}

fn range_of(node: &SyntaxNode) -> Range<usize> {
    start(node)..end(node)
}

fn element_range(element: &SyntaxElement) -> Range<usize> {
    let range = element.text_range();
    range.start().into()..range.end().into()
}

fn first_child(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    node.children().find(|n| n.kind() == kind)
}

fn first_token(node: &SyntaxNode, kind: SyntaxKind) -> Option<String> {
    node.children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|t| t.kind() == kind)
        .map(|t| t.text().to_owned())
}

fn tag_name(tag: &SyntaxNode) -> Option<String> {
    ast::Tag::cast(tag.clone())?
        .name()
        .map(|t| t.text().to_owned())
}

fn is_trivia(kind: SyntaxKind) -> bool {
    matches!(kind, BLANK_LINE | COMMENT_LINE | IMPORT_LINE | USING_LINE)
}

/// The line's own elements: without the lines nested under it.
fn own(line: &SyntaxNode) -> Vec<SyntaxElement> {
    line.children_with_tokens()
        .filter(|e| e.kind() != BLOCK)
        .collect()
}

/// Where the line's content ends: before trailing blanks and the newline.
fn content_end(line: &SyntaxNode) -> usize {
    own(line)
        .iter()
        .rev()
        .find(|e| !matches!(e.kind(), NEWLINE | WHITESPACE))
        .map_or(start(line), |e| element_range(e).end)
}

/// A choice's `[if ...]` when it is the last thing on the line.
fn trailing_condition(line: &SyntaxNode) -> Option<SyntaxNode> {
    own(line)
        .into_iter()
        .rev()
        .find(|e| !matches!(e.kind(), NEWLINE | WHITESPACE))
        .and_then(SyntaxElement::into_node)
        .filter(|n| n.kind() == INLINE_CONDITION)
}

/// Where the words of a dialogue line or choice are: after its markers and speaker, before
/// the tags, id, jump (and a choice's condition) at its end. Empty when it has none.
fn words_range(line: &SyntaxNode) -> Range<usize> {
    let elements = own(line);
    let leading = |e: &SyntaxElement| {
        matches!(
            e.kind(),
            INDENT | WHITESPACE | RANDOM_PREFIX | DASH | PIPE | SPEAKER
        ) || (e.kind() == ESCAPE && e.as_token().is_some_and(|t| t.text() == "\\"))
    };
    let trailing = |e: &SyntaxElement| {
        matches!(
            e.kind(),
            NEWLINE | WHITESPACE | TAG_GROUP | STATIC_ID | JUMP
        ) || (line.kind() == RESPONSE_LINE && e.kind() == INLINE_CONDITION)
    };
    let first = elements
        .iter()
        .position(|e| !leading(e))
        .unwrap_or(elements.len());
    let last = elements
        .iter()
        .rposition(|e| !trailing(e))
        .map_or(first, |i| (i + 1).max(first));
    if first < last {
        element_range(&elements[first]).start..element_range(&elements[last - 1]).end
    } else {
        let at = first
            .checked_sub(1)
            .map_or_else(|| start(line), |i| element_range(&elements[i]).end);
        at..at
    }
}

/// Spaces to put around words written where there were none.
fn padding(line: &SyntaxNode, region: &Range<usize>) -> (&'static str, &'static str) {
    if !region.is_empty() {
        return ("", "");
    }
    let elements = own(line);
    let previous = elements
        .iter()
        .rfind(|e| element_range(e).end <= region.start)
        .map(SyntaxElement::kind);
    let next = elements
        .iter()
        .find(|e| element_range(e).start >= region.end)
        .map(SyntaxElement::kind);
    let pre = if matches!(previous, Some(SPEAKER | DASH | PIPE)) {
        " "
    } else {
        ""
    };
    let post = if matches!(next, Some(TAG_GROUP | STATIC_ID | JUMP | INLINE_CONDITION)) {
        " "
    } else {
        ""
    };
    (pre, post)
}

/// Where a line starts together with the comments written right above it.
fn unit_start(line: &SyntaxNode) -> usize {
    let mut at = start(line);
    let mut previous = line.prev_sibling();
    while let Some(node) = previous {
        if node.kind() != COMMENT_LINE {
            break;
        }
        at = start(&node);
        previous = node.prev_sibling();
    }
    at
}

fn newline_of(root: &SyntaxNode) -> &'static str {
    let crlf = root
        .descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .find(|t| t.kind() == NEWLINE)
        .is_some_and(|t| t.text() == "\r\n");
    if crlf { "\r\n" } else { "\n" }
}

fn splice(text: &str, range: Range<usize>, with: &str) -> String {
    let mut out = String::with_capacity(text.len() + with.len());
    out.push_str(&text[..range.start]);
    out.push_str(with);
    out.push_str(&text[range.end..]);
    out
}

/// Words as they go into a file: one line, `\n` for a line break, a `[` that would open a
/// tag or id escaped, and for narration a `: ` that would read as a speaker escaped.
///
/// The words are what is in the file (escapes included, as the editor shows them), so
/// only what is not escaped yet gets escaped.
fn escape_words(words: &str, narration: bool) -> Result<String, EditError> {
    let mut out = words.trim().replace("\r\n", "\n").replace('\n', "\\n");
    out = escape_unescaped(&out, "[#");
    out = escape_unescaped(&out, "[ID:");
    if narration {
        out = escape_unescaped(&out, ": ");
    }
    if out.is_empty() {
        return fail("a line needs some words");
    }
    if out.contains(" => ") || out.contains(" =>< ") {
        return fail("`=>` in the words would read as a jump");
    }
    Ok(out)
}

/// Put a `\` before each `pattern` that has none yet.
fn escape_unescaped(text: &str, pattern: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(pattern) {
        out.push_str(&rest[..at]);
        if !out.ends_with('\\') {
            out.push('\\');
        }
        out.push_str(pattern);
        rest = &rest[at + pattern.len()..];
    }
    out.push_str(rest);
    out
}

/// The words of a dialogue line or choice as they are in the file.
pub(crate) fn words_source(line: &SyntaxNode) -> String {
    let range = words_range(line);
    let offset = start(line);
    let text = line.text().to_string();
    text[range.start - offset..range.end - offset].to_owned()
}

// ---------------------------------------------------------------- checking

/// What an edit must keep, or change, about a line.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Summary {
    kind: SyntaxKind,
    speaker: Option<String>,
    tags: Vec<(String, Option<String>)>,
    jump: Option<String>,
    id: Option<String>,
    condition: Option<String>,
    /// Unreadable text on the line.
    unreadable: bool,
}

impl Summary {
    fn of(line: &SyntaxNode) -> Self {
        let mut summary = Summary {
            kind: line.kind(),
            speaker: None,
            tags: Vec::new(),
            jump: None,
            id: None,
            condition: None,
            unreadable: own(line).iter().any(|e| match e {
                SyntaxElement::Token(t) => t.kind() == ERROR,
                SyntaxElement::Node(n) => n.descendants_with_tokens().any(|d| d.kind() == ERROR),
            }),
        };
        let tags = |tags: &mut dyn Iterator<Item = ast::Tag>| {
            tags.filter_map(|t| {
                Some((
                    t.name()?.text().to_owned(),
                    t.value().map(|v| v.text().to_owned()),
                ))
            })
            .collect()
        };
        let expr = |e: Option<ast::Expr>| e.map(|e| e.syntax().text().to_string());
        match AstLine::cast(line.clone()) {
            Some(AstLine::Dialogue(d)) => {
                summary.speaker = d.speaker().map(|s| s.name());
                summary.tags = tags(&mut d.tags());
                summary.jump = d.jump().map(|j| j.syntax().text().to_string());
                summary.id = d.static_id().map(|i| i.syntax().text().to_string());
            }
            Some(AstLine::Response(r)) => {
                summary.speaker = r.speaker().map(|s| s.name());
                summary.tags = tags(&mut r.tags());
                summary.jump = r.jump().map(|j| j.syntax().text().to_string());
                summary.id = r.static_id().map(|i| i.syntax().text().to_string());
                summary.condition = r.condition().and_then(|c| expr(c.expr()));
            }
            Some(AstLine::Goto(g)) => {
                summary.jump = g.jump().map(|j| j.syntax().text().to_string())
            }
            Some(AstLine::Condition(c)) => summary.condition = expr(c.expr()),
            Some(AstLine::While(w)) => summary.condition = expr(w.expr()),
            Some(AstLine::Match(m)) => summary.condition = expr(m.expr()),
            Some(AstLine::When(w)) => summary.condition = expr(w.expr()),
            _ => {}
        }
        summary
    }
}

/// Parse the edited file and check the line at `at` with `ok`.
fn checked(text: &str, at: usize, ok: impl Fn(&Summary) -> bool) -> Result<Applied, EditError> {
    let root = es_syntax::parse(text).syntax();
    let line = line_at(&root, at)?;
    if ok(&Summary::of(&line)) {
        Ok(Applied {
            text: text.to_owned(),
            at,
        })
    } else {
        fail("the edit does not read back as asked")
    }
}
