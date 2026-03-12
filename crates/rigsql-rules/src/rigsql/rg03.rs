use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// RG03: Use of BETWEEN operator.
///
/// The BETWEEN operator can be confusing, especially with date ranges,
/// because it is inclusive on both ends. Prefer explicit >= AND <= comparisons.
#[derive(Debug, Default)]
pub struct RuleRG03;

impl Rule for RuleRG03 {
    fn code(&self) -> &'static str {
        "RG03"
    }
    fn name(&self) -> &'static str {
        "rigsql.no_between"
    }
    fn description(&self) -> &'static str {
        "Use of BETWEEN operator."
    }
    fn explanation(&self) -> &'static str {
        "The BETWEEN operator is inclusive on both ends and can lead to subtle bugs, \
         especially with date/time ranges. For example, 'BETWEEN '2024-01-01' AND '2024-01-31'' \
         may miss times on the 31st after midnight. Prefer explicit '>= ... AND <= ...' for clarity."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::BetweenExpression])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        vec![LintViolation::with_msg_key(
            self.code(),
            "Use of BETWEEN. Consider using >= and <= for explicit range boundaries.",
            ctx.segment.span(),
            "rules.RG03.msg",
            vec![],
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_rg03_flags_between() {
        let violations = lint_sql("SELECT * FROM t WHERE x BETWEEN 1 AND 10", RuleRG03);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_rg03_accepts_comparison() {
        let violations = lint_sql("SELECT * FROM t WHERE x >= 1", RuleRG03);
        assert_eq!(violations.len(), 0);
    }
}
