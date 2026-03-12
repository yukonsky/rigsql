use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// ST08: DISTINCT used with parentheses.
///
/// DISTINCT applies to the entire row, not a single column. Using
/// parentheses after DISTINCT makes it look like a function call.
#[derive(Debug, Default)]
pub struct RuleST08;

impl Rule for RuleST08 {
    fn code(&self) -> &'static str {
        "ST08"
    }
    fn name(&self) -> &'static str {
        "structure.distinct"
    }
    fn description(&self) -> &'static str {
        "DISTINCT used with parentheses is misleading."
    }
    fn explanation(&self) -> &'static str {
        "DISTINCT is a keyword that applies to the entire SELECT result set, not a function. \
         Writing SELECT DISTINCT(col) suggests DISTINCT operates on a single column like a \
         function, which is misleading. Use SELECT DISTINCT col instead."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Structure]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Look for DISTINCT keyword followed by a ParenExpression
        let mut found_distinct = false;
        for child in children {
            if child.segment_type().is_trivia() {
                continue;
            }

            if found_distinct {
                // The next non-trivia after DISTINCT
                if child.segment_type() == SegmentType::ParenExpression {
                    return vec![LintViolation::with_msg_key(
                        self.code(),
                        "DISTINCT used with parentheses is misleading. DISTINCT is not a function.",
                        child.span(),
                        "rules.ST08.msg",
                        vec![],
                    )];
                }
                // If it's not a paren expression, stop looking
                found_distinct = false;
            }

            if let Segment::Token(t) = child {
                if t.segment_type == SegmentType::Keyword
                    && t.token.text.eq_ignore_ascii_case("DISTINCT")
                {
                    found_distinct = true;
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
    fn test_st08_accepts_distinct_without_parens() {
        let violations = lint_sql("SELECT DISTINCT a, b FROM t;", RuleST08);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_st08_accepts_no_distinct() {
        let violations = lint_sql("SELECT a FROM t;", RuleST08);
        assert_eq!(violations.len(), 0);
    }
}
