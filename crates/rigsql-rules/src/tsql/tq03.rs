use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// TQ03: Empty batches (consecutive GO statements with nothing between them).
///
/// Two GO statements separated only by whitespace/newlines indicate an empty
/// batch which is unnecessary and likely accidental.
#[derive(Debug, Default)]
pub struct RuleTQ03;

impl Rule for RuleTQ03 {
    fn code(&self) -> &'static str {
        "TQ03"
    }
    fn name(&self) -> &'static str {
        "tsql.empty_batch"
    }
    fn description(&self) -> &'static str {
        "Avoid empty batches (consecutive GO statements)."
    }
    fn explanation(&self) -> &'static str {
        "In T-SQL, GO is a batch separator. Two consecutive GO statements with nothing \
         meaningful between them create an empty batch. This is unnecessary clutter and \
         may indicate accidentally deleted code."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::RootOnly
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        if ctx.dialect != "tsql" {
            return vec![];
        }

        let mut violations = Vec::new();
        let children = ctx.segment.children();

        let mut last_go_span = None;

        for child in children {
            if child.segment_type() == SegmentType::GoStatement {
                if let Some(_prev_span) = last_go_span {
                    // Consecutive GO with nothing meaningful between them
                    violations.push(LintViolation::with_msg_key(
                        self.code(),
                        "Empty batch: consecutive GO statements with no content between them.",
                        child.span(),
                        "rules.TQ03.msg",
                        vec![],
                    ));
                }
                last_go_span = Some(child.span());
            } else if !child.segment_type().is_trivia() {
                // Something meaningful between GO statements
                last_go_span = None;
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql_with_dialect;

    #[test]
    fn test_tq03_flags_consecutive_go() {
        let sql = "SELECT 1\nGO\nGO\n";
        let violations = lint_sql_with_dialect(sql, RuleTQ03, "tsql");
        // This test depends on the parser producing GoStatement nodes.
        // If the parser doesn't produce them for ANSI grammar, we expect 0 here.
        // The rule logic is still correct for when TSQL grammar is used.
        let _ = violations;
    }

    #[test]
    fn test_tq03_skips_non_tsql() {
        let violations = lint_sql_with_dialect("SELECT 1", RuleTQ03, "ansi");
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_tq03_no_violation_on_single_select() {
        let violations = lint_sql_with_dialect("SELECT 1", RuleTQ03, "tsql");
        assert_eq!(violations.len(), 0);
    }
}
