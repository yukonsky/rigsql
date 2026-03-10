use rigsql_core::{Segment, SegmentType, TokenKind};
use rigsql_lexer::is_keyword;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// CP01: Keywords must be consistently capitalised.
///
/// By default, expects UPPER case keywords.
#[derive(Debug)]
pub struct RuleCP01 {
    pub policy: CapitalisationPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapitalisationPolicy {
    Upper,
    Lower,
    Capitalise,
}

impl Default for RuleCP01 {
    fn default() -> Self {
        Self {
            policy: CapitalisationPolicy::Upper,
        }
    }
}

impl Rule for RuleCP01 {
    fn code(&self) -> &'static str {
        "CP01"
    }
    fn name(&self) -> &'static str {
        "capitalisation.keywords"
    }
    fn description(&self) -> &'static str {
        "Keywords must be consistently capitalised."
    }
    fn explanation(&self) -> &'static str {
        "SQL keywords like SELECT, FROM, WHERE should use consistent capitalisation. \
         Mixed case reduces readability. Most style guides recommend UPPER case keywords \
         to distinguish them from identifiers."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Capitalisation]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::Keyword, SegmentType::Unparsable])
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(policy) = settings.get("capitalisation_policy") {
            self.policy = match policy.as_str() {
                "lower" => CapitalisationPolicy::Lower,
                "capitalise" | "capitalize" => CapitalisationPolicy::Capitalise,
                _ => CapitalisationPolicy::Upper,
            };
        }
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let Segment::Token(t) = ctx.segment else {
            return vec![];
        };
        if t.token.kind != TokenKind::Word {
            return vec![];
        }
        if !is_keyword(&t.token.text) {
            return vec![];
        }

        let text = t.token.text.as_str();
        let expected = match self.policy {
            CapitalisationPolicy::Upper => text.to_ascii_uppercase(),
            CapitalisationPolicy::Lower => text.to_ascii_lowercase(),
            CapitalisationPolicy::Capitalise => capitalise(text),
        };

        if text != expected {
            vec![LintViolation::with_fix(
                self.code(),
                format!(
                    "Keywords must be {} case. Found '{}' instead of '{}'.",
                    match self.policy {
                        CapitalisationPolicy::Upper => "upper",
                        CapitalisationPolicy::Lower => "lower",
                        CapitalisationPolicy::Capitalise => "capitalised",
                    },
                    text,
                    expected
                ),
                t.token.span,
                vec![SourceEdit::replace(t.token.span, expected.clone())],
            )]
        } else {
            vec![]
        }
    }
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}
