use rigsql_core::{Span, Token, TokenKind};
use smol_str::SmolStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LexerError {
    #[error("Unexpected character '{ch}' at offset {offset}")]
    UnexpectedChar { ch: char, offset: u32 },
    #[error("Unterminated string literal starting at offset {offset}")]
    UnterminatedString { offset: u32 },
    #[error("Unterminated block comment starting at offset {offset}")]
    UnterminatedBlockComment { offset: u32 },
    #[error("Unterminated quoted identifier starting at offset {offset}")]
    UnterminatedQuotedIdentifier { offset: u32 },
}

/// Dialect-specific lexer configuration.
#[derive(Debug, Clone, Default)]
pub struct LexerConfig {
    /// Enable `::` as cast operator (PostgreSQL).
    pub double_colon: bool,
    /// Enable `[identifier]` quoting (SQL Server).
    pub bracket_identifiers: bool,
    /// Enable backtick identifier quoting (MySQL).
    pub backtick_identifiers: bool,
    /// Enable `@@variable` (SQL Server).
    pub double_at: bool,
    /// Enable dollar-quoted strings `$$...$$` (PostgreSQL).
    pub dollar_quoting: bool,
}

impl LexerConfig {
    pub fn ansi() -> Self {
        Self::default()
    }

    pub fn postgres() -> Self {
        Self {
            double_colon: true,
            dollar_quoting: true,
            ..Self::default()
        }
    }

    pub fn tsql() -> Self {
        Self {
            bracket_identifiers: true,
            double_at: true,
            ..Self::default()
        }
    }
}

pub struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
    config: LexerConfig,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, config: LexerConfig) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            config,
        }
    }

    /// Tokenize the entire source into a Vec of tokens.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, LexerError> {
        if self.pos >= self.bytes.len() {
            return Ok(Token::new(
                TokenKind::Eof,
                Span::new(self.pos as u32, self.pos as u32),
                "",
            ));
        }

        let start = self.pos;
        let ch = self.bytes[self.pos];

        match ch {
            // Newline
            b'\n' => {
                self.pos += 1;
                Ok(self.make_token(TokenKind::Newline, start))
            }
            b'\r' => {
                self.pos += 1;
                if self.peek() == Some(b'\n') {
                    self.pos += 1;
                }
                Ok(self.make_token(TokenKind::Newline, start))
            }

            // Whitespace (not newline)
            b' ' | b'\t' => {
                self.pos += 1;
                while let Some(b) = self.peek() {
                    if b == b' ' || b == b'\t' {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                Ok(self.make_token(TokenKind::Whitespace, start))
            }

            // Line comment: -- ...
            b'-' if self.peek_at(1) == Some(b'-') => {
                self.pos += 2;
                while let Some(b) = self.peek() {
                    if b == b'\n' || b == b'\r' {
                        break;
                    }
                    self.pos += 1;
                }
                Ok(self.make_token(TokenKind::LineComment, start))
            }

            // Block comment: /* ... */
            b'/' if self.peek_at(1) == Some(b'*') => {
                self.pos += 2;
                let mut depth = 1u32;
                while self.pos < self.bytes.len() && depth > 0 {
                    if self.bytes[self.pos] == b'/' && self.peek_at(1) == Some(b'*') {
                        depth += 1;
                        self.pos += 2;
                    } else if self.bytes[self.pos] == b'*' && self.peek_at(1) == Some(b'/') {
                        depth -= 1;
                        self.pos += 2;
                    } else {
                        self.pos += 1;
                    }
                }
                if depth > 0 {
                    return Err(LexerError::UnterminatedBlockComment {
                        offset: start as u32,
                    });
                }
                Ok(self.make_token(TokenKind::BlockComment, start))
            }

            // String literal: 'hello'
            b'\'' => self.lex_string_literal(start),

            // Double-quoted identifier: "name"
            b'"' => self.lex_quoted_identifier(start, b'"'),

            // Bracket-quoted identifier: [name] (SQL Server)
            b'[' if self.config.bracket_identifiers => self.lex_bracket_identifier(start),

            // Array subscript brackets (PostgreSQL)
            b'[' => {
                self.pos += 1;
                Ok(self.make_token(TokenKind::LBracket, start))
            }
            b']' => {
                self.pos += 1;
                Ok(self.make_token(TokenKind::RBracket, start))
            }

            // Backtick identifier: `name` (MySQL)
            b'`' if self.config.backtick_identifiers => self.lex_quoted_identifier(start, b'`'),

            // Numbers
            b'0'..=b'9' => self.lex_number(start),

            // Dot (could be start of .123 numeric or just dot)
            b'.' if self.peek_at(1).is_some_and(|b| b.is_ascii_digit()) => self.lex_number(start),

            // Single-character operators & punctuation
            b'.' => {
                self.pos += 1;
                Ok(self.make_token(TokenKind::Dot, start))
            }
            b',' => {
                self.pos += 1;
                Ok(self.make_token(TokenKind::Comma, start))
            }
            b';' => {
                self.pos += 1;
                Ok(self.make_token(TokenKind::Semicolon, start))
            }
            b'(' => {
                self.pos += 1;
                Ok(self.make_token(TokenKind::LParen, start))
            }
            b')' => {
                self.pos += 1;
                Ok(self.make_token(TokenKind::RParen, start))
            }
            b'*' => {
                self.pos += 1;
                Ok(self.make_token(TokenKind::Star, start))
            }
            b'+' => {
                self.pos += 1;
                Ok(self.make_token(TokenKind::Plus, start))
            }
            b'-' => {
                // Single minus (-- already handled above)
                self.pos += 1;
                Ok(self.make_token(TokenKind::Minus, start))
            }
            b'/' => {
                // Single slash (/* already handled above)
                self.pos += 1;
                Ok(self.make_token(TokenKind::Slash, start))
            }
            b'%' => {
                self.pos += 1;
                Ok(self.make_token(TokenKind::Percent, start))
            }
            b'=' => {
                self.pos += 1;
                Ok(self.make_token(TokenKind::Eq, start))
            }

            // < <= <> operators
            b'<' => {
                self.pos += 1;
                match self.peek() {
                    Some(b'=') => {
                        self.pos += 1;
                        Ok(self.make_token(TokenKind::LtEq, start))
                    }
                    Some(b'>') => {
                        self.pos += 1;
                        Ok(self.make_token(TokenKind::Neq, start))
                    }
                    _ => Ok(self.make_token(TokenKind::Lt, start)),
                }
            }

            // > >= operators
            b'>' => {
                self.pos += 1;
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    Ok(self.make_token(TokenKind::GtEq, start))
                } else {
                    Ok(self.make_token(TokenKind::Gt, start))
                }
            }

            // != operator
            b'!' if self.peek_at(1) == Some(b'=') => {
                self.pos += 2;
                Ok(self.make_token(TokenKind::Neq, start))
            }

            // || concat operator
            b'|' if self.peek_at(1) == Some(b'|') => {
                self.pos += 2;
                Ok(self.make_token(TokenKind::Concat, start))
            }

            // :: cast (PostgreSQL)
            b':' if self.config.double_colon && self.peek_at(1) == Some(b':') => {
                self.pos += 2;
                Ok(self.make_token(TokenKind::ColonColon, start))
            }

            // : named parameter
            b':' => {
                self.pos += 1;
                if self
                    .peek()
                    .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    while self
                        .peek()
                        .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
                    {
                        self.pos += 1;
                    }
                    Ok(self.make_token(TokenKind::Placeholder, start))
                } else {
                    Ok(self.make_token(TokenKind::Colon, start))
                }
            }

            // @ or @@ (SQL Server)
            b'@' => {
                self.pos += 1;
                if self.config.double_at && self.peek() == Some(b'@') {
                    self.pos += 1;
                }
                // Read variable name (including non-ASCII chars like Japanese)
                self.eat_word_chars();
                Ok(self.make_token(TokenKind::AtSign, start))
            }

            // ? positional parameter
            b'?' => {
                self.pos += 1;
                Ok(self.make_token(TokenKind::Placeholder, start))
            }

            // $ positional parameter ($1) or dollar-quoting (PostgreSQL)
            b'$' => {
                if self.config.dollar_quoting {
                    self.lex_dollar_quote_or_param(start)
                } else {
                    self.pos += 1;
                    // $1, $2 etc
                    while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                        self.pos += 1;
                    }
                    Ok(self.make_token(TokenKind::Placeholder, start))
                }
            }

            // Word: keyword or identifier (including non-ASCII like Japanese)
            b if is_word_start(b) || b >= 0x80 => {
                if b >= 0x80 {
                    let s = &self.source[self.pos..];
                    let first_char = s.chars().next().unwrap();
                    self.pos += first_char.len_utf8();
                } else {
                    self.pos += 1;
                }
                self.eat_word_chars();
                Ok(self.make_token(TokenKind::Word, start))
            }

            _ => {
                let ch = self.source[self.pos..].chars().next().unwrap();
                Err(LexerError::UnexpectedChar {
                    ch,
                    offset: start as u32,
                })
            }
        }
    }

    fn lex_string_literal(&mut self, start: usize) -> Result<Token, LexerError> {
        self.pos += 1; // skip opening quote
        loop {
            match self.peek() {
                None => {
                    return Err(LexerError::UnterminatedString {
                        offset: start as u32,
                    })
                }
                Some(b'\'') => {
                    self.pos += 1;
                    // Escaped quote ''
                    if self.peek() == Some(b'\'') {
                        self.pos += 1;
                        continue;
                    }
                    return Ok(self.make_token(TokenKind::StringLiteral, start));
                }
                Some(_) => self.pos += 1,
            }
        }
    }

    fn lex_quoted_identifier(&mut self, start: usize, quote: u8) -> Result<Token, LexerError> {
        self.pos += 1; // skip opening quote
        loop {
            match self.peek() {
                None => {
                    return Err(LexerError::UnterminatedQuotedIdentifier {
                        offset: start as u32,
                    })
                }
                Some(b) if b == quote => {
                    self.pos += 1;
                    // Escaped quote
                    if self.peek() == Some(quote) {
                        self.pos += 1;
                        continue;
                    }
                    return Ok(self.make_token(TokenKind::QuotedIdentifier, start));
                }
                Some(_) => self.pos += 1,
            }
        }
    }

    fn lex_bracket_identifier(&mut self, start: usize) -> Result<Token, LexerError> {
        self.pos += 1; // skip [
        loop {
            match self.peek() {
                None => {
                    return Err(LexerError::UnterminatedQuotedIdentifier {
                        offset: start as u32,
                    })
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(self.make_token(TokenKind::QuotedIdentifier, start));
                }
                Some(_) => self.pos += 1,
            }
        }
    }

    fn lex_number(&mut self, start: usize) -> Result<Token, LexerError> {
        // Integer part
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1;
        }
        // Decimal part
        if self.peek() == Some(b'.') && self.peek_at(1).is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1; // skip .
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.pos += 1;
            }
        } else if self.bytes[start] == b'.' {
            // .123 form — dot already consumed before we got here
            self.pos += 1; // skip .
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        // Exponent part
        if self.peek().is_some_and(|b| b == b'e' || b == b'E') {
            self.pos += 1;
            if self.peek().is_some_and(|b| b == b'+' || b == b'-') {
                self.pos += 1;
            }
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        Ok(self.make_token(TokenKind::NumberLiteral, start))
    }

    fn lex_dollar_quote_or_param(&mut self, start: usize) -> Result<Token, LexerError> {
        // Check if it's a dollar-quoted string: $tag$...$tag$ or $$...$$
        let after_dollar = self.pos + 1;
        if after_dollar < self.bytes.len() {
            // $$ or $tag$
            if self.bytes[after_dollar] == b'$' {
                // $$...$$ form
                self.pos += 2; // skip $$
                let tag = "";
                return self.lex_dollar_body(start, tag);
            }
            if self.bytes[after_dollar].is_ascii_alphabetic() || self.bytes[after_dollar] == b'_' {
                // $tag$...$tag$ form
                let tag_start = after_dollar;
                let mut p = after_dollar;
                while p < self.bytes.len()
                    && (self.bytes[p].is_ascii_alphanumeric() || self.bytes[p] == b'_')
                {
                    p += 1;
                }
                if p < self.bytes.len() && self.bytes[p] == b'$' {
                    let tag = &self.source[tag_start..p];
                    self.pos = p + 1; // skip closing $
                    return self.lex_dollar_body(start, tag);
                }
            }
        }

        // Plain parameter: $1, $2, etc.
        self.pos += 1;
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1;
        }
        Ok(self.make_token(TokenKind::Placeholder, start))
    }

    fn lex_dollar_body(&mut self, start: usize, tag: &str) -> Result<Token, LexerError> {
        let end_tag = format!("${tag}$");
        let end_bytes = end_tag.as_bytes();
        while self.pos + end_bytes.len() <= self.bytes.len() {
            if &self.bytes[self.pos..self.pos + end_bytes.len()] == end_bytes {
                self.pos += end_bytes.len();
                return Ok(self.make_token(TokenKind::StringLiteral, start));
            }
            self.pos += 1;
        }
        // If we hit EOF without closing, treat as unterminated string
        Err(LexerError::UnterminatedString {
            offset: start as u32,
        })
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    /// Advance past word-like characters (ASCII alphanumeric, `_`, and non-ASCII alphanumeric).
    fn eat_word_chars(&mut self) {
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if is_word_continue(b) {
                self.pos += 1;
            } else if b >= 0x80 {
                let remaining = &self.source[self.pos..];
                if let Some(c) = remaining.chars().next() {
                    if c.is_alphanumeric() || c == '_' {
                        self.pos += c.len_utf8();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn make_token(&self, kind: TokenKind, start: usize) -> Token {
        let text = &self.source[start..self.pos];
        Token::new(
            kind,
            Span::new(start as u32, self.pos as u32),
            SmolStr::new(text),
        )
    }
}

fn is_word_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'#'
}

fn is_word_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'#'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(input: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(input, LexerConfig::ansi());
        lexer.tokenize().unwrap()
    }

    fn kinds(input: &str) -> Vec<TokenKind> {
        lex(input).into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn test_simple_select() {
        let tokens = lex("SELECT 1");
        assert_eq!(tokens.len(), 4); // SELECT, WS, 1, EOF
        assert_eq!(tokens[0].kind, TokenKind::Word);
        assert_eq!(tokens[0].text.as_str(), "SELECT");
        assert_eq!(tokens[1].kind, TokenKind::Whitespace);
        assert_eq!(tokens[2].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[2].text.as_str(), "1");
        assert_eq!(tokens[3].kind, TokenKind::Eof);
    }

    #[test]
    fn test_select_star() {
        let k = kinds("SELECT * FROM users;");
        assert_eq!(
            k,
            vec![
                TokenKind::Word,       // SELECT
                TokenKind::Whitespace, // ' '
                TokenKind::Star,       // *
                TokenKind::Whitespace, // ' '
                TokenKind::Word,       // FROM
                TokenKind::Whitespace, // ' '
                TokenKind::Word,       // users
                TokenKind::Semicolon,  // ;
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_string_literal() {
        let tokens = lex("'hello world'");
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].text.as_str(), "'hello world'");
    }

    #[test]
    fn test_escaped_string() {
        let tokens = lex("'it''s'");
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].text.as_str(), "'it''s'");
    }

    #[test]
    fn test_line_comment() {
        let tokens = lex("-- comment\nSELECT");
        assert_eq!(tokens[0].kind, TokenKind::LineComment);
        assert_eq!(tokens[0].text.as_str(), "-- comment");
        assert_eq!(tokens[1].kind, TokenKind::Newline);
        assert_eq!(tokens[2].kind, TokenKind::Word);
    }

    #[test]
    fn test_block_comment() {
        let tokens = lex("/* multi\nline */");
        assert_eq!(tokens[0].kind, TokenKind::BlockComment);
        assert_eq!(tokens[0].text.as_str(), "/* multi\nline */");
    }

    #[test]
    fn test_nested_block_comment() {
        let tokens = lex("/* outer /* inner */ end */");
        assert_eq!(tokens[0].kind, TokenKind::BlockComment);
    }

    #[test]
    fn test_operators() {
        let k = kinds("<= >= <> !=");
        assert_eq!(
            k,
            vec![
                TokenKind::LtEq,
                TokenKind::Whitespace,
                TokenKind::GtEq,
                TokenKind::Whitespace,
                TokenKind::Neq,
                TokenKind::Whitespace,
                TokenKind::Neq,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_number_formats() {
        let tokens = lex("42 3.14 .5 1e10 2.5E-3");
        let nums: Vec<&str> = tokens
            .iter()
            .filter(|t| t.kind == TokenKind::NumberLiteral)
            .map(|t| t.text.as_str())
            .collect();
        assert_eq!(nums, vec!["42", "3.14", ".5", "1e10", "2.5E-3"]);
    }

    #[test]
    fn test_quoted_identifier() {
        let tokens = lex("\"my column\"");
        assert_eq!(tokens[0].kind, TokenKind::QuotedIdentifier);
        assert_eq!(tokens[0].text.as_str(), "\"my column\"");
    }

    #[test]
    fn test_postgres_double_colon() {
        let mut lexer = Lexer::new("col::int", LexerConfig::postgres());
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[1].kind, TokenKind::ColonColon);
    }

    #[test]
    fn test_tsql_bracket_identifier() {
        let mut lexer = Lexer::new("[my col]", LexerConfig::tsql());
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::QuotedIdentifier);
        assert_eq!(tokens[0].text.as_str(), "[my col]");
    }

    #[test]
    fn test_newline_types() {
        let k = kinds("a\nb\r\nc");
        assert_eq!(
            k,
            vec![
                TokenKind::Word,
                TokenKind::Newline,
                TokenKind::Word,
                TokenKind::Newline,
                TokenKind::Word,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_placeholder() {
        let tokens = lex(":name ?");
        assert_eq!(tokens[0].kind, TokenKind::Placeholder);
        assert_eq!(tokens[0].text.as_str(), ":name");
        assert_eq!(tokens[2].kind, TokenKind::Placeholder);
        assert_eq!(tokens[2].text.as_str(), "?");
    }
}
