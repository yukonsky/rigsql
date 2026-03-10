use rigsql_core::{Segment, SegmentType, TokenKind};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// CP04: Boolean/Null literals must be consistently capitalised.
///
/// By default, expects UPPER case (TRUE, FALSE, NULL).
#[derive(Debug, Default)]
pub struct RuleCP04;

impl Rule for RuleCP04 {
    fn code(&self) -> &'static str {
        "CP04"
    }
    fn name(&self) -> &'static str {
        "capitalisation.literals"
    }
    fn description(&self) -> &'static str {
        "Boolean/Null literals must be consistently capitalised."
    }
    fn explanation(&self) -> &'static str {
        "Boolean literals (TRUE, FALSE) and NULL should be consistently capitalised. \
         Using UPPER case for these literals is the most common convention and improves readability."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Capitalisation]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::BooleanLiteral, SegmentType::NullLiteral])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let Segment::Token(t) = ctx.segment else {
            return vec![];
        };
        if t.token.kind != TokenKind::Word {
            return vec![];
        }

        let text = t.token.text.as_str();
        let expected = text.to_ascii_uppercase();

        if text != expected {
            vec![LintViolation::with_fix(
                self.code(),
                format!(
                    "Boolean/Null literals must be upper case. Found '{}' instead of '{}'.",
                    text, expected
                ),
                t.token.span,
                vec![SourceEdit::replace(t.token.span, expected.clone())],
            )]
        } else {
            vec![]
        }
    }
}
