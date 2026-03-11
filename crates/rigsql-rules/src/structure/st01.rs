use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// ST01: Do not specify redundant ELSE NULL in a CASE expression.
///
/// CASE expressions without an ELSE clause implicitly return NULL,
/// so `ELSE NULL` is redundant and should be removed.
#[derive(Debug, Default)]
pub struct RuleST01;

impl Rule for RuleST01 {
    fn code(&self) -> &'static str {
        "ST01"
    }
    fn name(&self) -> &'static str {
        "structure.else_null"
    }
    fn description(&self) -> &'static str {
        "Do not specify redundant ELSE NULL in a CASE expression."
    }
    fn explanation(&self) -> &'static str {
        "A CASE expression without an ELSE clause implicitly returns NULL. \
         Writing ELSE NULL is therefore redundant and adds unnecessary noise \
         to the query. Remove the ELSE NULL clause for clarity."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Structure]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::CaseExpression])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        for child in children {
            if child.segment_type() == SegmentType::ElseClause {
                // Check if the ElseClause's non-trivia content after ELSE keyword is just NullLiteral
                let else_children = child.children();
                let non_trivia: Vec<_> = else_children
                    .iter()
                    .filter(|s| !s.segment_type().is_trivia())
                    .collect();

                // Should be [Keyword("ELSE"), NullLiteral]
                if non_trivia.len() == 2
                    && non_trivia[0].segment_type() == SegmentType::Keyword
                    && non_trivia[1].segment_type() == SegmentType::NullLiteral
                {
                    return vec![LintViolation::with_fix(
                        self.code(),
                        "Redundant ELSE NULL in CASE expression.",
                        child.span(),
                        vec![SourceEdit::delete(child.span())],
                    )];
                }
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
    fn test_st01_flags_else_null() {
        let violations = lint_sql("SELECT CASE WHEN x = 1 THEN 'a' ELSE NULL END;", RuleST01);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("Redundant ELSE NULL"));
    }

    #[test]
    fn test_st01_accepts_else_value() {
        let violations = lint_sql("SELECT CASE WHEN x = 1 THEN 'a' ELSE 'b' END;", RuleST01);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_st01_accepts_no_else() {
        let violations = lint_sql("SELECT CASE WHEN x = 1 THEN 'a' END;", RuleST01);
        assert_eq!(violations.len(), 0);
    }
}
