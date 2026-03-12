use rigsql_core::{Segment, SegmentType, TokenKind};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// RF02: Column references should be qualified when multiple tables are present.
///
/// When a SELECT statement references multiple tables (via FROM + JOINs),
/// all column references should be qualified with a table alias or name
/// to avoid ambiguity.
#[derive(Debug, Default)]
pub struct RuleRF02;

impl Rule for RuleRF02 {
    fn code(&self) -> &'static str {
        "RF02"
    }
    fn name(&self) -> &'static str {
        "references.qualification"
    }
    fn description(&self) -> &'static str {
        "Columns should be qualified when multiple tables are referenced."
    }
    fn explanation(&self) -> &'static str {
        "When a query references multiple tables (via FROM and JOIN clauses), \
         all column references should be qualified with a table name or alias \
         (e.g., 'users.id' instead of 'id') to prevent ambiguity and improve readability."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::References]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectStatement])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let table_count = count_tables(ctx.segment);

        if table_count < 2 {
            return vec![];
        }

        // Find unqualified column references across all relevant clauses
        let mut violations = Vec::new();
        collect_unqualified_columns(ctx.segment, &mut violations, self.code(), false);
        violations
    }
}

/// Count tables referenced in FROM and JOIN clauses.
fn count_tables(stmt: &Segment) -> usize {
    let mut count = 0;
    for child in stmt.children() {
        if child.segment_type() == SegmentType::FromClause {
            count += count_tables_in_clause(child);
        }
    }
    count
}

fn count_tables_in_clause(clause: &Segment) -> usize {
    let mut count = 0;
    for child in clause.children() {
        match child.segment_type() {
            SegmentType::Identifier
            | SegmentType::QuotedIdentifier
            | SegmentType::AliasExpression => {
                count += 1;
            }
            SegmentType::QualifiedIdentifier => {
                // e.g., schema.table — counts as one table
                count += 1;
            }
            SegmentType::JoinClause => {
                for join_child in child.children() {
                    match join_child.segment_type() {
                        SegmentType::Identifier
                        | SegmentType::QuotedIdentifier
                        | SegmentType::AliasExpression
                        | SegmentType::QualifiedIdentifier => {
                            count += 1;
                            break;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    count
}

/// Contexts where bare Identifiers are likely column references.
const COLUMN_CONTEXTS: &[SegmentType] = &[
    SegmentType::SelectClause,
    SegmentType::WhereClause,
    SegmentType::HavingClause,
    SegmentType::OrderByClause,
    SegmentType::GroupByClause,
    SegmentType::OnClause,
    SegmentType::OrderByExpression,
    SegmentType::BinaryExpression,
];

/// Segment types that represent table sources, not column references.
const TABLE_SOURCE_CONTEXTS: &[SegmentType] = &[SegmentType::FromClause, SegmentType::JoinClause];

/// Recursively find unqualified column references in column-relevant clauses.
fn collect_unqualified_columns(
    segment: &Segment,
    violations: &mut Vec<LintViolation>,
    code: &'static str,
    in_table_source: bool,
) {
    // Skip subqueries to avoid cross-scope analysis
    if segment.segment_type() == SegmentType::Subquery {
        return;
    }

    let st = segment.segment_type();
    let is_table_source = in_table_source || TABLE_SOURCE_CONTEXTS.contains(&st);

    // QualifiedIdentifier / ColumnRef in table sources are table names, skip them
    match st {
        SegmentType::QualifiedIdentifier | SegmentType::ColumnRef => {
            if is_table_source {
                return;
            }
            // In column context: qualified refs are fine, only unqualified are violations
            let has_dot = segment
                .children()
                .iter()
                .any(|c| c.segment_type() == SegmentType::Dot);
            if !has_dot {
                // Unqualified column ref
                if let Some(Segment::Token(t)) = segment
                    .children()
                    .iter()
                    .find(|c| c.segment_type() == SegmentType::Identifier)
                {
                    // Skip TSQL variables (@var)
                    if t.token.kind == TokenKind::AtSign {
                        return;
                    }
                    violations.push(LintViolation::with_msg_key(
                        code,
                        format!(
                            "Unqualified column reference '{}' in multi-table query.",
                            t.token.text
                        ),
                        t.token.span,
                        "rules.RF02.msg",
                        vec![("name".to_string(), t.token.text.to_string())],
                    ));
                }
            }
            return;
        }
        _ => {}
    }

    // In column-relevant contexts, bare Identifiers are likely column references
    if COLUMN_CONTEXTS.contains(&st) {
        for child in segment.children() {
            if child.segment_type() == SegmentType::Identifier {
                if let Segment::Token(t) = child {
                    // Skip TSQL variables (@var) — they're not column references
                    if t.token.kind != TokenKind::AtSign {
                        violations.push(LintViolation::with_msg_key(
                            code,
                            format!(
                                "Unqualified column reference '{}' in multi-table query.",
                                t.token.text
                            ),
                            t.token.span,
                            "rules.RF02.msg",
                            vec![("name".to_string(), t.token.text.to_string())],
                        ));
                    }
                }
            } else {
                collect_unqualified_columns(child, violations, code, is_table_source);
            }
        }
        return;
    }

    for child in segment.children() {
        collect_unqualified_columns(child, violations, code, is_table_source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_rf02_flags_unqualified_in_multi_table() {
        let violations = lint_sql(
            "SELECT id FROM users JOIN orders ON users.id = orders.user_id",
            RuleRF02,
        );
        assert!(!violations.is_empty(), "Should flag unqualified 'id'");
        assert!(violations[0].message.contains("id"));
    }

    #[test]
    fn test_rf02_accepts_qualified_in_multi_table() {
        let violations = lint_sql(
            "SELECT u.id FROM users u JOIN orders o ON u.id = o.user_id",
            RuleRF02,
        );
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_rf02_accepts_single_table() {
        let violations = lint_sql("SELECT id FROM users", RuleRF02);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_rf02_flags_unqualified_in_where() {
        let violations = lint_sql(
            "SELECT u.id FROM users u JOIN orders o ON u.id = o.user_id WHERE status = 1",
            RuleRF02,
        );
        assert!(
            !violations.is_empty(),
            "Should flag unqualified 'status' in WHERE"
        );
    }

    #[test]
    fn test_rf02_ignores_qualified_table_in_from() {
        // sys.columns is a table name, not a column ref
        let violations = lint_sql("SELECT name FROM sys.columns WHERE object_id = 1", RuleRF02);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_rf02_ignores_tsql_variables() {
        // @SiteName is a TSQL variable, not a column reference
        let violations = lint_sql(
            "SELECT t1.a FROM t1 JOIN t2 ON t1.id = t2.id WHERE t1.x = @SiteName",
            RuleRF02,
        );
        assert_eq!(violations.len(), 0);
    }
}
