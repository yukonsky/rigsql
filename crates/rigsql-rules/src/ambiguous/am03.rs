use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AM03: ORDER BY column with ambiguous direction.
///
/// When some ORDER BY expressions have explicit ASC/DESC and others don't,
/// the inconsistency is confusing. Either specify direction for all or none.
#[derive(Debug, Default)]
pub struct RuleAM03;

impl Rule for RuleAM03 {
    fn code(&self) -> &'static str {
        "AM03"
    }
    fn name(&self) -> &'static str {
        "ambiguous.order_by"
    }
    fn description(&self) -> &'static str {
        "Inconsistent ORDER BY direction."
    }
    fn explanation(&self) -> &'static str {
        "When an ORDER BY clause has multiple columns, mixing explicit (ASC/DESC) and \
         implicit sort directions is confusing. If some columns have an explicit direction, \
         all columns should have one for consistency and clarity."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Ambiguous]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::OrderByClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Collect OrderByExpression children
        let order_exprs: Vec<_> = children
            .iter()
            .filter(|c| c.segment_type() == SegmentType::OrderByExpression)
            .collect();

        if order_exprs.len() < 2 {
            return vec![];
        }

        // Check which ones have explicit sort order (ASC/DESC keyword or SortOrder node)
        let has_direction: Vec<bool> = order_exprs
            .iter()
            .map(|expr| has_explicit_direction(expr))
            .collect();

        let any_explicit = has_direction.iter().any(|&d| d);
        let all_explicit = has_direction.iter().all(|&d| d);

        // If some have and some don't, flag the ones without
        if any_explicit && !all_explicit {
            return order_exprs
                .iter()
                .zip(has_direction.iter())
                .filter(|(_, &has)| !has)
                .map(|(expr, _)| {
                    LintViolation::new(
                        self.code(),
                        "ORDER BY column without explicit ASC/DESC when other columns have one.",
                        expr.span(),
                    )
                })
                .collect();
        }

        vec![]
    }
}

/// Check if an OrderByExpression has an explicit ASC or DESC.
fn has_explicit_direction(expr: &Segment) -> bool {
    expr.children().iter().any(|c| {
        // Check for SortOrder node
        if c.segment_type() == SegmentType::SortOrder {
            return true;
        }
        // Check for bare ASC/DESC keyword
        if let Segment::Token(t) = c {
            if t.segment_type == SegmentType::Keyword
                && (t.token.text.eq_ignore_ascii_case("ASC")
                    || t.token.text.eq_ignore_ascii_case("DESC"))
            {
                return true;
            }
        }
        false
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_am03_flags_inconsistent_direction() {
        let violations = lint_sql("SELECT a, b FROM t ORDER BY a ASC, b", RuleAM03);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_am03_accepts_all_explicit() {
        let violations = lint_sql("SELECT a, b FROM t ORDER BY a ASC, b DESC", RuleAM03);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_am03_accepts_all_implicit() {
        let violations = lint_sql("SELECT a, b FROM t ORDER BY a, b", RuleAM03);
        assert_eq!(violations.len(), 0);
    }
}
