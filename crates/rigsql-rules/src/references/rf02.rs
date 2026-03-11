use rigsql_core::{Segment, SegmentType};

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

        // Find unqualified column references in the SelectClause
        let mut violations = Vec::new();
        find_unqualified_select_columns(ctx.segment, &mut violations, self.code());
        violations
    }
}

/// Count tables referenced in FROM and JOIN clauses.
///
/// The parser emits tables as direct Identifier/AliasExpression children of
/// FromClause (for the main table) and JoinClause (for joined tables).
/// JoinClause is nested inside FromClause.
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
            SegmentType::JoinClause => {
                // The join clause has its own table reference
                for join_child in child.children() {
                    match join_child.segment_type() {
                        SegmentType::Identifier
                        | SegmentType::QuotedIdentifier
                        | SegmentType::AliasExpression => {
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

/// Find unqualified column references in the SelectClause.
///
/// In the parser's CST, unqualified column references in SELECT are bare
/// Identifier tokens (not wrapped in ColumnRef). Qualified references use
/// ColumnRef with Identifier.Dot.Identifier children.
fn find_unqualified_select_columns(
    stmt: &Segment,
    violations: &mut Vec<LintViolation>,
    code: &'static str,
) {
    for child in stmt.children() {
        if child.segment_type() == SegmentType::SelectClause {
            for sel_child in child.children() {
                // A bare Identifier in the SelectClause that is not a keyword is an
                // unqualified column reference
                if sel_child.segment_type() == SegmentType::Identifier {
                    if let Segment::Token(t) = sel_child {
                        violations.push(LintViolation::new(
                            code,
                            format!(
                                "Unqualified column reference '{}' in multi-table query.",
                                t.token.text
                            ),
                            t.token.span,
                        ));
                    }
                }
            }
        }
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
}
