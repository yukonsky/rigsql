use rigsql_core::Segment;
use rigsql_lexer::{Lexer, LexerConfig, LexerError};
use thiserror::Error;

use crate::context::{ParseContext, ParseDiagnostic};
use crate::grammar::{AnsiGrammar, Grammar};
#[cfg(test)]
use crate::grammar::{PostgresGrammar, TsqlGrammar};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Lexer error: {0}")]
    Lexer(#[from] LexerError),
}

/// Result of parsing: a CST (always produced) plus any diagnostics
/// collected during error-recovery passes.
pub struct ParseResult {
    /// The concrete syntax tree.  Always present — unparsable regions
    /// are wrapped in `SegmentType::Unparsable` nodes.
    pub tree: Segment,
    /// Diagnostics emitted by the parser when it encountered
    /// unrecognised tokens and had to skip ahead.
    pub diagnostics: Vec<ParseDiagnostic>,
}

/// High-level SQL parser: source text → CST.
pub struct Parser {
    lexer_config: LexerConfig,
    grammar: Box<dyn Grammar>,
}

impl Parser {
    pub fn new(lexer_config: LexerConfig, grammar: Box<dyn Grammar>) -> Self {
        Self {
            lexer_config,
            grammar,
        }
    }

    /// Parse SQL source into a CST rooted at a File segment.
    pub fn parse(&self, source: &str) -> Result<Segment, ParseError> {
        self.parse_with_diagnostics(source).map(|r| r.tree)
    }

    /// Parse SQL source, returning both the CST and any diagnostics
    /// produced during error recovery.
    pub fn parse_with_diagnostics(&self, source: &str) -> Result<ParseResult, ParseError> {
        let mut lexer = Lexer::new(source, self.lexer_config.clone());
        let tokens = lexer.tokenize()?;
        let mut ctx = ParseContext::new(&tokens, source);
        let tree = self.grammar.parse_file(&mut ctx);
        let diagnostics = ctx.take_diagnostics();
        Ok(ParseResult { tree, diagnostics })
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new(LexerConfig::ansi(), Box::new(AnsiGrammar))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rigsql_core::SegmentType;

    fn parse(sql: &str) -> Segment {
        Parser::default().parse(sql).unwrap()
    }

    fn parse_tsql(sql: &str) -> Segment {
        Parser::new(LexerConfig::tsql(), Box::new(TsqlGrammar))
            .parse(sql)
            .unwrap()
    }

    fn assert_type(seg: &Segment, expected: SegmentType) {
        assert_eq!(
            seg.segment_type(),
            expected,
            "Expected {:?} but got {:?} for raw: {:?}",
            expected,
            seg.segment_type(),
            seg.raw()
        );
    }

    fn find_type(seg: &Segment, ty: SegmentType) -> Option<&Segment> {
        let mut result = None;
        seg.walk(&mut |s| {
            if result.is_none() && s.segment_type() == ty {
                result = Some(s as *const Segment);
            }
        });
        result.map(|p| unsafe { &*p })
    }

    fn assert_no_unparsable(seg: &Segment) {
        let mut unparsable = Vec::new();
        seg.walk(&mut |s| {
            if s.segment_type() == SegmentType::Unparsable {
                unparsable.push(s.raw());
            }
        });
        assert!(
            unparsable.is_empty(),
            "Found Unparsable segments: {:?}",
            unparsable
        );
    }

    #[test]
    fn test_simple_select() {
        let cst = parse("SELECT 1");
        assert_type(&cst, SegmentType::File);
        let stmt = &cst.children()[0];
        assert_type(stmt, SegmentType::Statement);
        assert!(find_type(&cst, SegmentType::SelectClause).is_some());
    }

    #[test]
    fn test_select_from_where() {
        let cst = parse("SELECT name FROM users WHERE id = 1");
        assert!(find_type(&cst, SegmentType::SelectClause).is_some());
        assert!(find_type(&cst, SegmentType::FromClause).is_some());
        assert!(find_type(&cst, SegmentType::WhereClause).is_some());
    }

    #[test]
    fn test_join() {
        let cst = parse("SELECT a.id FROM a INNER JOIN b ON a.id = b.id");
        assert!(find_type(&cst, SegmentType::JoinClause).is_some());
        assert!(find_type(&cst, SegmentType::OnClause).is_some());
    }

    #[test]
    fn test_group_by_having_order_by() {
        let cst = parse(
            "SELECT dept, COUNT(*) FROM emp GROUP BY dept HAVING COUNT(*) > 5 ORDER BY dept ASC",
        );
        assert!(find_type(&cst, SegmentType::GroupByClause).is_some());
        assert!(find_type(&cst, SegmentType::HavingClause).is_some());
        assert!(find_type(&cst, SegmentType::OrderByClause).is_some());
    }

    #[test]
    fn test_insert_values() {
        let cst = parse("INSERT INTO users (name, email) VALUES ('Alice', 'a@b.com')");
        assert!(find_type(&cst, SegmentType::InsertStatement).is_some());
        assert!(find_type(&cst, SegmentType::ValuesClause).is_some());
    }

    #[test]
    fn test_update_set_where() {
        let cst = parse("UPDATE users SET name = 'Bob' WHERE id = 1");
        assert!(find_type(&cst, SegmentType::UpdateStatement).is_some());
        assert!(find_type(&cst, SegmentType::SetClause).is_some());
        assert!(find_type(&cst, SegmentType::WhereClause).is_some());
    }

    #[test]
    fn test_delete() {
        let cst = parse("DELETE FROM users WHERE id = 1");
        assert!(find_type(&cst, SegmentType::DeleteStatement).is_some());
    }

    #[test]
    fn test_create_table() {
        let cst = parse("CREATE TABLE users (id INT, name VARCHAR(100))");
        assert!(find_type(&cst, SegmentType::CreateTableStatement).is_some());
    }

    #[test]
    fn test_with_cte() {
        let cst =
            parse("WITH active AS (SELECT * FROM users WHERE active = TRUE) SELECT * FROM active");
        assert!(find_type(&cst, SegmentType::WithClause).is_some());
        assert!(find_type(&cst, SegmentType::CteDefinition).is_some());
    }

    #[test]
    fn test_case_expression() {
        let cst = parse("SELECT CASE WHEN x > 0 THEN 'pos' ELSE 'neg' END FROM t");
        assert!(find_type(&cst, SegmentType::CaseExpression).is_some());
        assert!(find_type(&cst, SegmentType::WhenClause).is_some());
        assert!(find_type(&cst, SegmentType::ElseClause).is_some());
    }

    #[test]
    fn test_subquery() {
        let cst = parse("SELECT * FROM (SELECT 1) AS sub");
        assert!(find_type(&cst, SegmentType::Subquery).is_some());
    }

    #[test]
    fn test_function_call() {
        let cst = parse("SELECT COUNT(*) FROM users");
        assert!(find_type(&cst, SegmentType::FunctionCall).is_some());
    }

    #[test]
    fn test_roundtrip() {
        let sql = "SELECT a, b FROM t WHERE x = 1 ORDER BY a;";
        let cst = parse(sql);
        assert_eq!(
            cst.raw(),
            sql,
            "CST roundtrip must preserve source text exactly"
        );
    }

    #[test]
    fn test_multiple_statements() {
        let sql = "SELECT 1; SELECT 2;";
        let cst = parse(sql);
        let stmts: Vec<_> = cst
            .children()
            .iter()
            .filter(|s| s.segment_type() == SegmentType::Statement)
            .collect();
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn test_roundtrip_complex() {
        let sql = "WITH cte AS (\n  SELECT id, name\n  FROM users\n  WHERE active = TRUE\n)\nSELECT cte.id, cte.name\nFROM cte\nINNER JOIN orders ON cte.id = orders.user_id\nWHERE orders.total > 100\nORDER BY cte.name ASC\nLIMIT 10;";
        let cst = parse(sql);
        assert_eq!(cst.raw(), sql);
    }

    // ── TSQL Tests ──────────────────────────────────────────────

    #[test]
    fn test_tsql_declare_variable() {
        let cst = parse_tsql("DECLARE @id INT;");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::DeclareStatement).is_some());
        assert_eq!(cst.raw(), "DECLARE @id INT;");
    }

    #[test]
    fn test_tsql_declare_with_default() {
        let cst = parse_tsql("DECLARE @name VARCHAR(100) = 'test';");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::DeclareStatement).is_some());
    }

    #[test]
    fn test_tsql_declare_multiple() {
        let cst = parse_tsql("DECLARE @a INT, @b VARCHAR(50);");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::DeclareStatement).is_some());
        assert_eq!(cst.raw(), "DECLARE @a INT, @b VARCHAR(50);");
    }

    #[test]
    fn test_tsql_declare_table_variable() {
        let cst = parse_tsql("DECLARE @t TABLE (id INT, name VARCHAR(100));");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::DeclareStatement).is_some());
    }

    #[test]
    fn test_tsql_declare_cursor() {
        let cst = parse_tsql("DECLARE cur CURSOR FOR SELECT id FROM users;");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::DeclareStatement).is_some());
        assert!(find_type(&cst, SegmentType::SelectStatement).is_some());
    }

    #[test]
    fn test_tsql_set_variable() {
        let cst = parse_tsql("SET @id = 42;");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::SetVariableStatement).is_some());
        assert_eq!(cst.raw(), "SET @id = 42;");
    }

    #[test]
    fn test_tsql_set_option() {
        let cst = parse_tsql("SET NOCOUNT ON;");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::SetVariableStatement).is_some());
    }

    #[test]
    fn test_tsql_if_else() {
        let sql = "IF @x > 0\n    SELECT 1;\nELSE\n    SELECT 2;";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::IfStatement).is_some());
        assert_eq!(cst.raw(), sql);
    }

    #[test]
    fn test_tsql_if_begin_end() {
        let sql = "IF @x > 0\nBEGIN\n    SELECT 1;\n    SELECT 2;\nEND";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::IfStatement).is_some());
        assert!(find_type(&cst, SegmentType::BeginEndBlock).is_some());
    }

    #[test]
    fn test_tsql_begin_end() {
        let sql = "BEGIN\n    SELECT 1;\n    SELECT 2;\nEND";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::BeginEndBlock).is_some());
    }

    #[test]
    fn test_tsql_while() {
        let sql = "WHILE @i < 10\nBEGIN\n    SET @i = @i + 1;\nEND";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::WhileStatement).is_some());
        assert!(find_type(&cst, SegmentType::BeginEndBlock).is_some());
    }

    #[test]
    fn test_tsql_try_catch() {
        let sql = "BEGIN TRY\n    SELECT 1;\nEND TRY\nBEGIN CATCH\n    SELECT 2;\nEND CATCH";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::TryCatchBlock).is_some());
        assert_eq!(cst.raw(), sql);
    }

    #[test]
    fn test_tsql_exec_simple() {
        let cst = parse_tsql("EXEC sp_help;");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::ExecStatement).is_some());
    }

    #[test]
    fn test_tsql_exec_with_params() {
        let cst = parse_tsql("EXEC dbo.usp_GetUser @id = 1, @name = 'test';");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::ExecStatement).is_some());
    }

    #[test]
    fn test_tsql_execute_keyword() {
        let cst = parse_tsql("EXECUTE sp_help;");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::ExecStatement).is_some());
    }

    #[test]
    fn test_tsql_return() {
        let cst = parse_tsql("RETURN 0;");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::ReturnStatement).is_some());
    }

    #[test]
    fn test_tsql_return_no_value() {
        let cst = parse_tsql("RETURN;");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::ReturnStatement).is_some());
    }

    #[test]
    fn test_tsql_print() {
        let cst = parse_tsql("PRINT 'hello';");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::PrintStatement).is_some());
    }

    #[test]
    fn test_tsql_throw() {
        let cst = parse_tsql("THROW 50000, 'Error occurred', 1;");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::ThrowStatement).is_some());
    }

    #[test]
    fn test_tsql_throw_rethrow() {
        let cst = parse_tsql("THROW;");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::ThrowStatement).is_some());
    }

    #[test]
    fn test_tsql_raiserror() {
        let cst = parse_tsql("RAISERROR('Error', 16, 1);");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::RaiserrorStatement).is_some());
    }

    #[test]
    fn test_tsql_raiserror_with_nowait() {
        let cst = parse_tsql("RAISERROR('Error', 16, 1) WITH NOWAIT;");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::RaiserrorStatement).is_some());
    }

    #[test]
    fn test_tsql_go() {
        let cst = parse_tsql("SELECT 1;\nGO");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::GoStatement).is_some());
    }

    #[test]
    fn test_tsql_go_with_count() {
        let cst = parse_tsql("GO 5");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::GoStatement).is_some());
    }

    #[test]
    fn test_tsql_simple_statements() {
        let sql = "USE master;";
        assert_single_statement(&parse_tsql(sql), sql, SegmentType::SimpleStatement);
    }

    #[test]
    fn test_tsql_roundtrip_complex() {
        let sql = "SET NOCOUNT ON;\nDECLARE @id INT = 1;\nIF @id > 0\nBEGIN\n    SELECT @id;\n    PRINT 'done';\nEND";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert_eq!(cst.raw(), sql);
    }

    #[test]
    fn test_tsql_nested_begin_end() {
        let sql = "BEGIN\n    BEGIN\n        SELECT 1;\n    END\nEND";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert_eq!(cst.raw(), sql);
    }

    #[test]
    fn test_tsql_if_else_begin_end() {
        let sql = "IF @x = 1\nBEGIN\n    SELECT 1;\nEND\nELSE\nBEGIN\n    SELECT 2;\nEND";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::IfStatement).is_some());
    }

    #[test]
    fn test_tsql_try_catch_with_throw() {
        let sql = "BEGIN TRY\n    SELECT 1;\nEND TRY\nBEGIN CATCH\n    THROW;\nEND CATCH";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::TryCatchBlock).is_some());
        assert!(find_type(&cst, SegmentType::ThrowStatement).is_some());
    }

    #[test]
    fn test_tsql_case_inside_begin_end() {
        let sql = "BEGIN\n    SELECT CASE WHEN @x > 0 THEN 'pos' ELSE 'neg' END;\nEND";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::BeginEndBlock).is_some());
        assert!(find_type(&cst, SegmentType::CaseExpression).is_some());
    }

    #[test]
    fn test_tsql_exec_retval() {
        let cst = parse_tsql("EXEC @result = dbo.usp_Calculate;");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::ExecStatement).is_some());
    }

    #[test]
    fn test_tsql_multiple_set_options() {
        let sql = "SET ANSI_NULLS ON;\nSET QUOTED_IDENTIFIER ON;";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert_eq!(cst.raw(), sql);
    }

    // ── TSQL Table Hint Tests ────────────────────────────────────

    #[test]
    fn test_tsql_with_nolock() {
        let sql = "SELECT * FROM orders WITH(NOLOCK) WHERE id = 1";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::TableHint).is_some());
        assert!(find_type(&cst, SegmentType::FromClause).is_some());
        assert!(find_type(&cst, SegmentType::WhereClause).is_some());
        assert_eq!(cst.raw(), sql);
    }

    #[test]
    fn test_tsql_with_nolock_alias() {
        let sql = "SELECT o.id FROM orders o WITH(NOLOCK)";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::TableHint).is_some());
        assert!(find_type(&cst, SegmentType::AliasExpression).is_some());
        assert_eq!(cst.raw(), sql);
    }

    #[test]
    fn test_tsql_with_nolock_join() {
        let sql = "SELECT a.id FROM orders a WITH(NOLOCK) INNER JOIN items b WITH(READUNCOMMITTED) ON a.id = b.order_id";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        // Two table hints
        let mut hint_count = 0;
        cst.walk(&mut |s| {
            if s.segment_type() == SegmentType::TableHint {
                hint_count += 1;
            }
        });
        assert_eq!(hint_count, 2);
        assert!(find_type(&cst, SegmentType::JoinClause).is_some());
        assert_eq!(cst.raw(), sql);
    }

    #[test]
    fn test_tsql_with_multiple_hints() {
        let sql = "SELECT * FROM orders WITH(NOLOCK, NOWAIT)";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::TableHint).is_some());
        assert_eq!(cst.raw(), sql);
    }

    #[test]
    fn test_tsql_with_nolock_roundtrip() {
        let sql = "SELECT o.id, o.total\nFROM orders o WITH(NOLOCK)\nINNER JOIN customers c WITH(NOLOCK) ON o.customer_id = c.id\nWHERE c.active = 1\nORDER BY o.id";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert_eq!(cst.raw(), sql);
    }

    // ── Ordered-set aggregates (WITHIN GROUP) ─────────────────────

    #[test]
    fn test_within_group_string_agg() {
        let sql = "SELECT STRING_AGG(s.name, ',') WITHIN GROUP (ORDER BY s.sort) AS names\nFROM sites AS s";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert_eq!(cst.raw(), sql);
        assert!(find_type(&cst, SegmentType::WithinGroupClause).is_some());
        // WITHIN must not be swallowed as an implicit column alias.
        assert!(find_type(&cst, SegmentType::FromClause).is_some());
    }

    #[test]
    fn test_within_group_multiline() {
        let sql = "SELECT STRING_AGG(s.name, ',')\n    WITHIN GROUP (ORDER BY s.sort, s.code DESC)\nFROM sites AS s";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert_eq!(cst.raw(), sql);
        assert!(find_type(&cst, SegmentType::WithinGroupClause).is_some());
    }

    #[test]
    fn test_within_group_with_over() {
        let sql = "SELECT PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY t.amount) OVER (PARTITION BY t.shop) AS median\nFROM sales AS t";
        let cst = parse_tsql(sql);
        assert_no_unparsable(&cst);
        assert_eq!(cst.raw(), sql);
        assert!(find_type(&cst, SegmentType::WithinGroupClause).is_some());
        assert!(find_type(&cst, SegmentType::OverClause).is_some());
    }

    #[test]
    fn test_within_group_ansi() {
        let sql = "SELECT LISTAGG(name, ',') WITHIN GROUP (ORDER BY sort) AS names FROM sites";
        let cst = parse(sql);
        assert_no_unparsable(&cst);
        assert_eq!(cst.raw(), sql);
        assert!(find_type(&cst, SegmentType::WithinGroupClause).is_some());
    }

    // ── Error Recovery Tests ──────────────────────────────────────

    fn count_unparsable(seg: &Segment) -> usize {
        let mut count = 0;
        seg.walk(&mut |s| {
            if s.segment_type() == SegmentType::Unparsable {
                count += 1;
            }
        });
        count
    }

    #[test]
    fn test_error_recovery_garbage_then_valid() {
        // Garbage tokens followed by a valid statement
        let sql = "XYZZY FOOBAR; SELECT 1;";
        let cst = parse(sql);
        assert_eq!(cst.raw(), sql, "roundtrip must preserve source");
        // The garbage should be in one Unparsable node
        assert_eq!(count_unparsable(&cst), 1);
        // The valid SELECT should still parse
        assert!(find_type(&cst, SegmentType::SelectClause).is_some());
    }

    #[test]
    fn test_error_recovery_garbage_between_statements() {
        // Valid, garbage, valid
        let sql = "SELECT 1; NOTAKEYWORD 123 'abc'; SELECT 2;";
        let cst = parse(sql);
        assert_eq!(cst.raw(), sql);
        assert_eq!(count_unparsable(&cst), 1);
        let stmts: Vec<_> = cst
            .children()
            .iter()
            .filter(|s| s.segment_type() == SegmentType::Statement)
            .collect();
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn test_error_recovery_garbage_at_end() {
        let sql = "SELECT 1; XYZZY";
        let cst = parse(sql);
        assert_eq!(cst.raw(), sql);
        assert_eq!(count_unparsable(&cst), 1);
        assert!(find_type(&cst, SegmentType::SelectClause).is_some());
    }

    #[test]
    fn test_error_recovery_skips_to_statement_keyword() {
        // Garbage followed directly by SELECT (no semicolon separator)
        let sql = "XYZZY SELECT 1;";
        let cst = parse(sql);
        assert_eq!(cst.raw(), sql);
        assert_eq!(count_unparsable(&cst), 1);
        assert!(find_type(&cst, SegmentType::SelectClause).is_some());
    }

    #[test]
    fn test_error_recovery_diagnostics() {
        let parser = Parser::default();
        let result = parser.parse_with_diagnostics("XYZZY; SELECT 1;").unwrap();
        assert!(!result.diagnostics.is_empty());
        assert!(result.diagnostics[0].message.contains("Unparsable"));
        // Offset should point to the start of the unparsable region (byte 0 = 'X')
        assert_eq!(result.diagnostics[0].offset, 0);
        // CST still produced
        assert!(find_type(&result.tree, SegmentType::SelectClause).is_some());
    }

    #[test]
    fn test_error_recovery_diagnostics_offset_mid_file() {
        let parser = Parser::default();
        // "SELECT 1; " = 10 bytes, then garbage starts
        let result = parser
            .parse_with_diagnostics("SELECT 1; BADTOKEN;")
            .unwrap();
        assert_eq!(result.diagnostics.len(), 1);
        // Offset should point to 'B' in BADTOKEN, not to ';' or beyond
        assert_eq!(result.diagnostics[0].offset, 10);
    }

    #[test]
    fn test_error_recovery_all_garbage() {
        let sql = "NOTAKEYWORD 123 'hello'";
        let cst = parse(sql);
        assert_eq!(cst.raw(), sql);
        // Everything should be unparsable but still present
        assert!(count_unparsable(&cst) >= 1);
    }

    #[test]
    fn test_error_recovery_preserves_valid_statements() {
        // Multiple valid statements with garbage in the middle
        let sql = "INSERT INTO t VALUES (1); BADTOKEN; DELETE FROM t WHERE id = 1;";
        let cst = parse(sql);
        assert_eq!(cst.raw(), sql);
        assert!(find_type(&cst, SegmentType::InsertStatement).is_some());
        assert!(find_type(&cst, SegmentType::DeleteStatement).is_some());
        assert_eq!(count_unparsable(&cst), 1);
    }

    // ── PostgreSQL tests ────────────────────────────────────────────

    fn parse_pg(sql: &str) -> Segment {
        Parser::new(LexerConfig::postgres(), Box::new(PostgresGrammar))
            .parse(sql)
            .unwrap()
    }

    #[test]
    fn test_pg_double_colon_cast() {
        let cst = parse_pg("SELECT col::int FROM t");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::TypeCastExpression).is_some());
        assert_eq!(cst.raw(), "SELECT col::int FROM t");
    }

    #[test]
    fn test_pg_chained_cast() {
        let cst = parse_pg("SELECT '2024-01-01'::date::text FROM t");
        assert_no_unparsable(&cst);
        // Two nested TypeCastExpression
        let mut count = 0;
        cst.walk(&mut |s| {
            if s.segment_type() == SegmentType::TypeCastExpression {
                count += 1;
            }
        });
        assert_eq!(
            count, 2,
            "Expected 2 TypeCastExpression nodes for chained cast"
        );
    }

    #[test]
    fn test_pg_cast_with_precision() {
        let cst = parse_pg("SELECT col::numeric(10, 2) FROM t");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::TypeCastExpression).is_some());
        assert!(find_type(&cst, SegmentType::DataType).is_some());
    }

    #[test]
    fn test_pg_array_subscript() {
        let cst = parse_pg("SELECT arr[1] FROM t");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::ArrayAccessExpression).is_some());
    }

    #[test]
    fn test_pg_array_cast_chain() {
        let cst = parse_pg("SELECT arr[1]::text FROM t");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::ArrayAccessExpression).is_some());
        assert!(find_type(&cst, SegmentType::TypeCastExpression).is_some());
    }

    #[test]
    fn test_pg_insert_returning() {
        let cst = parse_pg("INSERT INTO users (name) VALUES ('Alice') RETURNING id, name");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::InsertStatement).is_some());
        assert!(find_type(&cst, SegmentType::ReturningClause).is_some());
    }

    #[test]
    fn test_pg_update_returning() {
        let cst = parse_pg("UPDATE users SET name = 'Bob' WHERE id = 1 RETURNING *");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::UpdateStatement).is_some());
        assert!(find_type(&cst, SegmentType::ReturningClause).is_some());
    }

    #[test]
    fn test_pg_delete_returning() {
        let cst = parse_pg("DELETE FROM users WHERE id = 1 RETURNING id");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::DeleteStatement).is_some());
        assert!(find_type(&cst, SegmentType::ReturningClause).is_some());
    }

    #[test]
    fn test_pg_on_conflict_do_nothing() {
        let cst = parse_pg(
            "INSERT INTO users (id, name) VALUES (1, 'Alice') ON CONFLICT (id) DO NOTHING",
        );
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::OnConflictClause).is_some());
    }

    #[test]
    fn test_pg_on_conflict_do_update() {
        let cst = parse_pg(
            "INSERT INTO users (id, name) VALUES (1, 'Alice') \
             ON CONFLICT (id) DO UPDATE SET name = 'Alice'",
        );
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::OnConflictClause).is_some());
        assert!(find_type(&cst, SegmentType::SetClause).is_some());
    }

    #[test]
    fn test_pg_upsert_returning() {
        let cst = parse_pg(
            "INSERT INTO users (id, name) VALUES (1, 'Alice') \
             ON CONFLICT (id) DO UPDATE SET name = 'Alice' RETURNING *",
        );
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::OnConflictClause).is_some());
        assert!(find_type(&cst, SegmentType::ReturningClause).is_some());
    }

    #[test]
    fn test_pg_distinct_on() {
        let cst = parse_pg(
            "SELECT DISTINCT ON (dept) name, salary FROM employees ORDER BY dept, salary DESC",
        );
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::SelectClause).is_some());
        assert!(find_type(&cst, SegmentType::OrderByClause).is_some());
    }

    #[test]
    fn test_pg_dollar_quoted_string() {
        let cst = parse_pg("SELECT $$hello world$$");
        assert_no_unparsable(&cst);
        assert_eq!(cst.raw(), "SELECT $$hello world$$");
    }

    #[test]
    fn test_pg_ilike() {
        let cst = parse_pg("SELECT * FROM users WHERE name ILIKE '%alice%'");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::LikeExpression).is_some());
    }

    #[test]
    fn test_pg_roundtrip_complex() {
        let sql = "INSERT INTO orders (user_id, total) \
                   VALUES (1, 99.99) \
                   ON CONFLICT (user_id) DO UPDATE SET total = orders.total + 99.99 \
                   RETURNING id, total::numeric(10, 2)";
        let cst = parse_pg(sql);
        assert_eq!(
            cst.raw(),
            sql,
            "CST roundtrip must preserve source text exactly"
        );
        assert_no_unparsable(&cst);
    }

    // ── GRANT / REVOKE / DENY ────────────────────────────────────

    fn assert_single_grant(cst: &Segment, sql: &str) {
        assert_single_statement(cst, sql, SegmentType::GrantStatement);
        assert!(find_type(cst, SegmentType::SelectStatement).is_none());
    }

    #[test]
    fn test_tsql_grant_on_object() {
        let sql = "GRANT SELECT ON OBJECT::dbo.ExampleTable TO [example_user];";
        assert_single_grant(&parse_tsql(sql), sql);
    }

    #[test]
    fn test_tsql_grant_multiple_permissions() {
        let sql = "GRANT INSERT, SELECT, UPDATE, EXECUTE ON SCHEMA::Sales TO app_user, [reporting] WITH GRANT OPTION AS dbo;";
        assert_single_grant(&parse_tsql(sql), sql);
    }

    #[test]
    fn test_tsql_grant_column_list() {
        let sql = "GRANT SELECT (id, [name]) ON dbo.users TO app_user";
        assert_single_grant(&parse_tsql(sql), sql);
    }

    #[test]
    fn test_tsql_deny_and_revoke() {
        let sql = "DENY DELETE ON OBJECT::dbo.t TO app_user;";
        assert_single_grant(&parse_tsql(sql), sql);
        let sql = "REVOKE GRANT OPTION FOR SELECT ON dbo.t FROM app_user CASCADE;";
        assert_single_grant(&parse_tsql(sql), sql);
    }

    #[test]
    fn test_pg_grant_on_all_tables() {
        let sql = "GRANT SELECT, INSERT ON ALL TABLES IN SCHEMA public TO reader";
        assert_single_grant(&parse_pg(sql), sql);
    }

    #[test]
    fn test_pg_grant_role_with_admin_option() {
        let sql = "GRANT admin TO bob WITH ADMIN OPTION GRANTED BY alice";
        assert_single_grant(&parse_pg(sql), sql);
    }

    #[test]
    fn test_grant_without_semicolon_does_not_swallow_next_cte() {
        let cst =
            parse_tsql("GRANT SELECT ON dbo.t TO u\nWITH cte AS (SELECT 1 AS x) SELECT x FROM cte");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::GrantStatement).is_some());
        assert!(find_type(&cst, SegmentType::WithClause).is_some());
    }

    #[test]
    fn test_pg_grant_quoted_role() {
        let sql = "GRANT \"read-only\" TO app_user;";
        assert_single_grant(&parse_pg(sql), sql);
    }

    #[test]
    fn test_pg_grant_to_group() {
        let sql = "GRANT SELECT ON t TO GROUP reporting, app_user;";
        let cst = parse_pg(sql);
        assert_single_grant(&cst, sql);
        let mut idents = Vec::new();
        cst.walk(&mut |s| {
            if s.segment_type() == SegmentType::Identifier {
                idents.push(s.raw());
            }
        });
        assert!(idents.contains(&"reporting".to_string()), "{idents:?}");
        assert!(!idents.contains(&"GROUP".to_string()), "{idents:?}");
    }

    #[test]
    fn test_pg_grant_on_keyword_object_name() {
        // UPDATE is non-reserved in PostgreSQL, so it can name a table / schema.
        for sql in [
            "GRANT SELECT ON update TO app_user;",
            "GRANT SELECT ON TABLE update TO app_user;",
            "GRANT USAGE ON SCHEMA update TO app_user;",
            "REVOKE SELECT ON update FROM app_user;",
            "GRANT SELECT ON t, update TO app_user;",
            // A class word used as the object name itself.
            "GRANT SELECT ON type TO app_user;",
        ] {
            assert_single_grant(&parse_pg(sql), sql);
        }
    }

    #[test]
    fn test_grant_on_without_to_stops_at_next_statement() {
        let cst = parse_pg("GRANT SELECT ON t\nUPDATE t SET a = 1");
        assert!(find_type(&cst, SegmentType::GrantStatement).is_some());
        assert!(find_type(&cst, SegmentType::UpdateStatement).is_some());
    }

    // ── TRUNCATE / MERGE / simple statements ─────────────────────

    /// Exactly one top-level Statement, containing `inner`, with no nested
    /// Statement node (CV06 would flag the inner one as unterminated).
    fn assert_single_statement(cst: &Segment, sql: &str, inner: SegmentType) {
        assert_eq!(cst.raw(), sql);
        assert_no_unparsable(cst);
        assert!(
            find_type(cst, inner).is_some(),
            "missing {inner:?} in {sql}"
        );
        let mut statements = 0;
        cst.walk(&mut |s| {
            if s.segment_type() == SegmentType::Statement {
                statements += 1;
            }
        });
        assert_eq!(statements, 1, "expected one Statement node in {sql}");
    }

    #[test]
    fn test_truncate_keyword_named_tables() {
        for sql in [
            "TRUNCATE update;",
            "TRUNCATE TABLE update;",
            "TRUNCATE s.update;",
            "TRUNCATE a, update;",
            "TRUNCATE TABLE ONLY s.orders, items RESTART IDENTITY CASCADE;",
        ] {
            assert_single_statement(&parse_pg(sql), sql, SegmentType::TruncateStatement);
        }
    }

    #[test]
    fn test_tsql_truncate_with_partitions() {
        let sql = "TRUNCATE TABLE dbo.t WITH (PARTITIONS (2, 4 TO 6));";
        assert_single_statement(&parse_tsql(sql), sql, SegmentType::TruncateStatement);
    }

    #[test]
    fn test_truncate_without_semicolon_stops_at_next_statement() {
        let cst = parse_pg("TRUNCATE t\nUPDATE t SET a = 1");
        assert!(find_type(&cst, SegmentType::TruncateStatement).is_some());
        assert!(find_type(&cst, SegmentType::UpdateStatement).is_some());
    }

    #[test]
    fn test_merge_with_update_insert_delete_clauses() {
        let sql = "MERGE INTO t USING s ON t.id = s.id \
                   WHEN MATCHED AND s.del = 1 THEN DELETE \
                   WHEN MATCHED THEN UPDATE SET a = s.a \
                   WHEN NOT MATCHED THEN INSERT (id, a) VALUES (s.id, s.a);";
        assert_single_statement(&parse(sql), sql, SegmentType::MergeStatement);
        assert_single_statement(&parse_pg(sql), sql, SegmentType::MergeStatement);
        assert_single_statement(&parse_tsql(sql), sql, SegmentType::MergeStatement);
    }

    #[test]
    fn test_simple_statements_are_not_nested_statements() {
        for (sql, inner) in [
            ("USE mydb;", SegmentType::SimpleStatement),
            ("CREATE INDEX idx_a ON t (a);", SegmentType::CreateStatement),
        ] {
            assert_single_statement(&parse(sql), sql, inner);
        }
    }

    #[test]
    fn test_simple_statement_operand_named_like_a_keyword() {
        for sql in [
            "USE update;",
            "OPEN update;",
            "CLOSE update;",
            "DEALLOCATE update;",
            "FETCH NEXT FROM update;",
            "FETCH NEXT IN update;",
            // Optional modifiers before the name keep it a name.
            "DEALLOCATE PREPARE update;",
            "CLOSE GLOBAL update;",
            "LOCK update;",
            "LOCK TABLE update IN ACCESS EXCLUSIVE MODE;",
            "LOCK TABLE ONLY s.update, update IN SHARE MODE;",
        ] {
            assert_single_statement(&parse_pg(sql), sql, SegmentType::SimpleStatement);
        }
    }

    #[test]
    fn test_operandless_simple_statement_stops_at_next_statement() {
        let cst = parse_tsql("WHILE 1 = 1\nBEGIN\n    BREAK\n    SELECT 1;\nEND");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::SimpleStatement).is_some());
        assert!(find_type(&cst, SegmentType::SelectStatement).is_some());
    }

    #[test]
    fn test_error_recovery_keeps_parens_and_dotted_names_together() {
        // Unknown statements go through error recovery; its boundary check
        // must not split `s.update` or stop at a SELECT inside parentheses.
        let cst = parse_pg(
            "COMMENT ON TABLE update IS 'x';\nANALYZE s.update (a, (SELECT 1));\nSELECT 1;",
        );
        let unparsable: Vec<_> = cst
            .children()
            .iter()
            .filter(|s| s.segment_type() == SegmentType::Unparsable)
            .map(|s| s.raw())
            .collect();
        assert_eq!(
            unparsable,
            [
                "COMMENT ON TABLE update IS 'x';",
                "ANALYZE s.update (a, (SELECT 1));"
            ]
        );
        assert!(find_type(&cst, SegmentType::SelectStatement).is_some());
    }

    #[test]
    fn test_statement_keyword_after_separator_is_not_a_boundary() {
        // No statement can start right after `,` or `.`.
        let sql = "FETCH NEXT FROM cur INTO a, update;";
        assert_single_statement(&parse(sql), sql, SegmentType::SimpleStatement);
    }

    #[test]
    fn test_grant_unrecognised_tail_stays_in_statement() {
        let sql = "GRANT SELECT ON t TO app_user UNKNOWN_OPTION x;";
        assert_single_grant(&parse_pg(sql), sql);
    }

    #[test]
    fn test_pg_grant_role_with_inherit_false() {
        let sql = "GRANT admin TO bob WITH INHERIT FALSE";
        assert_single_grant(&parse_pg(sql), sql);
    }

    #[test]
    fn test_incomplete_grant_does_not_swallow_next_statement() {
        let cst = parse_tsql("GRANT CONNECT\nSELECT a FROM t");
        assert!(find_type(&cst, SegmentType::GrantStatement).is_some());
        assert!(find_type(&cst, SegmentType::SelectStatement).is_some());
    }

    #[test]
    fn test_grant_without_semicolon_stops_at_next_statement() {
        let cst = parse_tsql("GRANT SELECT ON dbo.t TO app_user\nSELECT 1");
        assert_no_unparsable(&cst);
        assert!(find_type(&cst, SegmentType::GrantStatement).is_some());
        assert!(find_type(&cst, SegmentType::SelectStatement).is_some());
    }
}
