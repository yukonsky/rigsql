use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// LT10: SELECT modifiers (DISTINCT, ALL) must be on same line as SELECT.
///
/// When using SELECT DISTINCT or SELECT ALL, the modifier should appear on
/// the same line as the SELECT keyword, with no intervening newline.
#[derive(Debug, Default)]
pub struct RuleLT10;

impl Rule for RuleLT10 {
    fn code(&self) -> &'static str {
        "LT10"
    }
    fn name(&self) -> &'static str {
        "layout.select_modifier"
    }
    fn description(&self) -> &'static str {
        "SELECT modifiers (DISTINCT, ALL) must be on same line as SELECT."
    }
    fn explanation(&self) -> &'static str {
        "SELECT modifiers such as DISTINCT or ALL should appear on the same line as \
         the SELECT keyword. Placing them on a separate line is confusing and reduces \
         readability."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Find the SELECT keyword
        let mut select_idx = None;
        for (i, child) in children.iter().enumerate() {
            if let Segment::Token(t) = child {
                if t.token.text.as_str().eq_ignore_ascii_case("SELECT") {
                    select_idx = Some(i);
                    break;
                }
            }
        }

        let Some(select_idx) = select_idx else {
            return vec![];
        };

        // Look at tokens after SELECT for DISTINCT or ALL
        let mut has_newline = false;
        for child in &children[select_idx + 1..] {
            let st = child.segment_type();
            if st == SegmentType::Newline {
                has_newline = true;
            } else if st.is_trivia() {
                continue;
            } else if let Segment::Token(t) = child {
                let text = t.token.text.as_str();
                if (text.eq_ignore_ascii_case("DISTINCT") || text.eq_ignore_ascii_case("ALL"))
                    && has_newline
                {
                    return vec![LintViolation::new(
                        self.code(),
                        format!(
                            "'{}' must be on the same line as SELECT.",
                            text.to_uppercase()
                        ),
                        t.token.span,
                    )];
                }
                // Whether it was a modifier or not, stop looking
                break;
            } else {
                break;
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
    fn test_lt10_accepts_same_line() {
        let violations = lint_sql("SELECT DISTINCT a FROM t", RuleLT10);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_lt10_flags_next_line() {
        let violations = lint_sql("SELECT\nDISTINCT a FROM t", RuleLT10);
        assert!(!violations.is_empty());
        assert!(violations.iter().all(|v| v.rule_code == "LT10"));
    }
}
