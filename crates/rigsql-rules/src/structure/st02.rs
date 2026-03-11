use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// ST02: Unnecessary CASE expression wrapping a simple boolean.
///
/// A CASE with one WHEN that returns TRUE/FALSE (or 1/0) and an ELSE
/// returning the opposite can be replaced by the condition itself.
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
        "Unnecessary CASE expression wrapping a simple boolean."
    }
    fn explanation(&self) -> &'static str {
        "A CASE expression with a single WHEN clause that returns TRUE/FALSE (or 1/0) \
         and an ELSE clause returning the opposite boolean value is unnecessarily complex. \
         The WHEN condition itself (or its negation) can be used directly."
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

        // Pattern: CASE WHEN <cond> THEN <bool> ELSE <opposite_bool> END
        // non_trivia should be: [Keyword(CASE), WhenClause, ElseClause, Keyword(END)]
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

        let then_value = extract_then_value(when_clauses[0]);
        let else_value = extract_else_value(else_clauses[0]);

        if let (Some(then_val), Some(else_val)) = (then_value, else_value) {
            let is_bool_pair = (is_truthy(&then_val) && is_falsy(&else_val))
                || (is_falsy(&then_val) && is_truthy(&else_val));

            if is_bool_pair {
                return vec![LintViolation::new(
                    self.code(),
                    "Unnecessary CASE expression. Use the boolean condition directly.",
                    ctx.segment.span(),
                )];
            }
        }

        vec![]
    }
}

fn extract_then_value(when_clause: &Segment) -> Option<String> {
    let children = when_clause.children();
    let non_trivia: Vec<_> = children
        .iter()
        .filter(|s| !s.segment_type().is_trivia())
        .collect();

    // WhenClause: WHEN <cond> THEN <value>
    // Find the token after THEN keyword
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

    // ElseClause: ELSE <value>
    // Skip the ELSE keyword, take the next segment
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
}
