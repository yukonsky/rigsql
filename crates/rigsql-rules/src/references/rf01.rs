use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// RF01: Unresolved column reference.
///
/// Note: This rule requires schema information and is not yet fully implemented.
/// Currently a stub that returns no violations.
#[derive(Debug, Default)]
pub struct RuleRF01;

impl Rule for RuleRF01 {
    fn code(&self) -> &'static str {
        "RF01"
    }
    fn name(&self) -> &'static str {
        "references.from"
    }
    fn description(&self) -> &'static str {
        "References cannot be resolved without schema information."
    }
    fn explanation(&self) -> &'static str {
        "Column references in SELECT, WHERE, and other clauses should resolve to \
         a column in one of the referenced tables. This rule requires schema information \
         and is not yet fully implemented."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::References]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectStatement])
    }

    fn eval(&self, _ctx: &RuleContext) -> Vec<LintViolation> {
        // Stub: requires schema information to resolve column references.
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_rf01_stub_no_false_positives() {
        let violations = lint_sql("SELECT id, name FROM users WHERE active = 1", RuleRF01);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_rf01_stub_no_false_positives_join() {
        let violations = lint_sql(
            "SELECT u.id, o.total FROM users u JOIN orders o ON u.id = o.user_id",
            RuleRF01,
        );
        assert_eq!(violations.len(), 0);
    }
}
