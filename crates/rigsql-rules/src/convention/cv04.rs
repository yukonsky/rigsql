use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

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
        let func_name = children.iter().find(|c| !c.segment_type().is_trivia());
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
                        return vec![LintViolation::new(
                            self.code(),
                            format!("Use COUNT(*) instead of COUNT({}).", text),
                            ctx.segment.span(),
                        )];
                    }
                }
            }
        }

        vec![]
    }
}
