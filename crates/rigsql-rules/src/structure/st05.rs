use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// ST05: Derived tables (subqueries in FROM) should be CTEs.
///
/// Subqueries in the FROM clause are harder to read than CTEs.
#[derive(Debug, Default)]
pub struct RuleST05;

impl Rule for RuleST05 {
    fn code(&self) -> &'static str {
        "ST05"
    }
    fn name(&self) -> &'static str {
        "structure.subquery"
    }
    fn description(&self) -> &'static str {
        "Derived tables should use CTEs instead."
    }
    fn explanation(&self) -> &'static str {
        "Subqueries in the FROM clause (derived tables) reduce readability compared \
         to Common Table Expressions (CTEs). Consider refactoring derived tables \
         into CTEs defined in a WITH clause."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Structure]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::FromClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        ctx.segment.walk(&mut |seg| {
            if seg.segment_type() == SegmentType::Subquery {
                violations.push(LintViolation::new(
                    self.code(),
                    "Use a CTE instead of a derived table (subquery in FROM).",
                    seg.span(),
                ));
            }
        });

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_st05_flags_subquery_in_from() {
        let violations = lint_sql("SELECT * FROM (SELECT id FROM t) AS sub;", RuleST05);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("CTE"));
    }

    #[test]
    fn test_st05_accepts_simple_from() {
        let violations = lint_sql("SELECT * FROM t;", RuleST05);
        assert_eq!(violations.len(), 0);
    }
}
