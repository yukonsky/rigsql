use rigsql_core::Segment;
use rigsql_lexer::{Lexer, LexerConfig, LexerError};
use thiserror::Error;

use crate::context::ParseContext;
use crate::grammar::Grammar;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Lexer error: {0}")]
    Lexer(#[from] LexerError),
}

/// High-level SQL parser: source text → CST.
pub struct Parser {
    lexer_config: LexerConfig,
}

impl Parser {
    pub fn new(lexer_config: LexerConfig) -> Self {
        Self { lexer_config }
    }

    /// Parse SQL source into a CST rooted at a File segment.
    pub fn parse(&self, source: &str) -> Result<Segment, ParseError> {
        let mut lexer = Lexer::new(source, self.lexer_config.clone());
        let tokens = lexer.tokenize()?;
        let mut ctx = ParseContext::new(&tokens, source);
        let file = Grammar::parse_file(&mut ctx);
        Ok(file)
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new(LexerConfig::ansi())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rigsql_core::SegmentType;

    fn parse(sql: &str) -> Segment {
        Parser::default().parse(sql).unwrap()
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
}
