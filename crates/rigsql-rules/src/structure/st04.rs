use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// ST04: Nested CASE expression in an ELSE clause.
///
/// The inner WHEN branches can be moved to the end of the outer CASE.
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
        "Nested CASE expression in an ELSE clause should be flattened."
    }
    fn explanation(&self) -> &'static str {
        "A CASE expression in the ELSE clause of another CASE is harder to read than \
         the equivalent flat form. Its WHEN branches can be moved to the end of the \
         outer CASE. A CASE inside a THEN branch is not reported, because it has no \
         flat equivalent."
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
        // Only a CASE inside the ELSE clause is reported: its WHEN branches can
        // be lifted into the outer CASE. A CASE inside a THEN branch produces a
        // value for one branch and has no flat equivalent, so it is left alone.
        let found_nested = ctx
            .segment
            .children()
            .iter()
            .filter(|c| c.segment_type() == SegmentType::ElseClause)
            .any(contains_case_expression);

        if found_nested {
            return vec![LintViolation::with_msg_key(
                self.code(),
                "Nested CASE expression in ELSE clause could be flattened.",
                ctx.segment.span(),
                "rules.ST04.msg",
                vec![],
            )];
        }

        vec![]
    }
}

/// Search for a CASE expression below `segment`, without descending into a
/// subquery — a CASE inside `ELSE (SELECT ...)` is not a nested CASE.
fn contains_case_expression(segment: &Segment) -> bool {
    segment
        .children()
        .iter()
        .any(|child| match child.segment_type() {
            SegmentType::CaseExpression => true,
            SegmentType::Subquery | SegmentType::SelectStatement => false,
            _ => contains_case_expression(child),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_st04_flags_nested_case() {
        let violations = lint_sql(
            "SELECT CASE WHEN x = 1 THEN 'a' ELSE CASE WHEN y = 2 THEN 'b' ELSE 'c' END END;",
            RuleST04,
        );
        // The outer CASE is flagged (its ELSE clause holds a nested CASE)
        assert!(!violations.is_empty());
        assert!(violations[0].message.contains("Nested CASE"));
    }

    #[test]
    fn test_st04_accepts_case_nested_in_then() {
        // A CASE in a THEN branch cannot be flattened into the outer CASE.
        let violations = lint_sql(
            "SELECT CASE WHEN x = 1 THEN CASE WHEN y = 2 THEN 'a' ELSE 'b' END ELSE 'c' END;",
            RuleST04,
        );
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_st04_accepts_case_in_else_subquery() {
        let sql = "SELECT CASE WHEN x = 1 THEN 'a'                    ELSE (SELECT CASE WHEN y = 2 THEN 'b' ELSE 'c' END FROM t) END;";
        let violations = lint_sql(sql, RuleST04);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_st04_accepts_simple_case() {
        let violations = lint_sql("SELECT CASE WHEN x = 1 THEN 'a' ELSE 'b' END;", RuleST04);
        assert_eq!(violations.len(), 0);
    }
}
