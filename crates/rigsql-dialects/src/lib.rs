use rigsql_lexer::LexerConfig;
use rigsql_parser::Parser;
use strum::{Display, EnumString};

/// Supported SQL dialects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum DialectKind {
    #[strum(serialize = "ansi")]
    Ansi,
    #[strum(serialize = "postgres", serialize = "postgresql")]
    Postgres,
    #[strum(serialize = "tsql", serialize = "sqlserver")]
    Tsql,
}

impl DialectKind {
    /// Create a parser configured for this dialect.
    pub fn parser(self) -> Parser {
        Parser::new(self.lexer_config())
    }

    /// Get the lexer configuration for this dialect.
    pub fn lexer_config(self) -> LexerConfig {
        match self {
            DialectKind::Ansi => LexerConfig::ansi(),
            DialectKind::Postgres => LexerConfig::postgres(),
            DialectKind::Tsql => LexerConfig::tsql(),
        }
    }
}

impl Default for DialectKind {
    fn default() -> Self {
        DialectKind::Ansi
    }
}
