use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::{extract_alias_name, is_in_table_context};
use crate::violation::LintViolation;

/// AL06: Enforce table alias lengths in FROM clauses and JOIN conditions.
///
/// Configurable minimum and maximum alias length. Disabled by default
/// (both min and max are 0).
#[derive(Debug, Default)]
pub struct RuleAL06 {
    pub min_alias_length: usize,
    pub max_alias_length: usize,
}

impl Rule for RuleAL06 {
    fn code(&self) -> &'static str {
        "AL06"
    }
    fn name(&self) -> &'static str {
        "aliasing.length"
    }
    fn description(&self) -> &'static str {
        "Enforce table alias lengths in FROM clauses and JOIN conditions."
    }
    fn explanation(&self) -> &'static str {
        "Table aliases that are too short (like single letters) can be cryptic. \
         Aliases that are too long defeat the purpose of aliasing. Configure \
         min_alias_length and max_alias_length to enforce your team's standards."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Aliasing]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(val) = settings.get("min_alias_length") {
            if let Ok(n) = val.parse() {
                self.min_alias_length = n;
            }
        }
        if let Some(val) = settings.get("max_alias_length") {
            if let Ok(n) = val.parse() {
                self.max_alias_length = n;
            }
        }
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::AliasExpression])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        if !is_in_table_context(ctx) {
            return vec![];
        }

        // Both 0 means disabled
        if self.min_alias_length == 0 && self.max_alias_length == 0 {
            return vec![];
        }

        let Some(alias_name) = extract_alias_name(ctx.segment.children()) else {
            return vec![];
        };

        let len = alias_name.len();

        if self.min_alias_length > 0 && len < self.min_alias_length {
            return vec![LintViolation::new(
                self.code(),
                format!(
                    "Alias '{}' is too short ({} chars, minimum {}).",
                    alias_name, len, self.min_alias_length
                ),
                ctx.segment.span(),
            )];
        }

        if self.max_alias_length > 0 && len > self.max_alias_length {
            return vec![LintViolation::new(
                self.code(),
                format!(
                    "Alias '{}' is too long ({} chars, maximum {}).",
                    alias_name, len, self.max_alias_length
                ),
                ctx.segment.span(),
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
    fn test_al06_default_no_violation() {
        let violations = lint_sql("SELECT * FROM users AS u", RuleAL06::default());
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_al06_min_length_flags_short() {
        let rule = RuleAL06 {
            min_alias_length: 2,
            max_alias_length: 0,
        };
        let violations = lint_sql("SELECT * FROM users AS u", rule);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_al06_min_length_accepts_long() {
        let rule = RuleAL06 {
            min_alias_length: 2,
            max_alias_length: 0,
        };
        let violations = lint_sql("SELECT * FROM users AS usr", rule);
        assert_eq!(violations.len(), 0);
    }
}
