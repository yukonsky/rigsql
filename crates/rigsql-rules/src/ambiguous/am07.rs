use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AM07: UNION/INTERSECT/EXCEPT branches should have matching column counts.
///
/// Checks that set operations have the same number of select items on each side.
#[derive(Debug, Default)]
pub struct RuleAM07;

impl Rule for RuleAM07 {
    fn code(&self) -> &'static str {
        "AM07"
    }
    fn name(&self) -> &'static str {
        "ambiguous.set_column_count"
    }
    fn description(&self) -> &'static str {
        "Set operation column count mismatch."
    }
    fn explanation(&self) -> &'static str {
        "UNION, INTERSECT, and EXCEPT operations require each branch to have the same \
         number of columns. A mismatch will cause a runtime error in most databases. \
         This rule checks that each branch has a consistent number of select items."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Ambiguous]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::RootOnly
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut violations = Vec::new();
        check_set_operations(ctx.root, &mut violations);
        violations
    }
}

fn check_set_operations(segment: &Segment, violations: &mut Vec<LintViolation>) {
    let children = segment.children();

    // Check if this node has set operation keywords among its children
    let has_set_op = children.iter().any(|c| {
        if let Segment::Token(t) = c {
            t.segment_type == SegmentType::Keyword
                && (t.token.text.eq_ignore_ascii_case("UNION")
                    || t.token.text.eq_ignore_ascii_case("INTERSECT")
                    || t.token.text.eq_ignore_ascii_case("EXCEPT"))
        } else {
            false
        }
    });

    if has_set_op {
        // Collect SELECT clauses from SelectStatement children at this level
        let mut select_item_counts = Vec::new();

        for child in children {
            if child.segment_type() == SegmentType::SelectStatement
                || child.segment_type() == SegmentType::SelectClause
            {
                if let Some(count) = count_select_items(child) {
                    select_item_counts.push((child.span(), count));
                }
            }
        }

        // Also check if this segment itself starts with a SelectClause
        // (for the first branch before the UNION keyword)
        if segment.segment_type() == SegmentType::SelectStatement {
            let direct_clause = children
                .iter()
                .find(|c| c.segment_type() == SegmentType::SelectClause);
            if let Some(clause) = direct_clause {
                let count = count_clause_items(clause);
                if count > 0 {
                    select_item_counts.insert(0, (clause.span(), count));
                }
            }
        }

        if select_item_counts.len() >= 2 {
            let first_count = select_item_counts[0].1;
            for (span, count) in &select_item_counts[1..] {
                if *count != first_count {
                    violations.push(LintViolation::new(
                        "AM07",
                        format!(
                            "Set operation column count mismatch: expected {} but found {}.",
                            first_count, count
                        ),
                        *span,
                    ));
                }
            }
        }
    }

    // Recurse
    for child in children {
        check_set_operations(child, violations);
    }
}

/// Count select items in a SelectStatement by finding its SelectClause.
fn count_select_items(segment: &Segment) -> Option<usize> {
    if segment.segment_type() == SegmentType::SelectClause {
        return Some(count_clause_items(segment));
    }

    for child in segment.children() {
        if child.segment_type() == SegmentType::SelectClause {
            return Some(count_clause_items(child));
        }
    }
    None
}

/// Count items in a SelectClause by counting commas + 1.
fn count_clause_items(clause: &Segment) -> usize {
    let commas = clause
        .children()
        .iter()
        .filter(|c| c.segment_type() == SegmentType::Comma)
        .count();
    commas + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_am07_accepts_matching_columns() {
        let violations = lint_sql("SELECT a, b FROM t UNION ALL SELECT c, d FROM u", RuleAM07);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_am07_accepts_single_select() {
        let violations = lint_sql("SELECT a, b FROM t", RuleAM07);
        assert_eq!(violations.len(), 0);
    }
}
