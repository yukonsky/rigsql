use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// ST06: Select column order convention.
///
/// This is a stub rule. The full sqlfluff ST06 checks for specific column
/// ordering conventions (e.g., aggregates after non-aggregates) which is
/// complex to detect reliably.
#[derive(Debug, Default)]
pub struct RuleST06;

impl Rule for RuleST06 {
    fn code(&self) -> &'static str {
        "ST06"
    }
    fn name(&self) -> &'static str {
        "structure.column_order"
    }
    fn description(&self) -> &'static str {
        "Select column order convention."
    }
    fn explanation(&self) -> &'static str {
        "Columns in a SELECT clause should follow a consistent ordering convention. \
         For example, wildcards and simple columns before aggregate or window functions. \
         This rule is currently a stub and does not produce violations."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Structure]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectClause])
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
    fn test_st06_no_false_positives() {
        let violations = lint_sql("SELECT a, b, COUNT(*) FROM t GROUP BY a, b;", RuleST06);
        assert_eq!(violations.len(), 0);
    }
}
