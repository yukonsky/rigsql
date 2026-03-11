use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AM01: DISTINCT used with GROUP BY is redundant.
///
/// GROUP BY already produces unique rows for the grouped columns,
/// so adding DISTINCT is ambiguous and potentially misleading.
#[derive(Debug, Default)]
pub struct RuleAM01;

impl Rule for RuleAM01 {
    fn code(&self) -> &'static str {
        "AM01"
    }
    fn name(&self) -> &'static str {
        "ambiguous.distinct"
    }
    fn description(&self) -> &'static str {
        "DISTINCT used with GROUP BY."
    }
    fn explanation(&self) -> &'static str {
        "Using DISTINCT together with GROUP BY is redundant because GROUP BY already \
         deduplicates the result set for the grouped columns. This combination is \
         ambiguous and suggests the author may not fully understand the query semantics."
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
        let children = ctx.segment.children();

        // Check if there's a GROUP BY clause
        let has_group_by = children
            .iter()
            .any(|c| c.segment_type() == SegmentType::GroupByClause);

        if !has_group_by {
            return vec![];
        }

        // Check if the SelectClause has a DISTINCT keyword
        let select_clause = children
            .iter()
            .find(|c| c.segment_type() == SegmentType::SelectClause);

        if let Some(select) = select_clause {
            let distinct_token = find_distinct_keyword(select);
            if let Some(span) = distinct_token {
                return vec![LintViolation::new(
                    self.code(),
                    "DISTINCT is redundant when used with GROUP BY.",
                    span,
                )];
            }
        }

        vec![]
    }
}

fn find_distinct_keyword(select_clause: &Segment) -> Option<rigsql_core::Span> {
    for child in select_clause.children() {
        if let Segment::Token(t) = child {
            if t.segment_type == SegmentType::Keyword
                && t.token.text.eq_ignore_ascii_case("DISTINCT")
            {
                return Some(t.token.span);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_am01_flags_distinct_with_group_by() {
        let violations = lint_sql("SELECT DISTINCT a FROM t GROUP BY a", RuleAM01);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("DISTINCT"));
    }

    #[test]
    fn test_am01_accepts_distinct_without_group_by() {
        let violations = lint_sql("SELECT DISTINCT a FROM t", RuleAM01);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_am01_accepts_group_by_without_distinct() {
        let violations = lint_sql("SELECT a FROM t GROUP BY a", RuleAM01);
        assert_eq!(violations.len(), 0);
    }
}
