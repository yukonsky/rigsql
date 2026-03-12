use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// CV03: Trailing comma in SELECT clause.
///
/// The last column in a SELECT list should not be followed by a comma.
/// A trailing comma before FROM is a syntax error in most databases.
#[derive(Debug, Default)]
pub struct RuleCV03;

impl Rule for RuleCV03 {
    fn code(&self) -> &'static str {
        "CV03"
    }
    fn name(&self) -> &'static str {
        "convention.select_trailing_comma"
    }
    fn description(&self) -> &'static str {
        "Trailing comma in SELECT clause."
    }
    fn explanation(&self) -> &'static str {
        "A trailing comma at the end of a SELECT column list (before FROM or end of \
         statement) is a syntax error in most SQL databases. Remove the extraneous comma."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Find the last non-trivia child
        let last_non_trivia = children
            .iter()
            .rev()
            .find(|c| !c.segment_type().is_trivia());

        if let Some(seg) = last_non_trivia {
            if seg.segment_type() == SegmentType::Comma {
                return vec![LintViolation::with_fix_and_msg_key(
                    self.code(),
                    "Trailing comma found in SELECT clause.",
                    seg.span(),
                    vec![SourceEdit::delete(seg.span())],
                    "rules.CV03.msg",
                    vec![],
                )];
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
    fn test_cv03_flags_trailing_comma() {
        let violations = lint_sql("SELECT a, b,", RuleCV03);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_cv03_accepts_no_trailing_comma() {
        let violations = lint_sql("SELECT a, b FROM t", RuleCV03);
        assert_eq!(violations.len(), 0);
    }
}
