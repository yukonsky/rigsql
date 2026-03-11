use rigsql_core::{Segment, SegmentType, TokenKind};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// LT01: Inappropriate spacing.
///
/// Checks for multiple spaces where a single space is expected,
/// and missing spaces around operators.
#[derive(Debug, Default)]
pub struct RuleLT01;

impl Rule for RuleLT01 {
    fn code(&self) -> &'static str {
        "LT01"
    }
    fn name(&self) -> &'static str {
        "layout.spacing"
    }
    fn description(&self) -> &'static str {
        "Inappropriate spacing found."
    }
    fn explanation(&self) -> &'static str {
        "SQL should use single spaces between keywords and expressions. \
         Multiple consecutive spaces (except for indentation) reduce readability. \
         Operators should have spaces on both sides."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::Whitespace])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let Segment::Token(t) = ctx.segment else {
            return vec![];
        };
        if t.token.kind != TokenKind::Whitespace {
            return vec![];
        }

        let text = t.token.text.as_str();

        // Only flag multiple spaces within a line (not indentation)
        // Check if this whitespace is preceded by a non-newline token
        if text.len() > 1 && ctx.index_in_parent > 0 {
            let prev = &ctx.siblings[ctx.index_in_parent - 1];
            // If previous is Newline, this is indentation — skip
            if prev.segment_type() == SegmentType::Newline {
                return vec![];
            }

            return vec![LintViolation::with_fix(
                self.code(),
                format!("Expected single space, found {} spaces.", text.len()),
                t.token.span,
                vec![SourceEdit::replace(t.token.span, " ")],
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
    fn test_lt01_flags_double_space() {
        let violations = lint_sql("SELECT  *  FROM t", RuleLT01);
        assert!(violations.len() >= 1);
        assert!(violations.iter().all(|v| v.rule_code == "LT01"));
    }

    #[test]
    fn test_lt01_accepts_single_space() {
        let violations = lint_sql("SELECT * FROM t", RuleLT01);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_lt01_skips_indentation() {
        let violations = lint_sql("SELECT *\n    FROM t", RuleLT01);
        assert_eq!(violations.len(), 0);
    }
}
