use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// RF03: Column qualification should be consistent (all qualified or all unqualified).
///
/// In a SELECT statement, if some column references are qualified and some are not,
/// flag the inconsistency.
#[derive(Debug, Default)]
pub struct RuleRF03;

impl Rule for RuleRF03 {
    fn code(&self) -> &'static str {
        "RF03"
    }
    fn name(&self) -> &'static str {
        "references.consistent"
    }
    fn description(&self) -> &'static str {
        "Column qualification should be consistent."
    }
    fn explanation(&self) -> &'static str {
        "Within a single SELECT statement, column references should be consistently \
         qualified or unqualified. Mixing styles (e.g., 'users.id' alongside bare 'name') \
         reduces readability and can indicate accidental omissions."
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
        let mut qualified_count = 0usize;
        let mut unqualified: Vec<rigsql_core::Span> = Vec::new();

        // Look at the SelectClause children to find column references
        for child in ctx.segment.children() {
            if child.segment_type() == SegmentType::SelectClause {
                for sel_child in child.children() {
                    match sel_child.segment_type() {
                        // Bare Identifier = unqualified column reference
                        SegmentType::Identifier => {
                            if let Segment::Token(t) = sel_child {
                                unqualified.push(t.token.span);
                            }
                        }
                        // ColumnRef = qualified column reference (e.g., u.id)
                        SegmentType::ColumnRef => {
                            qualified_count += 1;
                        }
                        // AliasExpression may contain either
                        SegmentType::AliasExpression => {
                            // Check the first non-trivia child of the alias expression
                            for alias_child in sel_child.children() {
                                let st = alias_child.segment_type();
                                if st.is_trivia()
                                    || st == SegmentType::Keyword
                                    || st == SegmentType::Comma
                                {
                                    continue;
                                }
                                if st == SegmentType::ColumnRef {
                                    qualified_count += 1;
                                } else if st == SegmentType::Identifier {
                                    if let Segment::Token(t) = alias_child {
                                        unqualified.push(t.token.span);
                                    }
                                }
                                break; // Only check the first meaningful child
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Only flag if there is a mix of both styles
        if qualified_count == 0 || unqualified.is_empty() {
            return vec![];
        }

        unqualified
            .iter()
            .map(|span| {
                LintViolation::with_msg_key(
                    self.code(),
                    "Inconsistent column qualification. Mix of qualified and unqualified references."
                        .to_string(),
                    *span,
                    "rules.RF03.msg",
                    vec![],
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_rf03_flags_inconsistent_qualification() {
        let violations = lint_sql(
            "SELECT u.id, name FROM users u JOIN orders o ON u.id = o.user_id",
            RuleRF03,
        );
        assert!(
            !violations.is_empty(),
            "Should flag inconsistent references"
        );
    }

    #[test]
    fn test_rf03_accepts_all_qualified() {
        let violations = lint_sql(
            "SELECT u.id, u.name FROM users u JOIN orders o ON u.id = o.user_id",
            RuleRF03,
        );
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_rf03_accepts_all_unqualified() {
        let violations = lint_sql("SELECT id, name FROM users", RuleRF03);
        assert_eq!(violations.len(), 0);
    }
}
