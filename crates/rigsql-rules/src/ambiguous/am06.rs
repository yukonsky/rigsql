use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AM06: Inconsistent column references in GROUP BY/ORDER BY.
///
/// GROUP BY and ORDER BY clauses should not mix positional (numeric) references
/// with explicit (named) references. Use one style consistently.
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
        "Inconsistent column references in GROUP BY/ORDER BY."
    }
    fn explanation(&self) -> &'static str {
        "GROUP BY and ORDER BY clauses should use a consistent style for column references: \
         either all positional (e.g., GROUP BY 1, 2) or all explicit column names \
         (e.g., GROUP BY foo, bar). Mixing styles like GROUP BY foo, 2 is ambiguous \
         and hard to maintain."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Ambiguous]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::GroupByClause, SegmentType::OrderByClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut positional = Vec::new();
        let mut named = Vec::new();

        collect_ref_styles(ctx.segment, &mut positional, &mut named);

        // Only flag if there's a mix of styles
        if !positional.is_empty() && !named.is_empty() {
            let clause_name = match ctx.segment.segment_type() {
                SegmentType::GroupByClause => "GROUP BY",
                SegmentType::OrderByClause => "ORDER BY",
                _ => "Clause",
            };

            // Flag the minority style references.
            // If there are more positional than named, flag named ones and vice versa.
            let (targets, style) = if positional.len() >= named.len() {
                (&named, "explicit")
            } else {
                (&positional, "positional")
            };

            return targets
                .iter()
                .map(|span| {
                    LintViolation::new(
                        self.code(),
                        format!(
                            "Mixed positional and explicit references in {}. Found {} reference.",
                            clause_name, style
                        ),
                        *span,
                    )
                })
                .collect();
        }

        vec![]
    }
}

/// Classify references in a GROUP BY or ORDER BY clause as positional (numeric)
/// or named (identifier/expression).
fn collect_ref_styles(
    segment: &Segment,
    positional: &mut Vec<rigsql_core::Span>,
    named: &mut Vec<rigsql_core::Span>,
) {
    for child in segment.children() {
        let st = child.segment_type();
        match st {
            // Skip keywords (GROUP, BY, ORDER, ASC, DESC), trivia, commas
            SegmentType::Keyword
            | SegmentType::Whitespace
            | SegmentType::Newline
            | SegmentType::Comma
            | SegmentType::LineComment
            | SegmentType::BlockComment => {}

            // A bare NumberLiteral is a positional reference
            SegmentType::NumericLiteral => {
                positional.push(child.span());
            }

            // OrderByExpression wraps an expression + optional ASC/DESC
            SegmentType::OrderByExpression => {
                collect_ref_styles(child, positional, named);
            }

            // An expression node: check if it contains only a NumberLiteral
            SegmentType::Expression => {
                if is_single_number_literal(child) {
                    positional.push(child.span());
                } else {
                    named.push(child.span());
                }
            }

            // Identifiers, ColumnRef, QualifiedIdentifier, FunctionCall, etc. → named
            _ => {
                if !child.children().is_empty() {
                    // Node type — check if it's a wrapper around a single number
                    if is_single_number_literal(child) {
                        positional.push(child.span());
                    } else {
                        named.push(child.span());
                    }
                } else {
                    // Leaf token that's not a keyword/trivia → named reference
                    named.push(child.span());
                }
            }
        }
    }
}

/// Check if a segment is (or contains only) a single NumberLiteral.
fn is_single_number_literal(segment: &Segment) -> bool {
    match segment {
        Segment::Token(t) => t.segment_type == SegmentType::NumericLiteral,
        Segment::Node(n) => {
            let mut non_trivia = n.children.iter().filter(|c| !c.segment_type().is_trivia());
            match (non_trivia.next(), non_trivia.next()) {
                (Some(only), None) => is_single_number_literal(only),
                _ => false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_am06_flags_mixed_group_by() {
        // Mixing named 'foo' with positional '2'
        let violations = lint_sql("SELECT foo, bar, SUM(baz) FROM t GROUP BY foo, 2", RuleAM06);
        assert!(!violations.is_empty(), "Should flag mixed GROUP BY styles");
    }

    #[test]
    fn test_am06_accepts_all_explicit_group_by() {
        let violations = lint_sql(
            "SELECT foo, bar, SUM(baz) FROM t GROUP BY foo, bar",
            RuleAM06,
        );
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_am06_accepts_all_positional_group_by() {
        let violations = lint_sql("SELECT foo, bar, SUM(baz) FROM t GROUP BY 1, 2", RuleAM06);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_am06_flags_mixed_order_by() {
        let violations = lint_sql("SELECT a, b FROM t ORDER BY a, 2", RuleAM06);
        assert!(!violations.is_empty(), "Should flag mixed ORDER BY styles");
    }

    #[test]
    fn test_am06_accepts_all_explicit_order_by() {
        let violations = lint_sql("SELECT a, b FROM t ORDER BY a, b", RuleAM06);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_am06_accepts_all_positional_order_by() {
        let violations = lint_sql("SELECT a, b FROM t ORDER BY 1, 2", RuleAM06);
        assert_eq!(violations.len(), 0);
    }
}
