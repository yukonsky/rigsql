use rigsql_core::{Segment, SegmentType, TokenKind};
use rigsql_lexer::is_keyword;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// CP02: Identifiers (non-keywords) must be consistently capitalised.
///
/// By default, expects lower case identifiers.
#[derive(Debug)]
pub struct RuleCP02 {
    pub policy: IdentifierPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierPolicy {
    Lower,
    Upper,
    Consistent,
}

impl Default for RuleCP02 {
    fn default() -> Self {
        Self {
            policy: IdentifierPolicy::Consistent,
        }
    }
}

impl Rule for RuleCP02 {
    fn code(&self) -> &'static str { "CP02" }
    fn name(&self) -> &'static str { "capitalisation.identifiers" }
    fn description(&self) -> &'static str { "Unquoted identifiers must be consistently capitalised." }
    fn explanation(&self) -> &'static str {
        "Unquoted identifiers (table names, column names) should use consistent capitalisation. \
         Most SQL style guides recommend lower_snake_case for identifiers."
    }
    fn groups(&self) -> &[RuleGroup] { &[RuleGroup::Capitalisation] }
    fn is_fixable(&self) -> bool { true }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::Identifier])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let Segment::Token(t) = ctx.segment else {
            return vec![];
        };
        if t.token.kind != TokenKind::Word {
            return vec![];
        }
        // Skip if it's actually a keyword
        if is_keyword(&t.token.text) {
            return vec![];
        }
        // Skip if parent is a FunctionCall (function names are handled by CP03)
        if let Some(parent) = ctx.parent {
            if parent.segment_type() == rigsql_core::SegmentType::FunctionCall {
                return vec![];
            }
        }

        let text = t.token.text.as_str();

        // Skip identifiers containing non-ASCII characters (e.g. Japanese column names)
        // — ascii case conversion would produce broken results
        if !text.is_ascii() {
            return vec![];
        }

        let expected = match self.policy {
            IdentifierPolicy::Lower => text.to_ascii_lowercase(),
            IdentifierPolicy::Upper => text.to_ascii_uppercase(),
            IdentifierPolicy::Consistent => return vec![], // TODO: track first-seen case
        };

        if text != expected {
            vec![LintViolation::with_fix(
                self.code(),
                format!(
                    "Unquoted identifiers must be {} case. Found '{}'.",
                    match self.policy {
                        IdentifierPolicy::Lower => "lower",
                        IdentifierPolicy::Upper => "upper",
                        IdentifierPolicy::Consistent => "consistent",
                    },
                    text
                ),
                t.token.span,
                vec![SourceEdit::replace(t.token.span, expected.clone())],
            )]
        } else {
            vec![]
        }
    }
}
