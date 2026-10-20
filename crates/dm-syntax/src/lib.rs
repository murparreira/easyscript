//! Lossless syntax tree for Dialogue Manager `.dialogue` files.
//!
//! The first format adapter. Every file prints back byte for byte:
//! `parse(text).syntax().to_string() == text`, for any text. Nothing fails to parse;
//! what the parser does not read becomes `TEXT` or `ERROR` tokens, kept verbatim.
//!
//! The tree has one node per line, typed the way Dialogue Manager types it, and lines
//! nested by indentation sit in a [`SyntaxKind::BLOCK`] under the line above them. Blank
//! and comment lines sit at the level of the next line, as Dialogue Manager places them.
//! Inside a line: the speaker, tags, inline conditions and mutations, `{{ }}`
//! replacements, `[[a|b]]` random text, BBCode, static ids, jumps, and expressions as
//! flat token streams.
//!
//! The grammar follows the compiler of Dialogue Manager [`DIALOGUE_MANAGER_VERSION`].
//! Files must be UTF-8 (Godot reads them as UTF-8); a leading byte order mark is kept.

pub mod ast;
mod expr;
mod kind;
mod line;
mod parser;

pub use kind::{DialogueLanguage, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};
pub use line::indent_level;
pub use rowan::{GreenNode, TextRange, TextSize, ast::AstNode};

/// The Dialogue Manager release whose compiler the grammar follows.
pub const DIALOGUE_MANAGER_VERSION: &str = "3.10.3";

/// A parsed file. Cheap to clone: it shares the immutable tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Parse {
    green: GreenNode,
}

/// Parse a `.dialogue` file. Never fails and never panics.
pub fn parse(text: &str) -> Parse {
    Parse {
        green: parser::parse(text),
    }
}

impl Parse {
    pub fn green(&self) -> &GreenNode {
        &self.green
    }

    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    pub fn tree(&self) -> ast::SourceFile {
        ast::SourceFile::cast(self.syntax()).expect("the root is a source file")
    }

    /// Tokens the parser could not read (kept in the tree as they were).
    pub fn errors(&self) -> impl Iterator<Item = SyntaxToken> + use<> {
        self.syntax()
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::ERROR)
    }
}

#[cfg(test)]
mod tests;
