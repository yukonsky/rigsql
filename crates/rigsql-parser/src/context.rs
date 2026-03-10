use rigsql_core::{Token, TokenKind};

/// Parser context: a cursor over the token stream.
pub struct ParseContext<'a> {
    tokens: &'a [Token],
    pos: usize,
    source: &'a str,
}

impl<'a> ParseContext<'a> {
    pub fn new(tokens: &'a [Token], source: &'a str) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
        }
    }

    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Current position in the token stream.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Save cursor position for backtracking.
    pub fn save(&self) -> usize {
        self.pos
    }

    /// Restore cursor to a saved position.
    pub fn restore(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Peek at the current token without consuming.
    pub fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    /// Peek at the current non-trivia token kind.
    pub fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|t| t.kind)
    }

    /// Peek at the next non-trivia token (skipping whitespace/comments).
    pub fn peek_non_trivia(&self) -> Option<&'a Token> {
        let mut i = self.pos;
        while i < self.tokens.len() {
            if !self.tokens[i].kind.is_trivia() {
                return Some(&self.tokens[i]);
            }
            i += 1;
        }
        None
    }

    /// Check if the next non-trivia token is a keyword matching `kw` (case-insensitive).
    pub fn peek_keyword(&self, kw: &str) -> bool {
        self.peek_non_trivia()
            .is_some_and(|t| t.kind == TokenKind::Word && t.text.eq_ignore_ascii_case(kw))
    }

    /// Check if next non-trivia tokens form a keyword sequence (e.g. "GROUP", "BY").
    pub fn peek_keywords(&self, kws: &[&str]) -> bool {
        let mut i = self.pos;
        for kw in kws {
            // skip trivia
            while i < self.tokens.len() && self.tokens[i].kind.is_trivia() {
                i += 1;
            }
            if i >= self.tokens.len() {
                return false;
            }
            let t = &self.tokens[i];
            if t.kind != TokenKind::Word || !t.text.eq_ignore_ascii_case(kw) {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Consume and return the current token, advancing the cursor.
    pub fn advance(&mut self) -> Option<&'a Token> {
        if self.pos < self.tokens.len() {
            let token = &self.tokens[self.pos];
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }

    /// Consume all leading trivia tokens and return them.
    pub fn eat_trivia(&mut self) -> Vec<&'a Token> {
        let mut trivia = Vec::new();
        while self.pos < self.tokens.len() && self.tokens[self.pos].kind.is_trivia() {
            trivia.push(&self.tokens[self.pos]);
            self.pos += 1;
        }
        trivia
    }

    /// Try to consume a keyword (case-insensitive). Returns the token if matched.
    pub fn eat_keyword(&mut self, kw: &str) -> Option<&'a Token> {
        if self.pos < self.tokens.len()
            && self.tokens[self.pos].kind == TokenKind::Word
            && self.tokens[self.pos].text.eq_ignore_ascii_case(kw)
        {
            let token = &self.tokens[self.pos];
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }

    /// Try to consume a specific token kind.
    pub fn eat_kind(&mut self, kind: TokenKind) -> Option<&'a Token> {
        if self.pos < self.tokens.len() && self.tokens[self.pos].kind == kind {
            let token = &self.tokens[self.pos];
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }

    /// Are we at EOF?
    pub fn at_eof(&self) -> bool {
        self.pos >= self.tokens.len() || self.tokens[self.pos].kind == TokenKind::Eof
    }

    /// Remaining tokens from current position.
    pub fn remaining(&self) -> &'a [Token] {
        &self.tokens[self.pos..]
    }
}
