use std::collections::HashMap;

use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::extract_alias_name;
use crate::violation::LintViolation;

/// AL04: Table aliases should be unique within a statement.
///
/// Duplicate table aliases create ambiguity in column references.
#[derive(Debug, Default)]
pub struct RuleAL04;

impl Rule for RuleAL04 {
    fn code(&self) -> &'static str {
        "AL04"
    }
    fn name(&self) -> &'static str {
        "aliasing.unique_table"
    }
    fn description(&self) -> &'static str {
        "Table aliases should be unique within a statement."
    }
    fn explanation(&self) -> &'static str {
        "When the same alias is used for multiple tables in a single statement, \
         column references become ambiguous. Each table alias must be unique within \
         its containing statement."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Aliasing]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectStatement])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut aliases: Vec<(String, rigsql_core::Span)> = Vec::new();
        collect_table_aliases(ctx.segment, &mut aliases);

        let mut violations = Vec::new();
        let mut seen: HashMap<String, rigsql_core::Span> = HashMap::new();

        for (name, span) in &aliases {
            let lower = name.to_lowercase();
            if let Some(first_span) = seen.get(&lower) {
                violations.push(LintViolation::new(
                    self.code(),
                    format!(
                        "Duplicate table alias '{}'. First used at offset {}.",
                        name, first_span.start,
                    ),
                    *span,
                ));
            } else {
                seen.insert(lower, *span);
            }
        }

        violations
    }
}

/// Collect alias names from FROM and JOIN clauses within a statement.
fn collect_table_aliases(segment: &Segment, aliases: &mut Vec<(String, rigsql_core::Span)>) {
    let st = segment.segment_type();

    // Only look for aliases within FROM and JOIN clauses
    if st == SegmentType::FromClause || st == SegmentType::JoinClause {
        find_alias_names(segment, aliases);
        return;
    }

    // Don't recurse into nested SelectStatements (subqueries have their own scope)
    if st == SegmentType::SelectStatement || st == SegmentType::Subquery {
        // Only recurse into top-level children for the current statement
        // Skip nested selects
        if st == SegmentType::Subquery {
            return;
        }
    }

    for child in segment.children() {
        collect_table_aliases(child, aliases);
    }
}

/// Find AliasExpression nodes and extract the alias name (last identifier).
fn find_alias_names(segment: &Segment, aliases: &mut Vec<(String, rigsql_core::Span)>) {
    if segment.segment_type() == SegmentType::AliasExpression {
        if let Some(name) = extract_alias_name(segment.children()) {
            aliases.push((name, segment.span()));
        }
        return;
    }

    // Don't recurse into subqueries
    if segment.segment_type() == SegmentType::Subquery {
        return;
    }

    for child in segment.children() {
        find_alias_names(child, aliases);
    }
}
