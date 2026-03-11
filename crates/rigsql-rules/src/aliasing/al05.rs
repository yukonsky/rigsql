use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AL05: Tables/CTEs should not be unused.
///
/// Detects WITH clauses where a CTE name is defined but never referenced.
#[derive(Debug, Default)]
pub struct RuleAL05;

impl Rule for RuleAL05 {
    fn code(&self) -> &'static str {
        "AL05"
    }
    fn name(&self) -> &'static str {
        "aliasing.unused"
    }
    fn description(&self) -> &'static str {
        "Tables/CTEs should not be unused."
    }
    fn explanation(&self) -> &'static str {
        "Every CTE (Common Table Expression) defined in a WITH clause should be \
         referenced in the main query or in another CTE. Unused CTEs add complexity \
         without benefit and should be removed."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Aliasing]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::WithClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Collect CTE names
        let mut cte_names: Vec<(String, rigsql_core::Span)> = Vec::new();
        for child in children {
            if child.segment_type() == SegmentType::CteDefinition {
                if let Some(name) = extract_cte_name(child) {
                    cte_names.push((name.to_lowercase(), child.span()));
                }
            }
        }

        if cte_names.is_empty() {
            return vec![];
        }

        // Search the root (File) for references, not just the parent statement.
        // When parsing partially fails, references may end up in sibling Unparsable
        // segments outside the parent SelectStatement.
        let raw = ctx.root.raw().to_lowercase();

        let mut violations = Vec::new();
        for (name, span) in &cte_names {
            // Simple heuristic: check if the CTE name appears elsewhere in the statement
            // beyond its own definition. Count occurrences.
            let count = raw.matches(name.as_str()).count();
            // The name appears at least once in its own definition, so if count <= 1, unused
            if count <= 1 {
                violations.push(LintViolation::new(
                    self.code(),
                    format!("CTE '{}' is defined but not used.", name),
                    *span,
                ));
            }
        }

        violations
    }
}

fn extract_cte_name(cte_def: &Segment) -> Option<String> {
    // CteDefinition children: name (Identifier) [WS] AS [WS] ( subquery )
    for child in cte_def.children() {
        let st = child.segment_type();
        if st == SegmentType::Identifier || st == SegmentType::QuotedIdentifier {
            if let Segment::Token(t) = child {
                return Some(t.token.text.to_string());
            }
        }
        if st == SegmentType::Keyword {
            // Stop at AS keyword
            break;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_al05_flags_unused_cte() {
        let violations = lint_sql(
            "WITH unused AS (SELECT 1) SELECT * FROM other_table",
            RuleAL05,
        );
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_al05_accepts_used_cte() {
        let violations = lint_sql("WITH cte AS (SELECT 1) SELECT * FROM cte", RuleAL05);
        assert_eq!(violations.len(), 0);
    }
}
