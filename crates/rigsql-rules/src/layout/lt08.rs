use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// LT08: Blank line expected but not found before CTE definition.
///
/// Within a WITH clause, each CTE definition (after the first) should be
/// preceded by a blank line for readability.
#[derive(Debug, Default)]
pub struct RuleLT08;

impl Rule for RuleLT08 {
    fn code(&self) -> &'static str {
        "LT08"
    }
    fn name(&self) -> &'static str {
        "layout.cte_newline"
    }
    fn description(&self) -> &'static str {
        "Blank line expected but not found before CTE definition."
    }
    fn explanation(&self) -> &'static str {
        "When a WITH clause contains multiple CTEs, each CTE after the first should \
         be separated by a blank line to improve readability. A single newline between \
         CTEs makes it harder to distinguish where one ends and the next begins."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::WithClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();
        let mut violations = Vec::new();
        let mut cte_count = 0;

        for (i, child) in children.iter().enumerate() {
            if child.segment_type() != SegmentType::CteDefinition {
                continue;
            }
            cte_count += 1;
            if cte_count <= 1 {
                continue;
            }

            // Single backward scan: count newlines and find insertion point
            let (newline_count, insert_offset) = scan_trivia_before(children, i);

            if newline_count < 2 {
                violations.push(LintViolation::with_fix(
                    self.code(),
                    "Expected blank line before CTE definition.",
                    child.span(),
                    vec![SourceEdit::insert(insert_offset, "\n")],
                ));
            }
        }

        violations
    }
}

/// Single backward scan: count consecutive newlines before this CTE
/// and find the insertion point (end of the nearest newline).
fn scan_trivia_before(children: &[rigsql_core::Segment], cte_idx: usize) -> (usize, u32) {
    let mut newline_count = 0;
    let mut last_newline_end = children[cte_idx].span().start;

    for child in children[..cte_idx].iter().rev() {
        let st = child.segment_type();
        if st == SegmentType::Newline {
            newline_count += 1;
            if newline_count == 1 {
                last_newline_end = child.span().end;
            }
        } else if st.is_trivia() {
            continue;
        } else {
            break;
        }
    }

    (newline_count, last_newline_end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_lt08_accepts_single_cte() {
        let violations = lint_sql("WITH cte AS (SELECT 1) SELECT * FROM cte", RuleLT08);
        assert_eq!(violations.len(), 0);
    }
}
