use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// ST09: Join condition order convention.
///
/// This is a stub rule. The full implementation would check that in an ON
/// clause, the column from the most recently JOINed table appears on the
/// left side of the comparison. This is complex to detect reliably.
#[derive(Debug, Default)]
pub struct RuleST09;

impl Rule for RuleST09 {
    fn code(&self) -> &'static str {
        "ST09"
    }
    fn name(&self) -> &'static str {
        "structure.join_condition_order"
    }
    fn description(&self) -> &'static str {
        "Join condition column order convention."
    }
    fn explanation(&self) -> &'static str {
        "In a JOIN ... ON clause, the column from the table being joined should appear \
         on the left side of the comparison for consistency and readability. \
         This rule is currently a stub and does not produce violations."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Structure]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::OnClause])
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
    fn test_st09_no_false_positives() {
        let violations = lint_sql("SELECT * FROM a JOIN b ON a.id = b.id;", RuleST09);
        assert_eq!(violations.len(), 0);
    }
}
