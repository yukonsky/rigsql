use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AM02: UNION without DISTINCT or ALL is ambiguous.
///
/// Bare UNION implicitly means UNION DISTINCT, but this should be made
/// explicit to avoid confusion.
#[derive(Debug, Default)]
pub struct RuleAM02;

impl Rule for RuleAM02 {
    fn code(&self) -> &'static str {
        "AM02"
    }
    fn name(&self) -> &'static str {
        "ambiguous.union"
    }
    fn description(&self) -> &'static str {
        "UNION without DISTINCT or ALL."
    }
    fn explanation(&self) -> &'static str {
        "A bare UNION (without ALL or DISTINCT) implicitly deduplicates results, \
         which is equivalent to UNION DISTINCT. This implicit behavior can be confusing. \
         Use UNION ALL when you want all rows, or UNION DISTINCT to make the dedup explicit."
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
        find_bare_unions(ctx.root, &mut violations);
        violations
    }
}

fn find_bare_unions(segment: &Segment, violations: &mut Vec<LintViolation>) {
    let children = segment.children();

    for (i, child) in children.iter().enumerate() {
        if let Segment::Token(t) = child {
            if t.segment_type == SegmentType::Keyword && t.token.text.eq_ignore_ascii_case("UNION")
            {
                // Check if the next non-trivia sibling is ALL or DISTINCT
                let next = children[i + 1..]
                    .iter()
                    .find(|s| !s.segment_type().is_trivia());

                let has_qualifier = next.is_some_and(|s| {
                    if let Segment::Token(nt) = s {
                        nt.token.text.eq_ignore_ascii_case("ALL")
                            || nt.token.text.eq_ignore_ascii_case("DISTINCT")
                    } else {
                        false
                    }
                });

                if !has_qualifier {
                    violations.push(LintViolation::new(
                        "AM02",
                        "UNION without explicit DISTINCT or ALL.",
                        t.token.span,
                    ));
                }
            }
        }

        // Recurse into children
        find_bare_unions(child, violations);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_am02_flags_bare_union() {
        let violations = lint_sql("SELECT a FROM t UNION SELECT b FROM u", RuleAM02);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("UNION"));
    }

    #[test]
    fn test_am02_accepts_union_all() {
        let violations = lint_sql("SELECT a FROM t UNION ALL SELECT b FROM u", RuleAM02);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_am02_accepts_union_distinct() {
        let violations = lint_sql("SELECT a FROM t UNION DISTINCT SELECT b FROM u", RuleAM02);
        assert_eq!(violations.len(), 0);
    }
}
