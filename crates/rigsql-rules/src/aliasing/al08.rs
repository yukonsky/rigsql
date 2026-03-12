use std::collections::HashMap;

use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::extract_alias_name;
use crate::violation::LintViolation;

/// AL08: Column aliases should be unique within each SELECT clause.
///
/// Duplicate column aliases create ambiguity in the result set.
#[derive(Debug, Default)]
pub struct RuleAL08;

impl Rule for RuleAL08 {
    fn code(&self) -> &'static str {
        "AL08"
    }
    fn name(&self) -> &'static str {
        "aliasing.unique.column"
    }
    fn description(&self) -> &'static str {
        "Column aliases should be unique within each statement."
    }
    fn explanation(&self) -> &'static str {
        "When the same alias is used for multiple columns in a single SELECT clause, \
         the result set becomes ambiguous. Each column alias must be unique within \
         its containing SELECT clause."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Aliasing]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut violations = Vec::new();
        let mut seen: HashMap<String, rigsql_core::Span> = HashMap::new();

        for child in ctx.segment.children() {
            if child.segment_type() == SegmentType::AliasExpression {
                if let Some(name) = extract_alias_name(child.children()) {
                    let span = child.span();
                    let lower = name.to_lowercase();
                    if let Some(first_span) = seen.get(&lower) {
                        violations.push(LintViolation::with_msg_key(
                            self.code(),
                            format!(
                                "Duplicate column alias '{}'. First used at offset {}.",
                                name, first_span.start,
                            ),
                            span,
                            "rules.AL08.msg",
                            vec![
                                ("name".to_string(), name.to_string()),
                                ("offset".to_string(), first_span.start.to_string()),
                            ],
                        ));
                    } else {
                        seen.insert(lower, span);
                    }
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_al08_flags_duplicate_column_alias() {
        let violations = lint_sql("SELECT a AS x, b AS x FROM t", RuleAL08);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_al08_accepts_unique_column_aliases() {
        let violations = lint_sql("SELECT a AS x, b AS y FROM t", RuleAL08);
        assert_eq!(violations.len(), 0);
    }
}
