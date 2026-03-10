use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// CV09: Use of blocked words.
///
/// Flag identifiers that match a configurable list of blocked words.
#[derive(Debug)]
pub struct RuleCV09 {
    pub blocked_words: Vec<String>,
}

impl Default for RuleCV09 {
    fn default() -> Self {
        Self {
            blocked_words: Vec::new(),
        }
    }
}

impl Rule for RuleCV09 {
    fn code(&self) -> &'static str { "CV09" }
    fn name(&self) -> &'static str { "convention.blocked_words" }
    fn description(&self) -> &'static str { "Use of blocked words." }
    fn explanation(&self) -> &'static str {
        "Certain words may be reserved, deprecated, or disallowed by team convention. \
         This rule flags identifiers that match a configurable list of blocked words."
    }
    fn groups(&self) -> &[RuleGroup] { &[RuleGroup::Convention] }
    fn is_fixable(&self) -> bool { false }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(val) = settings.get("blocked_words") {
            self.blocked_words = val
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::Identifier])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        if self.blocked_words.is_empty() {
            return vec![];
        }

        let Segment::Token(t) = ctx.segment else {
            return vec![];
        };

        let word = t.token.text.to_lowercase();
        if self.blocked_words.contains(&word) {
            return vec![LintViolation::new(
                self.code(),
                format!("Identifier '{}' is a blocked word.", t.token.text),
                t.token.span,
            )];
        }

        vec![]
    }
}
