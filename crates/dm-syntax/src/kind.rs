//! Token and node kinds, and the rowan language they belong to.

macro_rules! kinds {
    ($($(#[$meta:meta])* $name:ident,)*) => {
        /// Every kind of token and node in a `.dialogue` syntax tree.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
        #[repr(u16)]
        pub enum SyntaxKind {
            $($(#[$meta])* $name,)*
        }

        impl SyntaxKind {
            const ALL: &[SyntaxKind] = &[$(SyntaxKind::$name,)*];
        }
    };
}

kinds! {
    // Layout and trivia.
    /// A UTF-8 byte order mark at the start of the file.
    BOM,
    /// Leading whitespace of a line that sets its nesting level.
    INDENT,
    /// Any other run of whitespace.
    WHITESPACE,
    /// `\n` or `\r\n`.
    NEWLINE,
    /// `# ...` to the end of the line: a comment line, or a comment inside an expression.
    COMMENT,
    /// Text the lexer could not read, kept verbatim.
    ERROR,

    // Dialogue text.
    /// A run of dialogue or choice text.
    TEXT,
    /// A backslash escape: `\[`, `\]`, `\:`, `\n`, or the `\` that stops a line reading as a keyword.
    ESCAPE,
    /// Name of a BBCode tag (`wave`, `/wave`, `color`, `wait`, `speed`, ...).
    BBCODE_NAME,
    /// Everything after the name of a BBCode tag, before `]`.
    BBCODE_ARGS,
    /// Name of a line tag (`reaction` in `[#reaction=x]`).
    TAG_NAME,
    /// Value of a line tag (`x` in `[#reaction=x]`).
    TAG_VALUE,
    /// `ID` in `[ID:...]`.
    ID_KW,
    /// The static line id in `[ID:...]`.
    ID_VALUE,
    /// The title name in `~ name`.
    TITLE_NAME,
    /// What a jump goes to: a title, `END` or `END!`.
    JUMP_TARGET,

    // Line markers.
    /// `~`
    TILDE,
    /// `=>`
    ARROW,
    /// `=><`
    ARROW_RETURN,
    /// `-` opening a response.
    DASH,
    /// `|`: a concurrent line, or between inline random options.
    PIPE,
    /// `%`: weighted random lines, and modulo in expressions.
    PERCENT,

    // Keywords.
    IF_KW,
    ELIF_KW,
    ELSE_KW,
    WHILE_KW,
    MATCH_KW,
    WHEN_KW,
    DO_KW,
    /// `do!`
    DO_BANG_KW,
    SET_KW,
    /// `$>`
    DOLLAR_KW,
    /// `$>>`
    DOLLAR2_KW,
    IMPORT_KW,
    AS_KW,
    USING_KW,
    AND_KW,
    OR_KW,
    NOT_KW,
    IN_KW,
    TRUE_KW,
    FALSE_KW,
    NULL_KW,

    // Expressions.
    IDENT,
    NUMBER,
    /// `"..."` or `'...'`, optionally prefixed with `&`.
    STRING,
    L_PAREN,
    R_PAREN,
    L_BRACK,
    R_BRACK,
    L_BRACE,
    R_BRACE,
    /// `{{`
    L_DBRACE,
    /// `}}`
    R_DBRACE,
    /// `[[`
    L_DBRACK,
    /// `]]`
    R_DBRACK,
    COLON,
    COMMA,
    DOT,
    /// `?.`
    QUESTION_DOT,
    HASH,
    EQ,
    EQ2,
    NEQ,
    LT,
    LTEQ,
    GT,
    GTEQ,
    PLUS,
    MINUS,
    STAR,
    SLASH,
    PLUS_EQ,
    MINUS_EQ,
    STAR_EQ,
    SLASH_EQ,
    BANG,
    AMP2,
    PIPE2,

    // Nodes: the file and its structure.
    /// The whole file.
    SOURCE_FILE,
    /// The lines nested under a line by indentation.
    BLOCK,

    // Nodes: one per line, typed as Dialogue Manager types them.
    BLANK_LINE,
    COMMENT_LINE,
    /// `import "res://file.dialogue" as name`
    IMPORT_LINE,
    /// `using StateName`
    USING_LINE,
    /// `~ title`
    TITLE_LINE,
    /// `if`, `elif`, `else`
    CONDITION_LINE,
    WHILE_LINE,
    MATCH_LINE,
    WHEN_LINE,
    /// `do`, `do!`, `set`, `$>`, `$>>`
    MUTATION_LINE,
    /// `=> title` or `=>< title`
    GOTO_LINE,
    /// `- choice`
    RESPONSE_LINE,
    /// A bare `%` opening a block of random lines.
    RANDOM_LINE,
    /// Everything else with text: `Speaker: line`, or narration.
    DIALOGUE_LINE,

    // Nodes inside lines.
    /// `%`, an optional weight and `[if ...]`, before a randomised line.
    RANDOM_PREFIX,
    /// The speaker of a line, up to and including the `:`.
    SPEAKER,
    /// A condition or mutation expression.
    EXPR,
    /// `=> target` or `=>< target`, on its own line or at the end of one.
    JUMP,
    /// `[#a, b=c]`
    TAG_GROUP,
    /// One tag in a group.
    TAG,
    /// `[ID:...]`
    STATIC_ID,
    /// `[if ...]`: a condition on a response or random line, or opening an inline condition.
    INLINE_CONDITION,
    /// `[do ...]`, `[set ...]`, `[$> ...]` inside a line.
    INLINE_MUTATION,
    /// `[[a|b|c]]`
    INLINE_RANDOM,
    /// `{{ expression }}`
    REPLACEMENT,
    /// Any other `[...]`: BBCode, `[wait]`, `[speed]`, `[next]`, `[else]`, `[/if]`.
    BBCODE,
}

impl SyntaxKind {
    /// Is this the kind of a node holding one line?
    pub fn is_line(self) -> bool {
        (SyntaxKind::BLANK_LINE as u16..=SyntaxKind::DIALOGUE_LINE as u16).contains(&(self as u16))
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

/// The rowan language of `.dialogue` files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DialogueLanguage {}

impl rowan::Language for DialogueLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> SyntaxKind {
        SyntaxKind::ALL[raw.0 as usize]
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<DialogueLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<DialogueLanguage>;
pub type SyntaxElement = rowan::SyntaxElement<DialogueLanguage>;
