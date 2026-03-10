use rigsql_core::Segment;

use crate::context::ParseContext;

use super::Grammar;

/// ANSI SQL grammar — parses standard SQL statements only.
pub struct AnsiGrammar;

const ANSI_STATEMENT_KEYWORDS: &[&str] = &[
    "ALTER",
    "BREAK",
    "CLOSE",
    "CONTINUE",
    "CREATE",
    "DEALLOCATE",
    "DELETE",
    "DROP",
    "ELSE",
    "END",
    "FETCH",
    "INSERT",
    "MERGE",
    "OPEN",
    "SELECT",
    "TRUNCATE",
    "UPDATE",
    "USE",
    "WITH",
];

impl Grammar for AnsiGrammar {
    fn statement_keywords(&self) -> &[&str] {
        ANSI_STATEMENT_KEYWORDS
    }

    fn dispatch_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        self.dispatch_ansi_statement(ctx)
    }
}
