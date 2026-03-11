use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// RG02: Consistent use of NULL in UNION.
///
/// Bare NULL literals in UNION SELECT items should have an explicit type cast
/// for clarity and consistency.
#[derive(Debug, Default)]
pub struct RuleRG02;

impl Rule for RuleRG02 {
    fn code(&self) -> &'static str {
        "RG02"
    }
    fn name(&self) -> &'static str {
        "rigsql.union_null"
    }
    fn description(&self) -> &'static str {
        "Consistent use of NULL in UNION."
    }
    fn explanation(&self) -> &'static str {
        "When using NULL in a UNION query, the type of the NULL column is ambiguous. \
         Use an explicit CAST (e.g. CAST(NULL AS INTEGER)) to make the intent clear \
         and avoid potential type-mismatch issues across UNION branches."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::RootOnly
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut violations = Vec::new();
        find_union_nulls(ctx.root, false, &mut violations);
        violations
    }
}

/// Recursively walk the tree looking for SelectStatements that are part of a
/// UNION. When we find one, scan its SelectClause items for bare NullLiterals.
fn find_union_nulls(segment: &Segment, in_union: bool, violations: &mut Vec<LintViolation>) {
    let children = segment.children();

    let has_union = children.iter().any(|c| {
        if let Segment::Token(t) = c {
            t.token.text.eq_ignore_ascii_case("UNION")
        } else {
            false
        }
    });

    let union_context = in_union || has_union;

    for child in children {
        if union_context && child.segment_type() == SegmentType::SelectStatement {
            check_select_for_bare_null(child, violations);
        }

        if child.segment_type() == SegmentType::SelectClause && union_context {
            check_clause_for_bare_null(child, violations);
        }

        find_union_nulls(child, union_context, violations);
    }
}

fn check_select_for_bare_null(select: &Segment, violations: &mut Vec<LintViolation>) {
    select.walk(&mut |seg| {
        if seg.segment_type() == SegmentType::SelectClause {
            check_clause_for_bare_null(seg, violations);
        }
    });
}

fn check_clause_for_bare_null(clause: &Segment, violations: &mut Vec<LintViolation>) {
    for child in clause.children() {
        find_bare_nulls(child, violations);
    }
}

/// Walk looking for NullLiterals that are NOT inside a CastExpression.
fn find_bare_nulls(segment: &Segment, violations: &mut Vec<LintViolation>) {
    if segment.segment_type() == SegmentType::CastExpression {
        return;
    }

    if segment.segment_type() == SegmentType::NullLiteral {
        violations.push(LintViolation::new(
            "RG02",
            "Bare NULL in UNION. Consider using an explicit CAST.",
            segment.span(),
        ));
        return;
    }

    for child in segment.children() {
        find_bare_nulls(child, violations);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_rg02_accepts_non_union() {
        let violations = lint_sql("SELECT NULL FROM t", RuleRG02);
        assert_eq!(violations.len(), 0);
    }
}
