use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// CV09: Use of blocked words.
///
/// Flag identifiers that match a configurable list of blocked words.
#[derive(Debug, Default)]
pub struct RuleCV09 {
    pub blocked_words: Vec<String>,
}

impl Rule for RuleCV09 {
    fn code(&self) -> &'static str {
        "CV09"
    }
    fn name(&self) -> &'static str {
        "convention.blocked_words"
    }
    fn description(&self) -> &'static str {
        "Use of blocked words."
    }
    fn explanation(&self) -> &'static str {
        "Certain words may be reserved, deprecated, or disallowed by team convention. \
         This rule flags identifiers that match a configurable list of blocked words."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        false
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_cv09_no_blocked_words_no_violation() {
        let violations = lint_sql("SELECT temp FROM t", RuleCV09::default());
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cv09_flags_blocked_word() {
        let rule = RuleCV09 {
            blocked_words: vec!["temp".to_string(), "foo".to_string()],
        };
        let violations = lint_sql("SELECT temp FROM t", rule);
        assert_eq!(violations.len(), 1);
    }
}
