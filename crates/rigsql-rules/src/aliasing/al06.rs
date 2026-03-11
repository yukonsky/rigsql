use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AL06: Subqueries in FROM clause should have an alias.
///
/// A subquery used as a table source must be given an explicit alias
/// so that its columns can be referenced unambiguously.
#[derive(Debug, Default)]
pub struct RuleAL06;

impl Rule for RuleAL06 {
    fn code(&self) -> &'static str {
        "AL06"
    }
    fn name(&self) -> &'static str {
        "aliasing.subquery"
    }
    fn description(&self) -> &'static str {
        "Subqueries in FROM clause should have an alias."
    }
    fn explanation(&self) -> &'static str {
        "When a subquery is used as a table source in a FROM or JOIN clause, \
         it must be given an explicit alias. Without an alias, columns from the \
         subquery cannot be referenced clearly in the outer query."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Aliasing]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::FromClause, SegmentType::JoinClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut violations = Vec::new();
        check_subqueries_have_alias(ctx.segment, &mut violations, self.code());
        violations
    }
}

fn check_subqueries_have_alias(
    segment: &Segment,
    violations: &mut Vec<LintViolation>,
    code: &'static str,
) {
    let children = segment.children();

    for child in children {
        let st = child.segment_type();

        // A bare Subquery (not wrapped in AliasExpression) lacks an alias
        if st == SegmentType::Subquery {
            violations.push(LintViolation::new(
                code,
                "Subquery in FROM/JOIN clause should have an alias.",
                child.span(),
            ));
            continue;
        }

        // AliasExpression wraps aliased subqueries — skip (already aliased)
        if st == SegmentType::AliasExpression {
            continue;
        }

        // Recurse into other nodes (e.g., JoinClause inside FromClause)
        check_subqueries_have_alias(child, violations, code);
    }
}
