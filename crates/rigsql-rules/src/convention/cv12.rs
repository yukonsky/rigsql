use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// CV12: Use of HAVING without GROUP BY.
///
/// A HAVING clause without a corresponding GROUP BY is likely a mistake;
/// use WHERE instead, or add the missing GROUP BY.
#[derive(Debug, Default)]
pub struct RuleCV12;

impl Rule for RuleCV12 {
    fn code(&self) -> &'static str {
        "CV12"
    }
    fn name(&self) -> &'static str {
        "convention.having_without_group_by"
    }
    fn description(&self) -> &'static str {
        "Use of HAVING without GROUP BY."
    }
    fn explanation(&self) -> &'static str {
        "HAVING is designed to filter grouped results. Using HAVING without GROUP BY \
         treats the entire result set as a single group, which is almost always a mistake. \
         Use WHERE for filtering ungrouped rows, or add the missing GROUP BY clause."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectStatement])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        let has_having = children
            .iter()
            .any(|c| c.segment_type() == SegmentType::HavingClause);
        let has_group_by = children
            .iter()
            .any(|c| c.segment_type() == SegmentType::GroupByClause);

        if has_having && !has_group_by {
            // Find the HavingClause span to report on
            let having_span = children
                .iter()
                .find(|c| c.segment_type() == SegmentType::HavingClause)
                .map(|c| c.span())
                .unwrap_or(ctx.segment.span());

            return vec![LintViolation::new(
                self.code(),
                "HAVING clause without GROUP BY. Use WHERE for ungrouped filtering.",
                having_span,
            )];
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_cv12_flags_having_without_group_by() {
        let violations = lint_sql("SELECT COUNT(*) FROM t HAVING COUNT(*) > 1", RuleCV12);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_cv12_accepts_having_with_group_by() {
        let violations = lint_sql(
            "SELECT a, COUNT(*) FROM t GROUP BY a HAVING COUNT(*) > 1",
            RuleCV12,
        );
        assert_eq!(violations.len(), 0);
    }
}
