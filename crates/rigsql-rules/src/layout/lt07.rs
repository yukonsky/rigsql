use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::has_trailing_newline;
use crate::violation::LintViolation;

/// LT07: WITH clause closing bracket should be on a new line.
///
/// The closing parenthesis of a CTE definition should be on its own line.
#[derive(Debug, Default)]
pub struct RuleLT07;

impl Rule for RuleLT07 {
    fn code(&self) -> &'static str {
        "LT07"
    }
    fn name(&self) -> &'static str {
        "layout.cte_bracket"
    }
    fn description(&self) -> &'static str {
        "WITH clause closing bracket should be on a new line."
    }
    fn explanation(&self) -> &'static str {
        "The closing parenthesis of a CTE definition should be placed on its \
         own line, not on the same line as the last expression in the CTE body."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::CteDefinition])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // The RParen may be a direct child or nested inside a Subquery node.
        let (search_children, rparen_idx) = if let Some(idx) = children
            .iter()
            .rposition(|c| c.segment_type() == SegmentType::RParen)
        {
            (children, idx)
        } else {
            // Look inside a Subquery child
            let subquery = children
                .iter()
                .find(|c| c.segment_type() == SegmentType::Subquery);
            let Some(sq) = subquery else {
                return vec![];
            };
            let sq_children = sq.children();
            let Some(idx) = sq_children
                .iter()
                .rposition(|c| c.segment_type() == SegmentType::RParen)
            else {
                return vec![];
            };
            (sq_children, idx)
        };

        // Look backwards from RParen for a Newline (only whitespace allowed between).
        // Newlines may be absorbed as trailing trivia of the previous segment.
        let mut found_newline = false;
        for child in search_children[..rparen_idx].iter().rev() {
            let st = child.segment_type();
            if st == SegmentType::Newline {
                found_newline = true;
                break;
            }
            if st == SegmentType::Whitespace {
                continue;
            }
            // Check if this segment ends with a trailing Newline
            found_newline = has_trailing_newline(child);
            break;
        }

        if !found_newline {
            return vec![LintViolation::new(
                self.code(),
                "Closing bracket of CTE should be on a new line.",
                search_children[rparen_idx].span(),
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
    fn test_lt07_accepts_newline_before_bracket() {
        let violations = lint_sql("WITH cte AS (\n  SELECT 1\n) SELECT * FROM cte", RuleLT07);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_lt07_flags_inline_bracket() {
        let violations = lint_sql("WITH cte AS (SELECT 1) SELECT * FROM cte", RuleLT07);
        assert_eq!(violations.len(), 1);
    }
}
