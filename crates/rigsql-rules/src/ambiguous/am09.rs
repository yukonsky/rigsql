use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AM09: LIMIT without ORDER BY gives non-deterministic results.
///
/// Using LIMIT without ORDER BY means the returned rows are unpredictable.
#[derive(Debug, Default)]
pub struct RuleAM09;

impl Rule for RuleAM09 {
    fn code(&self) -> &'static str {
        "AM09"
    }
    fn name(&self) -> &'static str {
        "ambiguous.order_by_limit"
    }
    fn description(&self) -> &'static str {
        "LIMIT without ORDER BY."
    }
    fn explanation(&self) -> &'static str {
        "Using LIMIT without ORDER BY produces non-deterministic results because the \
         database is free to return rows in any order. Always pair LIMIT with ORDER BY \
         to get predictable, reproducible result sets."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Ambiguous]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectStatement])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        let limit_clause = children
            .iter()
            .find(|c| c.segment_type() == SegmentType::LimitClause);

        let has_order_by = children
            .iter()
            .any(|c| c.segment_type() == SegmentType::OrderByClause);

        if let Some(limit) = limit_clause {
            if !has_order_by {
                return vec![LintViolation::with_msg_key(
                    self.code(),
                    "LIMIT without ORDER BY gives non-deterministic results.",
                    limit.span(),
                    "rules.AM09.msg",
                    vec![],
                )];
            }
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_am09_flags_limit_without_order_by() {
        let violations = lint_sql("SELECT a FROM t LIMIT 10", RuleAM09);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_am09_accepts_limit_with_order_by() {
        let violations = lint_sql("SELECT a FROM t ORDER BY a LIMIT 10", RuleAM09);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_am09_accepts_no_limit() {
        let violations = lint_sql("SELECT a FROM t", RuleAM09);
        assert_eq!(violations.len(), 0);
    }
}
