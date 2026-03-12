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

        // Position 0 means start of siblings — skip
        if ctx.index_in_parent == 0 {
            return vec![];
        }

        let prev = &ctx.siblings[ctx.index_in_parent - 1];
        // If previous is Newline, this is indentation — skip
        if prev.segment_type() == SegmentType::Newline {
            return vec![];
        }

        let text = t.token.text.as_str();

        // Check if this is trailing whitespace (next sibling is Newline or this is last sibling)
        let is_trailing = if ctx.index_in_parent + 1 < ctx.siblings.len() {
            ctx.siblings[ctx.index_in_parent + 1].segment_type() == SegmentType::Newline
        } else {
            true
        };

        if is_trailing {
            return vec![LintViolation::with_fix_and_msg_key(
                self.code(),
                "Trailing whitespace.",
                t.token.span,
                vec![SourceEdit::delete(t.token.span)],
                "rules.LT01.msg.trailing",
                vec![],
            )];
        }

        // Excessive inline spacing (multiple spaces)
        if text.len() > 1 {
            return vec![LintViolation::with_fix_and_msg_key(
                self.code(),
                format!("Expected single space, found {} spaces.", text.len()),
                t.token.span,
                vec![SourceEdit::replace(t.token.span, " ")],
                "rules.LT01.msg",
                vec![("count".to_string(), text.len().to_string())],
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
        assert!(!violations.is_empty());
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

    #[test]
    fn test_lt01_trailing_whitespace_removed() {
        let violations = lint_sql("SELECT *  \n FROM t", RuleLT01);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].message, "Trailing whitespace.");
        assert!(violations[0].fixes[0].new_text.is_empty());
    }

    #[test]
    fn test_lt01_single_trailing_space() {
        let violations = lint_sql("SELECT * \nFROM t", RuleLT01);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].message, "Trailing whitespace.");
        assert!(violations[0].fixes[0].new_text.is_empty());
    }
}
