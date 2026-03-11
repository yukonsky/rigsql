use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// CV07: Top-level statements should not be wrapped in brackets.
///
/// Parentheses around a top-level statement are unnecessary and reduce
/// readability.
#[derive(Debug, Default)]
pub struct RuleCV07;

impl Rule for RuleCV07 {
    fn code(&self) -> &'static str {
        "CV07"
    }
    fn name(&self) -> &'static str {
        "convention.statement_brackets"
    }
    fn description(&self) -> &'static str {
        "Top-level statements should not be wrapped in brackets."
    }
    fn explanation(&self) -> &'static str {
        "Wrapping an entire statement in parentheses is unnecessary and can be \
         confusing. Remove the outer brackets to improve readability."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::Statement])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();
        let non_trivia: Vec<_> = children
            .iter()
            .filter(|c| !c.segment_type().is_trivia())
            .collect();

        if non_trivia.len() < 2 {
            return vec![];
        }

        let first = non_trivia.first().unwrap();
        let last_idx = non_trivia.len() - 1;
        // Last non-trivia might be Semicolon, check the one before it
        let (check_last, _has_semi) =
            if non_trivia[last_idx].segment_type() == SegmentType::Semicolon && last_idx >= 2 {
                (non_trivia[last_idx - 1], true)
            } else {
                (non_trivia[last_idx], false)
            };

        let is_lparen = first.segment_type() == SegmentType::LParen
            || matches!(first, Segment::Token(t) if t.token.text.as_str() == "(");
        let is_rparen = check_last.segment_type() == SegmentType::RParen
            || matches!(check_last, Segment::Token(t) if t.token.text.as_str() == ")");

        if is_lparen && is_rparen {
            vec![LintViolation::with_fix(
                self.code(),
                "Unnecessary brackets around statement.",
                ctx.segment.span(),
                vec![
                    SourceEdit::delete(first.span()),
                    SourceEdit::delete(check_last.span()),
                ],
            )]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_cv07_accepts_normal_statement() {
        let violations = lint_sql("SELECT 1", RuleCV07);
        assert_eq!(violations.len(), 0);
    }
}
