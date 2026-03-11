use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// ST02: Unnecessary CASE expression.
///
/// Detects two patterns:
/// 1. Boolean wrapping: CASE WHEN cond THEN TRUE ELSE FALSE END → use cond directly
/// 2. IS NULL fallback: CASE WHEN x IS NULL THEN y ELSE x END → use COALESCE(x, y)
#[derive(Debug, Default)]
pub struct RuleST02;

impl Rule for RuleST02 {
    fn code(&self) -> &'static str {
        "ST02"
    }
    fn name(&self) -> &'static str {
        "structure.simple_case"
    }
    fn description(&self) -> &'static str {
        "Unnecessary CASE expression."
    }
    fn explanation(&self) -> &'static str {
        "A CASE expression is unnecessary when it can be replaced by a simpler construct: \
         (1) A single WHEN returning TRUE/FALSE (or 1/0) with an opposite ELSE can use the \
         condition directly. (2) A CASE WHEN x IS NULL THEN y ELSE x END can be replaced \
         with COALESCE(x, y)."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Structure]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::CaseExpression])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();
        let non_trivia: Vec<_> = children
            .iter()
            .filter(|s| !s.segment_type().is_trivia())
            .collect();

        // Must have exactly one WHEN and one ELSE
        let when_clauses: Vec<_> = non_trivia
            .iter()
            .filter(|s| s.segment_type() == SegmentType::WhenClause)
            .collect();
        let else_clauses: Vec<_> = non_trivia
            .iter()
            .filter(|s| s.segment_type() == SegmentType::ElseClause)
            .collect();

        if when_clauses.len() != 1 || else_clauses.len() != 1 {
            return vec![];
        }

        let when_clause = when_clauses[0];
        let else_clause = else_clauses[0];

        // Pattern 1: Boolean wrapping (CASE WHEN cond THEN TRUE ELSE FALSE END)
        let then_value = extract_then_value(when_clause);
        let else_value = extract_else_value(else_clause);

        if let (Some(ref then_val), Some(ref else_val)) = (then_value, else_value) {
            let is_bool_pair = (is_truthy(then_val) && is_falsy(else_val))
                || (is_falsy(then_val) && is_truthy(else_val));

            if is_bool_pair {
                return vec![LintViolation::new(
                    self.code(),
                    "Unnecessary CASE expression. Use the boolean condition directly.",
                    ctx.segment.span(),
                )];
            }
        }

        // Pattern 2: IS NULL fallback (CASE WHEN x IS NULL THEN y ELSE x END)
        if let Some(msg) = check_is_null_coalesce_pattern(when_clause, else_clause) {
            return vec![LintViolation::new(self.code(), msg, ctx.segment.span())];
        }

        vec![]
    }
}

/// Check for CASE WHEN x IS NULL THEN y ELSE x END → COALESCE(x, y) pattern.
fn check_is_null_coalesce_pattern(when_clause: &Segment, else_clause: &Segment) -> Option<String> {
    let when_children: Vec<_> = when_clause
        .children()
        .iter()
        .filter(|c| !c.segment_type().is_trivia())
        .collect();

    // Find IS NULL expression in the WHEN clause
    let is_null_expr = when_children
        .iter()
        .find(|c| c.segment_type() == SegmentType::IsNullExpression)?;

    // Get the subject of IS NULL (the column/expression being tested)
    let tested_col = get_is_null_subject(is_null_expr)?;

    // Get the ELSE value
    let else_expr = get_else_expression(else_clause)?;

    // If the ELSE value matches the tested column, it's the COALESCE pattern
    if tested_col.eq_ignore_ascii_case(&else_expr) {
        Some(
            "Unnecessary CASE expression. Use COALESCE instead of CASE WHEN IS NULL pattern."
                .to_string(),
        )
    } else {
        None
    }
}

/// Extract the subject of an IS NULL expression (the part before IS NULL).
fn get_is_null_subject(segment: &Segment) -> Option<String> {
    let children = segment.children();
    let non_trivia: Vec<_> = children
        .iter()
        .filter(|c| !c.segment_type().is_trivia())
        .collect();

    non_trivia.first().map(|s| s.raw().trim().to_string())
}

/// Extract the expression from an ELSE clause (skip ELSE keyword).
fn get_else_expression(segment: &Segment) -> Option<String> {
    let children = segment.children();
    let non_trivia: Vec<_> = children
        .iter()
        .filter(|c| !c.segment_type().is_trivia())
        .collect();

    if non_trivia.len() >= 2 {
        let expr_parts: String = non_trivia[1..]
            .iter()
            .map(|s| s.raw())
            .collect::<Vec<_>>()
            .join("");
        Some(expr_parts.trim().to_string())
    } else {
        None
    }
}

fn extract_then_value(when_clause: &Segment) -> Option<String> {
    let children = when_clause.children();
    let non_trivia: Vec<_> = children
        .iter()
        .filter(|s| !s.segment_type().is_trivia())
        .collect();

    let mut found_then = false;
    for seg in &non_trivia {
        if found_then {
            return Some(seg.raw().trim().to_uppercase());
        }
        if seg.segment_type() == SegmentType::Keyword && seg.raw().eq_ignore_ascii_case("THEN") {
            found_then = true;
        }
    }
    None
}

fn extract_else_value(else_clause: &Segment) -> Option<String> {
    let children = else_clause.children();
    let non_trivia: Vec<_> = children
        .iter()
        .filter(|s| !s.segment_type().is_trivia())
        .collect();

    if non_trivia.len() >= 2 {
        return Some(non_trivia[1].raw().trim().to_uppercase());
    }
    None
}

fn is_truthy(val: &str) -> bool {
    val == "TRUE" || val == "1"
}

fn is_falsy(val: &str) -> bool {
    val == "FALSE" || val == "0"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_st02_flags_simple_boolean_case() {
        let violations = lint_sql("SELECT CASE WHEN x > 0 THEN TRUE ELSE FALSE END;", RuleST02);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("boolean"));
    }

    #[test]
    fn test_st02_accepts_non_boolean_case() {
        let violations = lint_sql("SELECT CASE WHEN x > 0 THEN 'yes' ELSE 'no' END;", RuleST02);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_st02_accepts_multi_when() {
        let violations = lint_sql(
            "SELECT CASE WHEN x > 0 THEN TRUE WHEN x < 0 THEN FALSE ELSE FALSE END;",
            RuleST02,
        );
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_st02_flags_is_null_coalesce_pattern() {
        let violations = lint_sql(
            "SELECT CASE WHEN col IS NULL THEN 'default' ELSE col END FROM t",
            RuleST02,
        );
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("COALESCE"));
    }

    #[test]
    fn test_st02_accepts_complex_case() {
        let violations = lint_sql(
            "SELECT CASE WHEN x = 1 THEN 'a' ELSE 'b' END FROM t",
            RuleST02,
        );
        assert_eq!(violations.len(), 0);
    }
}
