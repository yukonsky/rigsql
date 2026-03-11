use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AM08: Implicit cross join (comma in FROM clause).
///
/// Using commas to join tables in the FROM clause is an implicit cross join,
/// which should be replaced with explicit CROSS JOIN syntax.
#[derive(Debug, Default)]
pub struct RuleAM08;

impl Rule for RuleAM08 {
    fn code(&self) -> &'static str {
        "AM08"
    }
    fn name(&self) -> &'static str {
        "ambiguous.join_condition"
    }
    fn description(&self) -> &'static str {
        "Implicit cross join in FROM clause."
    }
    fn explanation(&self) -> &'static str {
        "Using commas to separate tables in a FROM clause creates an implicit cross join \
         (Cartesian product). Use explicit CROSS JOIN syntax instead, or use appropriate \
         JOIN ... ON clauses if you intend a different join type."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Ambiguous]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::FromClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        children
            .iter()
            .filter(|c| c.segment_type() == SegmentType::Comma)
            .map(|comma| {
                LintViolation::new(
                    self.code(),
                    "Implicit cross join via comma in FROM clause. Use explicit JOIN.",
                    comma.span(),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_am08_flags_comma_join() {
        let violations = lint_sql("SELECT a FROM t, u", RuleAM08);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_am08_accepts_explicit_join() {
        let violations = lint_sql("SELECT a FROM t INNER JOIN u ON t.id = u.id", RuleAM08);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_am08_flags_multiple_commas() {
        let violations = lint_sql("SELECT a FROM t, u, v", RuleAM08);
        assert_eq!(violations.len(), 2);
    }
}
