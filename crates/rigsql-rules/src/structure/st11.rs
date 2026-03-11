use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// ST11: Unused JOIN (join without reference in SELECT or WHERE).
///
/// This is a stub rule. The full implementation would require complex
/// cross-reference analysis to determine if a joined table is actually
/// used in the query.
#[derive(Debug, Default)]
pub struct RuleST11;

impl Rule for RuleST11 {
    fn code(&self) -> &'static str {
        "ST11"
    }
    fn name(&self) -> &'static str {
        "structure.unused_join"
    }
    fn description(&self) -> &'static str {
        "Joined table is not referenced in the query."
    }
    fn explanation(&self) -> &'static str {
        "A table that is joined but never referenced in the SELECT, WHERE, or other \
         clauses may be unnecessary. Remove unused joins to simplify the query and \
         improve performance. This rule is currently a stub."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Structure]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectStatement])
    }

    fn eval(&self, _ctx: &RuleContext) -> Vec<LintViolation> {
        // Stub: not implemented yet
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_st11_no_false_positives() {
        let violations = lint_sql("SELECT a.id FROM a JOIN b ON a.id = b.id;", RuleST11);
        assert_eq!(violations.len(), 0);
    }
}
