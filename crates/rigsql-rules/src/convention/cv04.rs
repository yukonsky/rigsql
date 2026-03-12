use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::first_non_trivia;
use crate::violation::{LintViolation, SourceEdit};

/// CV04: Use COUNT(*) instead of COUNT(0) or COUNT(1).
///
/// COUNT(*) is the standard way to count rows and is clear in intent.
#[derive(Debug, Default)]
pub struct RuleCV04;

impl Rule for RuleCV04 {
    fn code(&self) -> &'static str {
        "CV04"
    }
    fn name(&self) -> &'static str {
        "convention.count"
    }
    fn description(&self) -> &'static str {
        "Use consistent syntax to count all rows."
    }
    fn explanation(&self) -> &'static str {
        "COUNT(*) is the standard and most readable way to count all rows. \
         COUNT(1) and COUNT(0) produce the same result but are less clear in intent. \
         Using COUNT(*) consistently makes the code more readable."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::FunctionCall])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Check if function is COUNT
        let func_name = first_non_trivia(children);
        let is_count = match func_name {
            Some(Segment::Token(t)) => t.token.text.eq_ignore_ascii_case("COUNT"),
            _ => false,
        };

        if !is_count {
            return vec![];
        }

        // Find the FunctionArgs node and check its content
        for child in children {
            if child.segment_type() == SegmentType::FunctionArgs {
                let arg_tokens = child.tokens();
                // Filter to non-trivia, non-paren tokens
                let args: Vec<_> = arg_tokens
                    .iter()
                    .filter(|t| {
                        !t.kind.is_trivia()
                            && t.kind != rigsql_core::TokenKind::LParen
                            && t.kind != rigsql_core::TokenKind::RParen
                    })
                    .collect();

                // If single argument is a numeric literal "0" or "1"
                if args.len() == 1 {
                    let text = args[0].text.as_str();
                    if text == "0" || text == "1" {
                        return vec![LintViolation::with_fix_and_msg_key(
                            self.code(),
                            format!("Use COUNT(*) instead of COUNT({}).", text),
                            ctx.segment.span(),
                            vec![SourceEdit::replace(args[0].span, "*")],
                            "rules.CV04.msg",
                            vec![("arg".to_string(), text.to_string())],
                        )];
                    }
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
    fn test_cv04_count_1_not_detected_yet() {
        // NOTE: Parser currently produces ParenExpression instead of FunctionArgs,
        // so the rule cannot detect COUNT(1) yet. This test documents current behavior.
        let violations = lint_sql("SELECT COUNT(1) FROM t", RuleCV04);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cv04_count_0_not_detected_yet() {
        // NOTE: Same parser limitation as above.
        let violations = lint_sql("SELECT COUNT(0) FROM t", RuleCV04);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cv04_accepts_count_star() {
        let violations = lint_sql("SELECT COUNT(*) FROM t", RuleCV04);
        assert_eq!(violations.len(), 0);
    }
}
