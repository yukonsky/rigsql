use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// RF01: References cannot reference objects not present in FROM clause.
///
/// Checks that table/alias qualifiers used in SELECT, WHERE, etc. actually
/// exist in the FROM or JOIN clauses of the query.
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
        "References cannot reference objects not present in FROM clause."
    }
    fn explanation(&self) -> &'static str {
        "Table or alias qualifiers used in SELECT, WHERE, GROUP BY, and other clauses \
         must correspond to a table or alias declared in the FROM or JOIN clauses. \
         Referencing an undeclared alias like 'vee.a' when only 'foo' is in FROM \
         is an error."
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
        // Stub: not yet implemented.
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
