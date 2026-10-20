//! Typed views over the syntax tree. They hold a node and read it on demand; the tree
//! stays the only copy of the source.

use rowan::ast::AstNode;

use crate::SyntaxKind::{self, *};
use crate::{DialogueLanguage, SyntaxNode, SyntaxToken, indent_level};

macro_rules! node {
    ($($(#[$meta:meta])* $name:ident = $kind:ident;)*) => {$(
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(SyntaxNode);

        impl AstNode for $name {
            type Language = DialogueLanguage;

            fn can_cast(kind: SyntaxKind) -> bool {
                kind == $kind
            }

            fn cast(node: SyntaxNode) -> Option<Self> {
                Self::can_cast(node.kind()).then(|| Self(node))
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    )*};
}

node! {
    /// The whole file.
    SourceFile = SOURCE_FILE;
    /// Lines nested under a line.
    Block = BLOCK;
    BlankLine = BLANK_LINE;
    /// `# ...`; `## ...` is a doc comment, attached to the next line.
    CommentLine = COMMENT_LINE;
    ImportLine = IMPORT_LINE;
    UsingLine = USING_LINE;
    /// `~ title`
    TitleLine = TITLE_LINE;
    /// `if`, `elif` or `else`
    ConditionLine = CONDITION_LINE;
    WhileLine = WHILE_LINE;
    MatchLine = MATCH_LINE;
    WhenLine = WHEN_LINE;
    /// `do`, `do!`, `set`, `$>`, `$>>`
    MutationLine = MUTATION_LINE;
    /// `=> title`
    GotoLine = GOTO_LINE;
    /// `- choice`
    ResponseLine = RESPONSE_LINE;
    /// A bare `%` over a block of random lines.
    RandomLine = RANDOM_LINE;
    /// `Speaker: text`, or narration.
    DialogueLine = DIALOGUE_LINE;
    /// `%2 [if x] ` before a randomised line.
    RandomPrefix = RANDOM_PREFIX;
    /// The speaker of a line, with its `:`.
    Speaker = SPEAKER;
    Expr = EXPR;
    /// `=> target` or `=>< target`.
    Jump = JUMP;
    /// `[#a, b=c]`
    TagGroup = TAG_GROUP;
    Tag = TAG;
    /// `[ID:...]`
    StaticId = STATIC_ID;
    /// `[if ...]`
    InlineCondition = INLINE_CONDITION;
    /// `[do ...]` and the like.
    InlineMutation = INLINE_MUTATION;
    /// `[[a|b|c]]`
    InlineRandom = INLINE_RANDOM;
    /// `{{ ... }}`
    Replacement = REPLACEMENT;
    /// `[wave]`, `[/wave]`, `[wait=1]`, ...
    BbCode = BBCODE;
}

fn token(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == kind)
}

fn child<N: AstNode<Language = DialogueLanguage>>(node: &SyntaxNode) -> Option<N> {
    node.children().find_map(N::cast)
}

fn children<N: AstNode<Language = DialogueLanguage>>(
    node: &SyntaxNode,
) -> impl Iterator<Item = N> + use<N> {
    node.children().filter_map(N::cast)
}

/// Any line.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Line {
    Blank(BlankLine),
    Comment(CommentLine),
    Import(ImportLine),
    Using(UsingLine),
    Title(TitleLine),
    Condition(ConditionLine),
    While(WhileLine),
    Match(MatchLine),
    When(WhenLine),
    Mutation(MutationLine),
    Goto(GotoLine),
    Response(ResponseLine),
    Random(RandomLine),
    Dialogue(DialogueLine),
}

impl AstNode for Line {
    type Language = DialogueLanguage;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind.is_line()
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        Some(match node.kind() {
            BLANK_LINE => Self::Blank(BlankLine(node)),
            COMMENT_LINE => Self::Comment(CommentLine(node)),
            IMPORT_LINE => Self::Import(ImportLine(node)),
            USING_LINE => Self::Using(UsingLine(node)),
            TITLE_LINE => Self::Title(TitleLine(node)),
            CONDITION_LINE => Self::Condition(ConditionLine(node)),
            WHILE_LINE => Self::While(WhileLine(node)),
            MATCH_LINE => Self::Match(MatchLine(node)),
            WHEN_LINE => Self::When(WhenLine(node)),
            MUTATION_LINE => Self::Mutation(MutationLine(node)),
            GOTO_LINE => Self::Goto(GotoLine(node)),
            RESPONSE_LINE => Self::Response(ResponseLine(node)),
            RANDOM_LINE => Self::Random(RandomLine(node)),
            DIALOGUE_LINE => Self::Dialogue(DialogueLine(node)),
            _ => return None,
        })
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Blank(n) => n.syntax(),
            Self::Comment(n) => n.syntax(),
            Self::Import(n) => n.syntax(),
            Self::Using(n) => n.syntax(),
            Self::Title(n) => n.syntax(),
            Self::Condition(n) => n.syntax(),
            Self::While(n) => n.syntax(),
            Self::Match(n) => n.syntax(),
            Self::When(n) => n.syntax(),
            Self::Mutation(n) => n.syntax(),
            Self::Goto(n) => n.syntax(),
            Self::Response(n) => n.syntax(),
            Self::Random(n) => n.syntax(),
            Self::Dialogue(n) => n.syntax(),
        }
    }
}

impl Line {
    /// Nesting level from the line's own indentation.
    pub fn indent_level(&self) -> usize {
        token(self.syntax(), INDENT).map_or(0, |t| indent_level(t.text()))
    }

    /// The lines nested under this one.
    pub fn block(&self) -> Option<Block> {
        child(self.syntax())
    }

    pub fn random_prefix(&self) -> Option<RandomPrefix> {
        child(self.syntax())
    }

    /// The line itself, without the lines nested under it, as written (newline included).
    pub fn own_text(&self) -> String {
        self.syntax()
            .children_with_tokens()
            .take_while(|e| e.kind() != BLOCK)
            .map(|e| e.to_string())
            .collect()
    }
}

impl SourceFile {
    /// Lines at the top level, in order.
    pub fn lines(&self) -> impl Iterator<Item = Line> + use<> {
        children(&self.0)
    }

    /// Every line in the file, nested ones included, in source order.
    pub fn all_lines(&self) -> impl Iterator<Item = Line> + use<> {
        self.0.descendants().filter_map(Line::cast)
    }
}

impl Block {
    pub fn lines(&self) -> impl Iterator<Item = Line> + use<> {
        children(&self.0)
    }
}

impl CommentLine {
    /// The comment, `#` included.
    pub fn comment(&self) -> Option<SyntaxToken> {
        token(&self.0, COMMENT)
    }

    /// `## ...`, which Dialogue Manager attaches as notes to the next line.
    pub fn is_doc(&self) -> bool {
        self.comment().is_some_and(|c| c.text().starts_with("##"))
    }
}

impl TitleLine {
    pub fn name(&self) -> Option<SyntaxToken> {
        token(&self.0, TITLE_NAME)
    }
}

impl ImportLine {
    /// The path, quotes included.
    pub fn path(&self) -> Option<SyntaxToken> {
        token(&self.0, STRING)
    }

    pub fn alias(&self) -> Option<SyntaxToken> {
        token(&self.0, IDENT)
    }
}

impl UsingLine {
    pub fn state(&self) -> Option<SyntaxToken> {
        token(&self.0, IDENT)
    }
}

impl ConditionLine {
    /// `if`, `elif` or `else` (for `else if`, the `else`).
    pub fn keyword(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| matches!(t.kind(), IF_KW | ELIF_KW | ELSE_KW))
    }

    pub fn expr(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl WhileLine {
    pub fn expr(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl MatchLine {
    pub fn expr(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl WhenLine {
    pub fn expr(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl MutationLine {
    pub fn keyword(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| {
                matches!(
                    t.kind(),
                    DO_KW | DO_BANG_KW | SET_KW | DOLLAR_KW | DOLLAR2_KW
                )
            })
    }

    pub fn expr(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl GotoLine {
    pub fn jump(&self) -> Option<Jump> {
        child(&self.0)
    }
}

macro_rules! text_line {
    ($($name:ident),*) => {$(
        impl $name {
            pub fn speaker(&self) -> Option<Speaker> {
                child(&self.0)
            }

            /// Tags in all the line's tag groups, in order.
            pub fn tags(&self) -> impl Iterator<Item = Tag> + use<> {
                children::<TagGroup>(&self.0).flat_map(|group| group.tags())
            }

            /// A jump at the end of the line.
            pub fn jump(&self) -> Option<Jump> {
                child(&self.0)
            }

            pub fn static_id(&self) -> Option<StaticId> {
                child(&self.0)
            }

            /// `[if ...]` on the line. On a response, this is the condition that shows it.
            pub fn condition(&self) -> Option<InlineCondition> {
                child(&self.0)
            }
        }
    )*};
}

text_line!(DialogueLine, ResponseLine);

impl DialogueLine {
    /// `| text`: shown at the same time as the line before.
    pub fn is_concurrent(&self) -> bool {
        token(&self.0, PIPE).is_some()
    }
}

impl RandomPrefix {
    pub fn weight(&self) -> Option<SyntaxToken> {
        token(&self.0, NUMBER)
    }

    pub fn condition(&self) -> Option<InlineCondition> {
        child(&self.0)
    }
}

impl Speaker {
    /// The speaker as written, without the `:` and surrounding whitespace.
    pub fn name(&self) -> String {
        let text: String = self
            .0
            .children_with_tokens()
            .filter(|e| e.kind() != COLON)
            .map(|e| e.to_string())
            .collect();
        text.trim().to_owned()
    }
}

impl Jump {
    /// `=><`: jump, then come back.
    pub fn is_return(&self) -> bool {
        token(&self.0, ARROW_RETURN).is_some()
    }

    /// A title, `END` or `END!`.
    pub fn target(&self) -> Option<SyntaxToken> {
        token(&self.0, JUMP_TARGET)
    }

    /// `=> {{ expression }}`
    pub fn target_expr(&self) -> Option<Replacement> {
        child(&self.0)
    }
}

impl TagGroup {
    pub fn tags(&self) -> impl Iterator<Item = Tag> + use<> {
        children(&self.0)
    }
}

impl Tag {
    pub fn name(&self) -> Option<SyntaxToken> {
        token(&self.0, TAG_NAME)
    }

    pub fn value(&self) -> Option<SyntaxToken> {
        token(&self.0, TAG_VALUE)
    }
}

impl StaticId {
    pub fn value(&self) -> Option<SyntaxToken> {
        token(&self.0, ID_VALUE)
    }
}

impl InlineCondition {
    pub fn expr(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl InlineMutation {
    pub fn expr(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl Replacement {
    pub fn expr(&self) -> Option<Expr> {
        child(&self.0)
    }
}

impl InlineRandom {
    /// The options, as written.
    pub fn options(&self) -> Vec<String> {
        let inner = self.0.text().to_string();
        inner[2..inner.len() - 2]
            .split('|')
            .map(str::to_owned)
            .collect()
    }
}

impl BbCode {
    pub fn name(&self) -> Option<SyntaxToken> {
        token(&self.0, BBCODE_NAME)
    }

    pub fn args(&self) -> Option<SyntaxToken> {
        token(&self.0, BBCODE_ARGS)
    }
}
