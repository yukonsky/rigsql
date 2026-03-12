use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// CV12: Use JOIN … ON … instead of WHERE … for join conditions.
///
/// When FROM has comma-separated tables and WHERE contains join conditions,
/// prefer explicit JOIN syntax.
#[derive(Debug, Default)]
pub struct RuleCV12;

impl Rule for RuleCV12 {
    fn code(&self) -> &'static str {
        "CV12"
    }
    fn name(&self) -> &'static str {
        "convention.join_condition"
    }
    fn description(&self) -> &'static str {
        "Use JOIN … ON … instead of implicit join in WHERE."
    }
    fn explanation(&self) -> &'static str {
        "Using comma-separated tables in FROM with join conditions in WHERE (implicit join) \
         mixes join logic with filtering. Use explicit JOIN … ON … syntax to separate join \
         conditions from filter conditions, improving readability and maintainability."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectStatement])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Find FROM clause
        let from_clause = children
            .iter()
            .find(|c| c.segment_type() == SegmentType::FromClause);
        let where_clause = children
            .iter()
            .find(|c| c.segment_type() == SegmentType::WhereClause);

        let (Some(from), Some(where_seg)) = (from_clause, where_clause) else {
            return vec![];
        };

        // Check if FROM has comma-separated tables (contains Comma)
        let has_comma = from
            .children()
            .iter()
            .any(|c| c.segment_type() == SegmentType::Comma);

        if !has_comma {
            return vec![];
        }

        // FROM has comma-separated tables + WHERE exists → implicit join
        vec![LintViolation::with_msg_key(
            self.code(),
            "Use explicit JOIN … ON … instead of comma-separated tables with WHERE.",
            where_seg.span(),
            "rules.CV12.msg",
            vec![],
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_cv12_flags_implicit_join() {
        let violations = lint_sql("SELECT * FROM a, b WHERE a.id = b.id", RuleCV12);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_cv12_accepts_explicit_join() {
        let violations = lint_sql("SELECT * FROM a JOIN b ON a.id = b.id", RuleCV12);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cv12_accepts_single_table_where() {
        let violations = lint_sql("SELECT * FROM t WHERE x = 1", RuleCV12);
        assert_eq!(violations.len(), 0);
    }
}
