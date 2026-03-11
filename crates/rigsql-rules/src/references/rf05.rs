use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// RF05: Identifiers should not contain special characters.
///
/// Bare identifiers should only contain alphanumeric characters and underscores.
/// QuotedIdentifiers are excluded from this check as they may legitimately need
/// special characters.
#[derive(Debug, Default)]
pub struct RuleRF05;

impl Rule for RuleRF05 {
    fn code(&self) -> &'static str {
        "RF05"
    }
    fn name(&self) -> &'static str {
        "references.special_chars"
    }
    fn description(&self) -> &'static str {
        "Identifiers should not contain special characters."
    }
    fn explanation(&self) -> &'static str {
        "Bare identifiers should only contain alphanumeric characters and underscores. \
         Special characters (spaces, hyphens, etc.) in identifiers require quoting and \
         can cause portability issues. If special characters are needed, use quoted \
         identifiers explicitly."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::References]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::Identifier])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let Segment::Token(t) = ctx.segment else {
            return vec![];
        };

        let text = &t.token.text;
        let has_special = text.chars().any(|c| !c.is_alphanumeric() && c != '_');

        if has_special {
            vec![LintViolation::new(
                self.code(),
                format!("Identifier '{}' contains special characters.", text),
                t.token.span,
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
    fn test_rf05_accepts_normal_identifiers() {
        let violations = lint_sql("SELECT user_id, email FROM users_2024", RuleRF05);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_rf05_accepts_quoted_identifiers() {
        // QuotedIdentifier is not crawled by RF05
        let violations = lint_sql("SELECT \"my-col\" FROM t", RuleRF05);
        assert_eq!(violations.len(), 0);
    }
}
