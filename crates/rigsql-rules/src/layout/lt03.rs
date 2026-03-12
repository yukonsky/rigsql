use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// LT03: Operators should be followed by a single space.
///
/// Checks that comparison and arithmetic operators have spaces on both sides.
#[derive(Debug, Default)]
pub struct RuleLT03;

impl Rule for RuleLT03 {
    fn code(&self) -> &'static str {
        "LT03"
    }
    fn name(&self) -> &'static str {
        "layout.operators"
    }
    fn description(&self) -> &'static str {
        "Operators should be surrounded by single spaces."
    }
    fn explanation(&self) -> &'static str {
        "Binary operators (=, <, >, +, -, etc.) should have a single space on each side \
         for readability. 'a=b' and 'a  = b' are harder to read than 'a = b'."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![
            SegmentType::ComparisonOperator,
            SegmentType::ArithmeticOperator,
        ])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let span = ctx.segment.span();
        let mut violations = Vec::new();

        // Check space before operator
        // Newline (or whitespace followed by newline = indentation) counts as valid separation
        if ctx.index_in_parent > 0 {
            let prev = &ctx.siblings[ctx.index_in_parent - 1];
            let prev_type = prev.segment_type();
            if prev_type != SegmentType::Whitespace && prev_type != SegmentType::Newline {
                violations.push(LintViolation::with_fix_and_msg_key(
                    self.code(),
                    "Missing space before operator.",
                    span,
                    vec![SourceEdit::insert(span.start, " ")],
                    "rules.LT03.msg.before",
                    vec![],
                ));
            }
        }

        // Check space after operator
        // Newline after operator is acceptable (line break after operator style)
        if ctx.index_in_parent + 1 < ctx.siblings.len() {
            let next = &ctx.siblings[ctx.index_in_parent + 1];
            let next_type = next.segment_type();
            if next_type != SegmentType::Whitespace && next_type != SegmentType::Newline {
                violations.push(LintViolation::with_fix_and_msg_key(
                    self.code(),
                    "Missing space after operator.",
                    span,
                    vec![SourceEdit::insert(span.end, " ")],
                    "rules.LT03.msg.after",
                    vec![],
                ));
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_lt03_flags_missing_space() {
        let violations = lint_sql("SELECT * FROM t WHERE x=1", RuleLT03);
        assert!(!violations.is_empty());
        assert!(violations.iter().all(|v| v.rule_code == "LT03"));
    }

    #[test]
    fn test_lt03_accepts_proper_spacing() {
        let violations = lint_sql("SELECT * FROM t WHERE x = 1", RuleLT03);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_lt03_accepts_newline_after_operator() {
        // Operator at end of line — newline is valid separation (no conflict with LT01)
        let violations = lint_sql("SELECT 'Data' =\n    'n'", RuleLT03);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_lt03_accepts_newline_before_operator() {
        // Operator at start of line — newline is valid separation
        let violations = lint_sql("SELECT *\nFROM t WHERE x\n= 1", RuleLT03);
        let after_violations: Vec<_> = violations
            .iter()
            .filter(|v| v.message.contains("before"))
            .collect();
        assert_eq!(after_violations.len(), 0);
    }
}
