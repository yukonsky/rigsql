use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// LT09: Select targets should be on separate lines unless there is only one.
#[derive(Debug, Default)]
pub struct RuleLT09;

impl Rule for RuleLT09 {
    fn code(&self) -> &'static str {
        "LT09"
    }
    fn name(&self) -> &'static str {
        "layout.select_targets"
    }
    fn description(&self) -> &'static str {
        "Select targets should be on a new line unless there is only one."
    }
    fn explanation(&self) -> &'static str {
        "When a SELECT has multiple columns, each column should be on its own line. \
         This makes diffs cleaner and improves readability. A single column can stay \
         on the same line as SELECT."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Count non-trivia, non-keyword, non-comma items (the actual select targets)
        // A select target is an expression, column ref, alias expression, star, etc.
        let targets: Vec<_> = children
            .iter()
            .filter(|c| {
                let st = c.segment_type();
                !st.is_trivia() && st != SegmentType::Keyword && st != SegmentType::Comma
            })
            .collect();

        // If 0 or 1 target, no issue
        if targets.len() <= 1 {
            return vec![];
        }

        // Check if SELECT keyword and first target are on the same line
        // and there's no newline between targets
        let has_newline_between_targets = children
            .iter()
            .any(|c| c.segment_type() == SegmentType::Newline);

        if !has_newline_between_targets {
            return vec![LintViolation::with_msg_key(
                self.code(),
                "Select targets should be on separate lines.",
                ctx.segment.span(),
                "rules.LT09.msg",
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
    fn test_lt09_flags_multiple_targets_single_line() {
        let violations = lint_sql("SELECT a, b, c FROM t", RuleLT09);
        assert!(!violations.is_empty());
        assert!(violations.iter().all(|v| v.rule_code == "LT09"));
    }

    #[test]
    fn test_lt09_accepts_single_target() {
        let violations = lint_sql("SELECT a FROM t", RuleLT09);
        assert_eq!(violations.len(), 0);
    }
}
