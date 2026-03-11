use rigsql_core::Segment;
use rigsql_lexer::{Lexer, LexerConfig, LexerError};
use thiserror::Error;

use crate::context::{ParseContext, ParseDiagnostic};
#[cfg(test)]
use crate::grammar::TsqlGrammar;
use crate::grammar::{AnsiGrammar, Grammar};

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
        let cst = parse_tsql("USE master;");
        assert_no_unparsable(&cst);
        assert_eq!(cst.raw(), "USE master;");
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
}
