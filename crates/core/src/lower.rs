//! From Dialogue Manager syntax trees to the model.
//!
//! Comments attach to what follows them: before a label, to the label (or to the section,
//! when the section has no label yet); right under a title, to the label; in a body, to
//! the next line. Comments at the end of a label belong to the next one. A comment with a
//! marker from the project file becomes a [`Mark`] instead of a [`Note`].

use std::mem;

use dm_syntax::SyntaxKind::*;
use dm_syntax::ast::{self, Line as AstLine};
use dm_syntax::{AstNode, SyntaxElement, SyntaxNode, SyntaxToken};

use crate::config::{Compiled, Config};
use crate::model::*;
use crate::project::Project;

/// Build the script model of a whole project.
pub fn lower(project: &Project) -> Script {
    let mut script = Script::default();
    for (id, file) in project.files.iter().enumerate() {
        let mut lowerer = Lowerer {
            file: id,
            config: &project.config,
            compiled: &project.compiled,
            flags: Vec::new(),
            label: None,
        };
        script
            .files
            .push(lowerer.file(&file.path, &file.parse.tree()));
        script.flags.append(&mut lowerer.flags);
    }
    assign_places(&mut script, &project.config);
    script
}

struct Lowerer<'a> {
    file: FileId,
    config: &'a Config,
    compiled: &'a Compiled,
    flags: Vec<FlagRef>,
    /// The label being lowered.
    label: Option<String>,
}

/// Notes and marks waiting for what comes next.
#[derive(Default)]
struct Pending {
    notes: Vec<Note>,
    marks: Vec<Mark>,
}

impl Pending {
    fn take(&mut self) -> (Vec<Note>, Vec<Mark>) {
        (mem::take(&mut self.notes), mem::take(&mut self.marks))
    }

    fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.marks.is_empty()
    }
}

/// A label being collected: its title line, what was said about it, its lines.
struct OpenLabel {
    title: ast::TitleLine,
    pending: Pending,
    body: Vec<AstLine>,
}

impl Lowerer<'_> {
    fn span(&self, node: &SyntaxNode) -> Span {
        let range = node.text_range();
        Span {
            file: self.file,
            start: range.start().into(),
            end: range.end().into(),
        }
    }

    fn file(&mut self, path: &str, tree: &ast::SourceFile) -> ScriptFile {
        let mut out = ScriptFile {
            path: path.to_owned(),
            ..ScriptFile::default()
        };
        let mut pending = Pending::default();
        let mut open: Option<OpenLabel> = None;
        let mut preamble = Vec::new();

        for line in tree.lines() {
            match &line {
                AstLine::Import(import) => {
                    if let (Some(path), Some(alias)) = (import.path(), import.alias()) {
                        out.imports.push(Import {
                            path: unquote(path.text()).to_owned(),
                            alias: alias.text().to_owned(),
                            span: self.span(import.syntax()),
                        });
                    }
                }
                AstLine::Comment(comment) if self.section_title(comment).is_some() => {
                    self.close(&mut out, &mut open, &mut pending);
                    let title = self.section_title(comment);
                    let (notes, _) = pending.take();
                    out.sections.push(Section {
                        title,
                        notes,
                        labels: Vec::new(),
                    });
                }
                AstLine::Title(title) => {
                    self.close(&mut out, &mut open, &mut pending);
                    let section = current_section(&mut out);
                    if section.labels.is_empty() && !pending.notes.is_empty() {
                        section.notes.append(&mut pending.notes);
                    }
                    open = Some(OpenLabel {
                        title: title.clone(),
                        pending: mem::take(&mut pending),
                        body: Vec::new(),
                    });
                }
                _ => match &mut open {
                    Some(label) => label.body.push(line),
                    None => match &line {
                        AstLine::Comment(comment) => self.comment(comment, &mut pending),
                        _ => preamble.push(line),
                    },
                },
            }
        }
        self.close(&mut out, &mut open, &mut pending);
        out.preamble = self.block(&preamble);
        out.notes.append(&mut pending.notes);
        out
    }

    fn section_title(&self, comment: &ast::CommentLine) -> Option<String> {
        self.compiled.section_title(comment.comment()?.text())
    }

    /// Finish the open label; its trailing comments wait for the next one.
    fn close(&mut self, out: &mut ScriptFile, open: &mut Option<OpenLabel>, pending: &mut Pending) {
        let Some(OpenLabel {
            title,
            pending: mut said,
            mut body,
        }) = open.take()
        else {
            return;
        };
        let mut trail = Vec::new();
        while let Some(last) = body.pop_if(|l| matches!(l, AstLine::Comment(_) | AstLine::Blank(_)))
        {
            trail.push(last);
        }
        for line in trail.iter().rev() {
            if let AstLine::Comment(comment) = line {
                self.comment(comment, pending);
            }
        }
        // Comments right under the title are about the label.
        let lead = body
            .iter()
            .take_while(|l| matches!(l, AstLine::Comment(_) | AstLine::Blank(_)))
            .count();
        for line in &body[..lead] {
            if let AstLine::Comment(comment) = line {
                self.comment(comment, &mut said);
            }
        }
        let id = title
            .name()
            .map(|t| t.text().to_owned())
            .unwrap_or_default();
        self.label = Some(id.clone());
        let body = self.block(&body[lead..]);
        self.label = None;
        let (notes, marks) = said.take();
        current_section(out).labels.push(Label {
            id,
            span: self.span(title.syntax()),
            place: None,
            notes,
            marks,
            body,
        });
    }

    fn comment(&self, comment: &ast::CommentLine, pending: &mut Pending) {
        let Some(token) = comment.comment() else {
            return;
        };
        let text = token.text().trim_start_matches('#').trim();
        let span = self.span(comment.syntax());
        match self
            .config
            .marks
            .iter()
            .find_map(|m| text.find(&m.marker).map(|at| (m, at)))
        {
            Some((spec, at)) => {
                let rest = text[at + spec.marker.len()..].trim();
                let rest = rest
                    .strip_prefix('(')
                    .and_then(|r| r.strip_suffix(')'))
                    .unwrap_or(rest)
                    .trim();
                pending.marks.push(Mark {
                    who: spec.who.clone(),
                    note: (!rest.is_empty()).then(|| rest.to_owned()),
                    span,
                });
            }
            None => pending.notes.push(Note {
                text: text.to_owned(),
                span,
            }),
        }
    }

    /// Lines at one level, grouped into nodes.
    fn block(&mut self, lines: &[AstLine]) -> Vec<Node> {
        let mut nodes = Vec::new();
        let mut pending = Pending::default();
        let mut i = 0;
        while i < lines.len() {
            let line = &lines[i];
            match line {
                AstLine::Comment(comment) => {
                    self.comment(comment, &mut pending);
                    i += 1;
                    continue;
                }
                AstLine::Blank(_) | AstLine::Import(_) | AstLine::Using(_) => {
                    i += 1;
                    continue;
                }
                _ => {}
            }
            let (kind, next, extra) = if is_random(line) {
                let (kind, next) = self.random(lines, i);
                (kind, next, Vec::new())
            } else {
                self.node(lines, i, &mut pending)
            };
            let start = self.span(line.syntax()).start;
            let end = self.span(lines[next - 1].syntax()).end;
            let (notes, marks) = pending.take();
            nodes.push(Node {
                span: Span {
                    file: self.file,
                    start,
                    end,
                },
                notes,
                marks,
                kind,
            });
            nodes.extend(extra);
            i = next;
        }
        if !pending.is_empty()
            && let Some(last) = nodes.last_mut()
        {
            last.notes.append(&mut pending.notes);
            last.marks.append(&mut pending.marks);
        }
        nodes
    }

    /// The node starting at `lines[i]`: its kind, the index after it, and any nodes that
    /// were nested under a line but are not part of it.
    fn node(
        &mut self,
        lines: &[AstLine],
        i: usize,
        pending: &mut Pending,
    ) -> (NodeKind, usize, Vec<Node>) {
        let one = |kind| (kind, i + 1, Vec::new());
        match &lines[i] {
            AstLine::Response(_) => {
                let (kind, next) = self.menu(lines, i, pending);
                (kind, next, Vec::new())
            }
            AstLine::Condition(_) => {
                let (kind, next) = self.branch(lines, i);
                (kind, next, Vec::new())
            }
            AstLine::When(when) => {
                let arm = self.arm(
                    ArmKeyword::When,
                    when.expr(),
                    when.syntax(),
                    Pending::default(),
                );
                one(NodeKind::Branch { arms: vec![arm] })
            }
            AstLine::Match(m) => one(self.match_(m)),
            AstLine::While(w) => {
                let condition = self
                    .expr(w.expr(), Some(FlagAccess::Read))
                    .unwrap_or_default();
                let body = self.children(w.syntax());
                one(NodeKind::While { condition, body })
            }
            AstLine::Mutation(m) => {
                let keyword = m.keyword().map(|k| k.text().to_owned()).unwrap_or_default();
                let expr = self
                    .expr(m.expr(), Some(FlagAccess::Set))
                    .unwrap_or_default();
                one(NodeKind::Effect(Effect { keyword, expr }))
            }
            AstLine::Goto(g) => match g.jump() {
                Some(jump) => one(NodeKind::Jump(self.jump(&jump))),
                None => one(NodeKind::Anchor { id: String::new() }),
            },
            AstLine::Dialogue(d) => {
                let (line, extra) = self.dialogue(d);
                (NodeKind::Line(line), i + 1, extra)
            }
            AstLine::Title(t) => one(NodeKind::Anchor {
                id: t.name().map(|n| n.text().to_owned()).unwrap_or_default(),
            }),
            AstLine::Random(r) => {
                let body = self.children(r.syntax());
                one(NodeKind::Random {
                    options: vec![RandomOption {
                        weight: None,
                        condition: None,
                        span: self.span(r.syntax()),
                        body,
                    }],
                })
            }
            AstLine::Blank(_) | AstLine::Comment(_) | AstLine::Import(_) | AstLine::Using(_) => {
                unreachable!("skipped by block")
            }
        }
    }

    fn children(&mut self, line: &SyntaxNode) -> Vec<Node> {
        let lines: Vec<AstLine> = line
            .children()
            .find_map(ast::Block::cast)
            .map(|b| b.lines().collect())
            .unwrap_or_default();
        self.block(&lines)
    }

    /// Responses in a row (comments between them allowed) make one menu.
    fn menu(&mut self, lines: &[AstLine], start: usize, before: &mut Pending) -> (NodeKind, usize) {
        let mut choices = Vec::new();
        let mut pending = mem::take(before);
        let mut next = start;
        let mut i = start;
        while let Some(line) = lines.get(i) {
            match line {
                AstLine::Response(response) => {
                    choices.push(self.choice(response, pending.take()));
                    next = i + 1;
                }
                AstLine::Comment(comment) => self.comment(comment, &mut pending),
                AstLine::Blank(_) => {}
                _ => break,
            }
            i += 1;
        }
        // Comments after the last choice belong to whatever comes next: the caller reads them again.
        (NodeKind::Menu { choices }, next)
    }

    fn choice(
        &mut self,
        response: &ast::ResponseLine,
        (notes, marks): (Vec<Note>, Vec<Mark>),
    ) -> Choice {
        let condition_node = response.condition();
        let condition = condition_node
            .as_ref()
            .and_then(|c| self.expr(c.expr(), Some(FlagAccess::Read)));
        Choice {
            span: self.span(response.syntax()),
            notes,
            marks,
            speaker: response.speaker().map(|s| s.name()),
            text: self.segments(
                response.syntax(),
                condition_node.as_ref().map(|c| c.syntax()),
            ),
            words: crate::edit::words_source(response.syntax()),
            tags: self.tags(response.tags()),
            condition,
            jump: response.jump().map(|j| self.jump(&j)),
            id: static_id(response.static_id()),
            body: self.children(response.syntax()),
        }
    }

    /// `if` and the `elif`s and `else` after it.
    fn branch(&mut self, lines: &[AstLine], start: usize) -> (NodeKind, usize) {
        let mut arms = Vec::new();
        let mut pending = Pending::default();
        let mut next = start;
        let mut i = start;
        while let Some(line) = lines.get(i) {
            match line {
                AstLine::Condition(c) => {
                    let keyword = arm_keyword(c);
                    if i > start && keyword == ArmKeyword::If {
                        break;
                    }
                    arms.push(self.arm(keyword, c.expr(), c.syntax(), mem::take(&mut pending)));
                    next = i + 1;
                }
                AstLine::Comment(comment) => self.comment(comment, &mut pending),
                AstLine::Blank(_) => {}
                _ => break,
            }
            i += 1;
        }
        // Comments between arms were read for the arm after them; re-read any after the last.
        (NodeKind::Branch { arms }, next)
    }

    fn arm(
        &mut self,
        keyword: ArmKeyword,
        expr: Option<ast::Expr>,
        line: &SyntaxNode,
        pending: Pending,
    ) -> Arm {
        let mut pending = pending;
        let (notes, marks) = pending.take();
        Arm {
            keyword,
            condition: self.expr(expr, Some(FlagAccess::Read)),
            span: self.span(line),
            notes,
            marks,
            body: self.children(line),
        }
    }

    fn match_(&mut self, m: &ast::MatchLine) -> NodeKind {
        let subject = self
            .expr(m.expr(), Some(FlagAccess::Read))
            .unwrap_or_default();
        let whens_are_flags = self.config.flags.match_subjects.contains(&subject);
        let lines: Vec<AstLine> = m
            .syntax()
            .children()
            .find_map(ast::Block::cast)
            .map(|b| b.lines().collect())
            .unwrap_or_default();
        let mut arms = Vec::new();
        let mut pending = Pending::default();
        for line in &lines {
            match line {
                AstLine::When(when) => {
                    if whens_are_flags && let Some(expr) = when.expr() {
                        for token in tokens(expr.syntax()).filter(|t| t.kind() == STRING) {
                            self.flag(unquote(token.text()), FlagAccess::Read, expr.syntax());
                        }
                    }
                    let arm = self.arm(
                        ArmKeyword::When,
                        when.expr(),
                        when.syntax(),
                        mem::take(&mut pending),
                    );
                    arms.push(arm);
                }
                AstLine::Condition(c) => {
                    let arm = self.arm(
                        arm_keyword(c),
                        c.expr(),
                        c.syntax(),
                        mem::take(&mut pending),
                    );
                    arms.push(arm);
                }
                AstLine::Comment(comment) => self.comment(comment, &mut pending),
                // Anything else is a Dialogue Manager error; the checks do not need it here.
                _ => {}
            }
        }
        NodeKind::Match { subject, arms }
    }

    /// `%` lines in a row (no blank line between them) make one random pick.
    fn random(&mut self, lines: &[AstLine], start: usize) -> (NodeKind, usize) {
        let mut options = Vec::new();
        let mut i = start;
        while let Some(line) = lines.get(i).filter(|l| is_random(l)) {
            let prefix = line.random_prefix();
            let weight = prefix
                .as_ref()
                .and_then(|p| p.weight())
                .map(|w| w.text().to_owned());
            let condition = prefix
                .as_ref()
                .and_then(|p| p.condition())
                .and_then(|c| self.expr(c.expr(), Some(FlagAccess::Read)));
            let body = match line {
                AstLine::Random(r) => self.children(r.syntax()),
                _ => {
                    let mut pending = Pending::default();
                    let (kind, _, extra) = self.node(std::slice::from_ref(line), 0, &mut pending);
                    let mut body = vec![Node {
                        span: self.span(line.syntax()),
                        notes: Vec::new(),
                        marks: Vec::new(),
                        kind,
                    }];
                    body.extend(extra);
                    body
                }
            };
            options.push(RandomOption {
                weight,
                condition,
                span: self.span(line.syntax()),
                body,
            });
            i += 1;
        }
        (NodeKind::Random { options }, i)
    }

    /// A dialogue line, with the dialogue lines nested under it as more of its text.
    fn dialogue(&mut self, d: &ast::DialogueLine) -> (Line, Vec<Node>) {
        let mut line = Line {
            speaker: d.speaker().map(|s| s.name()),
            text: self.segments(d.syntax(), None),
            words: crate::edit::words_source(d.syntax()),
            tags: self.tags(d.tags()),
            jump: d.jump().map(|j| self.jump(&j)),
            id: static_id(d.static_id()),
            concurrent: d.is_concurrent(),
        };
        let children: Vec<AstLine> = AstLine::cast(d.syntax().clone())
            .and_then(|l| l.block())
            .map(|b| b.lines().collect())
            .unwrap_or_default();
        let mut others = Vec::new();
        for child in children {
            match &child {
                AstLine::Dialogue(more) => {
                    line.text.push(Segment::Text { text: "\n".into() });
                    line.text.extend(self.segments(more.syntax(), None));
                    line.tags.extend(self.tags(more.tags()));
                    if line.jump.is_none() {
                        line.jump = more.jump().map(|j| self.jump(&j));
                    }
                }
                _ => others.push(child),
            }
        }
        let extra = self.block(&others);
        (line, extra)
    }

    /// The text of a dialogue or response line, without speaker, tags, id or jump.
    fn segments(&mut self, line: &SyntaxNode, skip: Option<&SyntaxNode>) -> Vec<Segment> {
        let mut out: Vec<Segment> = Vec::new();
        let push_text = |out: &mut Vec<Segment>, text: &str| match out.last_mut() {
            Some(Segment::Text { text: last }) => last.push_str(text),
            _ => out.push(Segment::Text {
                text: text.to_owned(),
            }),
        };
        for element in line.children_with_tokens() {
            match element {
                SyntaxElement::Token(token) => match token.kind() {
                    TEXT | WHITESPACE | ERROR => push_text(&mut out, token.text()),
                    ESCAPE => push_text(&mut out, unescape(token.text())),
                    _ => {}
                },
                SyntaxElement::Node(node) => {
                    if skip == Some(&node) {
                        continue;
                    }
                    let segment = match node.kind() {
                        INLINE_RANDOM => {
                            let random = ast::InlineRandom::cast(node).expect("kind checked");
                            Segment::Random {
                                options: random.options(),
                            }
                        }
                        REPLACEMENT => {
                            let replacement = ast::Replacement::cast(node).expect("kind checked");
                            Segment::Expr {
                                expr: self.expr(replacement.expr(), None).unwrap_or_default(),
                            }
                        }
                        INLINE_CONDITION => {
                            let condition = ast::InlineCondition::cast(node).expect("kind checked");
                            Segment::If {
                                condition: self
                                    .expr(condition.expr(), Some(FlagAccess::Read))
                                    .unwrap_or_default(),
                            }
                        }
                        INLINE_MUTATION => {
                            let mutation = ast::InlineMutation::cast(node).expect("kind checked");
                            Segment::Effect {
                                expr: self
                                    .expr(mutation.expr(), Some(FlagAccess::Set))
                                    .unwrap_or_default(),
                            }
                        }
                        BBCODE => {
                            let bbcode = ast::BbCode::cast(node).expect("kind checked");
                            match bbcode.name().as_ref().map(SyntaxToken::text) {
                                Some("else") => Segment::Else,
                                Some("/if") => Segment::EndIf,
                                name => Segment::Markup {
                                    name: name.unwrap_or_default().to_owned(),
                                    args: bbcode.args().map(|a| a.text().to_owned()),
                                },
                            }
                        }
                        _ => continue,
                    };
                    out.push(segment);
                }
            }
        }
        if let Some(Segment::Text { text }) = out.first_mut() {
            *text = text.trim_start().to_owned();
        }
        if let Some(Segment::Text { text }) = out.last_mut() {
            *text = text.trim_end().to_owned();
        }
        out.retain(|s| !matches!(s, Segment::Text { text } if text.is_empty()));
        out
    }

    fn tags(&self, tags: impl Iterator<Item = ast::Tag>) -> Vec<Tag> {
        tags.filter_map(|tag| {
            Some(Tag {
                name: tag.name()?.text().to_owned(),
                value: tag.value().map(|v| v.text().to_owned()),
                span: self.span(tag.syntax()),
            })
        })
        .collect()
    }

    fn jump(&mut self, jump: &ast::Jump) -> Jump {
        let target = match (jump.target(), jump.target_expr()) {
            (_, Some(replacement)) => {
                JumpTarget::Expression(self.expr(replacement.expr(), None).unwrap_or_default())
            }
            (Some(target), None) => match target.text() {
                "END" => JumpTarget::End,
                "END!" => JumpTarget::EndConversation,
                name => JumpTarget::Label(name.to_owned()),
            },
            (None, None) => JumpTarget::Label(String::new()),
        };
        Jump {
            target,
            returns: jump.is_return(),
            span: self.span(jump.syntax()),
        }
    }

    /// The expression's text; flags it sets or reads (by the functions in the project
    /// file) are recorded. `access` says which a bare call means here.
    fn expr(&mut self, expr: Option<ast::Expr>, access: Option<FlagAccess>) -> Option<String> {
        let expr = expr?;
        if access.is_some() {
            let tokens: Vec<SyntaxToken> = tokens(expr.syntax())
                .filter(|t| t.kind() != WHITESPACE)
                .collect();
            for window in tokens.windows(3) {
                let [name, open, arg] = window else { continue };
                if name.kind() != IDENT || open.kind() != L_PAREN || arg.kind() != STRING {
                    continue;
                }
                let flags = &self.config.flags;
                let access = if flags.set.iter().any(|f| f == name.text()) {
                    FlagAccess::Set
                } else if flags.read.iter().any(|f| f == name.text()) {
                    FlagAccess::Read
                } else {
                    continue;
                };
                self.flag(unquote(arg.text()), access, expr.syntax());
            }
        }
        Some(expr.syntax().text().to_string())
    }

    fn flag(&mut self, flag: &str, access: FlagAccess, at: &SyntaxNode) {
        self.flags.push(FlagRef {
            flag: flag.to_owned(),
            access,
            label: self.label.clone(),
            span: self.span(at),
        });
    }
}

fn tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> + use<> {
    node.descendants_with_tokens()
        .filter_map(SyntaxElement::into_token)
}

fn current_section(out: &mut ScriptFile) -> &mut Section {
    if out.sections.is_empty() {
        out.sections.push(Section::default());
    }
    out.sections.last_mut().expect("just ensured")
}

/// A line in a random group: a bare `%`, or a `%`-prefixed line that says something
/// (a `% # comment` is still a comment).
fn is_random(line: &AstLine) -> bool {
    match line {
        AstLine::Random(_) => true,
        AstLine::Blank(_) | AstLine::Comment(_) | AstLine::Import(_) | AstLine::Using(_) => false,
        _ => line.random_prefix().is_some(),
    }
}

fn arm_keyword(c: &ast::ConditionLine) -> ArmKeyword {
    let keywords: Vec<_> = c
        .syntax()
        .children_with_tokens()
        .filter(|e| matches!(e.kind(), IF_KW | ELIF_KW | ELSE_KW))
        .map(|e| e.kind())
        .collect();
    match keywords.as_slice() {
        [IF_KW, ..] => ArmKeyword::If,
        [ELIF_KW, ..] | [ELSE_KW, IF_KW, ..] => ArmKeyword::Elif,
        _ => ArmKeyword::Else,
    }
}

fn static_id(id: Option<ast::StaticId>) -> Option<String> {
    id.and_then(|i| i.value()).map(|v| v.text().to_owned())
}

/// `"x"`, `'x'` or `&"x"` → `x`.
pub(crate) fn unquote(text: &str) -> &str {
    let text = text.strip_prefix('&').unwrap_or(text);
    text.get(1..text.len().saturating_sub(1)).unwrap_or("")
}

fn unescape(escape: &str) -> &str {
    match escape {
        "\\n" => "\n",
        "\\" => "",
        other => &other[1..],
    }
}

/// Give each label the first place whose files and label filters take it.
fn assign_places(script: &mut Script, config: &Config) {
    for place in &config.places {
        for file in script
            .files
            .iter_mut()
            .filter(|f| place.files.contains(&f.path))
        {
            for label in file.sections.iter_mut().flat_map(|s| &mut s.labels) {
                let wanted = place.labels.is_empty() || place.labels.contains(&label.id);
                if label.place.is_none() && wanted && !place.exclude_labels.contains(&label.id) {
                    label.place = Some(place.id.clone());
                }
            }
        }
    }
}
