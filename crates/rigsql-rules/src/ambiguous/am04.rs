use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AM04: SELECT * should list columns explicitly.
///
/// Using SELECT * is ambiguous because the column set depends on the table
/// definition and may change unexpectedly.
#[derive(Debug, Default)]
pub struct RuleAM04;

impl Rule for RuleAM04 {
    fn code(&self) -> &'static str {
        "AM04"
    }
    fn name(&self) -> &'static str {
        "ambiguous.column_count"
    }
    fn description(&self) -> &'static str {
        "SELECT * should list columns explicitly."
    }
    fn explanation(&self) -> &'static str {
        "Using SELECT * makes the query's column set depend on the table schema, which \
         can change over time. Listing columns explicitly makes the query self-documenting \
         and prevents unexpected changes when columns are added or removed."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Ambiguous]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut violations = Vec::new();
        find_bare_stars(ctx.segment, false, &mut violations);
        violations
    }
}

/// Find Star segments that are NOT inside a FunctionCall (e.g. COUNT(*) is ok).
fn find_bare_stars(segment: &Segment, in_function: bool, violations: &mut Vec<LintViolation>) {
    if segment.segment_type() == SegmentType::Star && !in_function {
        violations.push(LintViolation::new(
            "AM04",
            "SELECT * used. List columns explicitly.",
            segment.span(),
        ));
        return;
    }

    let entering_function = segment.segment_type() == SegmentType::FunctionCall;

    for child in segment.children() {
        find_bare_stars(child, in_function || entering_function, violations);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_am04_flags_select_star() {
        let violations = lint_sql("SELECT * FROM t", RuleAM04);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_am04_accepts_explicit_columns() {
        let violations = lint_sql("SELECT a, b FROM t", RuleAM04);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_am04_accepts_count_star() {
        let violations = lint_sql("SELECT COUNT(*) FROM t", RuleAM04);
        assert_eq!(violations.len(), 0);
    }
}
