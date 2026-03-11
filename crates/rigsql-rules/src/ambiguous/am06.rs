use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AM06: Inconsistent column references (qualified vs unqualified).
///
/// If some column references use table qualifiers and others don't,
/// the unqualified ones are flagged as ambiguous.
#[derive(Debug, Default)]
pub struct RuleAM06;

impl Rule for RuleAM06 {
    fn code(&self) -> &'static str {
        "AM06"
    }
    fn name(&self) -> &'static str {
        "ambiguous.column_references"
    }
    fn description(&self) -> &'static str {
        "Inconsistent column references."
    }
    fn explanation(&self) -> &'static str {
        "When a query mixes qualified (table.column) and unqualified (column) references, \
         the unqualified ones are ambiguous, especially when multiple tables are involved. \
         Either qualify all column references or none for consistency."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Ambiguous]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectStatement])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut qualified = Vec::new();
        let mut unqualified = Vec::new();

        collect_column_refs(ctx.segment, &mut qualified, &mut unqualified);

        // Only flag if there's a mix
        if !qualified.is_empty() && !unqualified.is_empty() {
            return unqualified
                .into_iter()
                .map(|span| {
                    LintViolation::new(
                        self.code(),
                        "Unqualified column reference when other references are qualified.",
                        span,
                    )
                })
                .collect();
        }

        vec![]
    }
}

/// Context for determining if an Identifier is likely a column reference.
const COLUMN_CONTEXTS: &[SegmentType] = &[
    SegmentType::SelectClause,
    SegmentType::WhereClause,
    SegmentType::HavingClause,
    SegmentType::OrderByClause,
    SegmentType::GroupByClause,
    SegmentType::OnClause,
    SegmentType::OrderByExpression,
    SegmentType::BinaryExpression,
];

fn collect_column_refs(
    segment: &Segment,
    qualified: &mut Vec<rigsql_core::Span>,
    unqualified: &mut Vec<rigsql_core::Span>,
) {
    // Skip subqueries to avoid cross-scope confusion
    if segment.segment_type() == SegmentType::Subquery {
        return;
    }

    match segment.segment_type() {
        SegmentType::QualifiedIdentifier | SegmentType::ColumnRef => {
            // Check if it contains a Dot (meaning it's table.column)
            let has_dot = segment
                .children()
                .iter()
                .any(|c| c.segment_type() == SegmentType::Dot);
            if has_dot {
                qualified.push(segment.span());
            } else {
                unqualified.push(segment.span());
            }
            return; // Don't recurse into children
        }
        _ => {}
    }

    // In column-relevant contexts, bare Identifiers are likely column references
    if COLUMN_CONTEXTS.contains(&segment.segment_type()) {
        for child in segment.children() {
            if child.segment_type() == SegmentType::Identifier {
                unqualified.push(child.span());
            } else {
                collect_column_refs(child, qualified, unqualified);
            }
        }
        return;
    }

    for child in segment.children() {
        collect_column_refs(child, qualified, unqualified);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_am06_flags_mixed_references() {
        // t.a is a qualified ColumnRef (has Dot), b is a bare Identifier (unqualified)
        let violations = lint_sql("SELECT t.a, b FROM t INNER JOIN u ON t.id = u.id", RuleAM06);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_am06_accepts_all_qualified() {
        let violations = lint_sql(
            "SELECT t.a, t.b FROM t INNER JOIN u ON t.id = u.id",
            RuleAM06,
        );
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_am06_accepts_all_unqualified() {
        let violations = lint_sql("SELECT a, b FROM t", RuleAM06);
        assert_eq!(violations.len(), 0);
    }
}
