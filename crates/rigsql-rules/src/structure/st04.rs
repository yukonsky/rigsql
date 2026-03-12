use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// ST04: Nested CASE expressions.
///
/// Nested CASE statements are hard to read and should be refactored.
#[derive(Debug, Default)]
pub struct RuleST04;

impl Rule for RuleST04 {
    fn code(&self) -> &'static str {
        "ST04"
    }
    fn name(&self) -> &'static str {
        "structure.nested_case"
    }
    fn description(&self) -> &'static str {
        "Nested CASE expressions should be avoided."
    }
    fn explanation(&self) -> &'static str {
        "Nested CASE expressions make SQL queries difficult to read and maintain. \
         Consider refactoring the logic using CTEs, subqueries, or separate columns \
         to improve readability."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Structure]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::CaseExpression])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        // Check if any descendant (not self) is also a CaseExpression
        let mut found_nested = false;
        let mut is_first = true;

        ctx.segment.walk(&mut |seg| {
            if is_first {
                is_first = false;
                return;
            }
            if seg.segment_type() == SegmentType::CaseExpression {
                found_nested = true;
            }
        });

        if found_nested {
            return vec![LintViolation::with_msg_key(
                self.code(),
                "Nested CASE expression found.",
                ctx.segment.span(),
                "rules.ST04.msg",
                vec![],
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
    fn test_st04_flags_nested_case() {
        let violations = lint_sql(
            "SELECT CASE WHEN x = 1 THEN CASE WHEN y = 2 THEN 'a' ELSE 'b' END ELSE 'c' END;",
            RuleST04,
        );
        // The outer CASE is flagged (it contains a nested CASE)
        assert!(!violations.is_empty());
        assert!(violations[0].message.contains("Nested CASE"));
    }

    #[test]
    fn test_st04_accepts_simple_case() {
        let violations = lint_sql("SELECT CASE WHEN x = 1 THEN 'a' ELSE 'b' END;", RuleST04);
        assert_eq!(violations.len(), 0);
    }
}
