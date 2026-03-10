use smol_str::SmolStr;
use strum::{Display, EnumString};

use crate::Span;

/// A single token produced by the lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub text: SmolStr,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span, text: impl Into<SmolStr>) -> Self {
        Self {
            kind,
            span,
            text: text.into(),
        }
    }
}

/// Classification of tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString)]
pub enum TokenKind {
    // Identifiers & Keywords
    /// An unquoted word (keyword or identifier — distinguished later by dialect).
    Word,

    // Literals
    NumberLiteral,
    /// Single-quoted string: 'hello'
    StringLiteral,
    /// Quoted identifier: "col", [col], `col`
    QuotedIdentifier,

    // Operators
    Dot,
    Comma,
    Semicolon,
    LParen,
    RParen,
    Star,
    Plus,
    Minus,
    Slash,
    Percent,
    Eq,
    Neq, // <> or !=
    Lt,
    Gt,
    LtEq,
    GtEq,
    Concat,     // ||
    ColonColon, // :: (PostgreSQL cast)
    AtSign,     // @ (SQL Server variable prefix)
    Colon,      // : (named parameter)

    // Whitespace & Comments
    Whitespace,
    Newline,
    LineComment,  // -- ...
    BlockComment, // /* ... */

    // Special
    /// Parameter placeholder: :name, $1, ?
    Placeholder,
    /// End of file
    Eof,
}

impl TokenKind {
    /// Returns true if this token is trivia (whitespace or comment).
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            TokenKind::Whitespace
                | TokenKind::Newline
                | TokenKind::LineComment
                | TokenKind::BlockComment
        )
    }
}
