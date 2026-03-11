use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// ST10: Constant expression in WHERE clause.
///
/// Detects WHERE clauses with tautological conditions like `WHERE 1 = 1`
/// or `WHERE TRUE`.
#[derive(Debug, Default)]
pub struct RuleST10;

impl Rule for RuleST10 {
    fn code(&self) -> &'static str {
        "ST10"
    }
    fn name(&self) -> &'static str {
        "structure.where_constant"
    }
    fn description(&self) -> &'static str {
        "WHERE clause contains a constant/tautological expression."
    }
    fn explanation(&self) -> &'static str {
        "A WHERE clause with a constant expression like WHERE 1 = 1 or WHERE TRUE \
         is either a placeholder that should be removed, or indicates dead code. \
         Remove the WHERE clause or replace it with a meaningful condition."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Structure]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::WhereClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();
        let non_trivia: Vec<_> = children
            .iter()
            .filter(|s| !s.segment_type().is_trivia())
            .collect();

        // WhereClause: WHERE <expression>
        // non_trivia[0] = Keyword(WHERE), rest = the condition
        if non_trivia.len() < 2 {
            return vec![];
        }

        // Check for single boolean literal: WHERE TRUE / WHERE FALSE
        if non_trivia.len() == 2 && non_trivia[1].segment_type() == SegmentType::BooleanLiteral {
            return vec![LintViolation::new(
                self.code(),
                "WHERE clause contains a constant expression.",
                ctx.segment.span(),
            )];
        }

        // Check for binary expression with both sides being literals (e.g., 1 = 1)
        if non_trivia.len() == 2 {
            if let Some(violation) = check_binary_literal(self.code(), non_trivia[1]) {
                return vec![violation];
            }
        }

        vec![]
    }
}

fn check_binary_literal(code: &'static str, seg: &Segment) -> Option<LintViolation> {
    if seg.segment_type() != SegmentType::BinaryExpression {
        return None;
    }

    let children = seg.children();
    let non_trivia: Vec<_> = children
        .iter()
        .filter(|s| !s.segment_type().is_trivia())
        .collect();

    // BinaryExpression: <left> <operator> <right>
    if non_trivia.len() != 3 {
        return None;
    }

    let left = non_trivia[0];
    let right = non_trivia[2];

    if is_literal(left) && is_literal(right) {
        return Some(LintViolation::new(
            code,
            "WHERE clause contains a constant expression.",
            seg.span(),
        ));
    }

    None
}

fn is_literal(seg: &Segment) -> bool {
    matches!(
        seg.segment_type(),
        SegmentType::NumericLiteral
            | SegmentType::StringLiteral
            | SegmentType::BooleanLiteral
            | SegmentType::NullLiteral
            | SegmentType::Literal
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_st10_flags_where_true() {
        let violations = lint_sql("SELECT * FROM t WHERE TRUE;", RuleST10);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_st10_flags_where_1_eq_1() {
        let violations = lint_sql("SELECT * FROM t WHERE 1 = 1;", RuleST10);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_st10_accepts_normal_where() {
        let violations = lint_sql("SELECT * FROM t WHERE x = 1;", RuleST10);
        assert_eq!(violations.len(), 0);
    }
}
