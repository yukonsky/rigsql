use rigsql_core::{Segment, SegmentType, TokenKind};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::check_capitalisation;
use crate::violation::LintViolation;

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

        check_capitalisation(
            self.code(),
            "Boolean/Null literals",
            text,
            &expected,
            "upper",
            t.token.span,
        )
        .into_iter()
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_cp04_flags_lowercase_null() {
        let violations = lint_sql("SELECT null", RuleCP04);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_cp04_accepts_uppercase_null() {
        let violations = lint_sql("SELECT NULL", RuleCP04);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cp04_flags_lowercase_true() {
        let violations = lint_sql("SELECT true", RuleCP04);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_cp04_fix_uppercases() {
        let violations = lint_sql("SELECT null", RuleCP04);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "NULL");
    }
}
