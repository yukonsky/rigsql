use rigsql_core::{Segment, SegmentType, TokenKind};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// CV05: Comparisons with NULL should use IS or IS NOT, not = or !=.
///
/// `WHERE col = NULL` is always false in SQL; use `WHERE col IS NULL` instead.
#[derive(Debug, Default)]
pub struct RuleCV05;

impl Rule for RuleCV05 {
    fn code(&self) -> &'static str {
        "CV05"
    }
    fn name(&self) -> &'static str {
        "convention.is_null"
    }
    fn description(&self) -> &'static str {
        "Comparisons with NULL should use IS or IS NOT."
    }
    fn explanation(&self) -> &'static str {
        "In SQL, NULL is not a value but the absence of one. Comparison operators \
         (=, !=, <>) with NULL always return NULL (which is falsy). Use 'IS NULL' or \
         'IS NOT NULL' instead of '= NULL' or '!= NULL'."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::BinaryExpression])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Look for pattern: <expr> <comparison_op> NULL
        // or: NULL <comparison_op> <expr>
        let non_trivia: Vec<_> = children
            .iter()
            .filter(|c| !c.segment_type().is_trivia())
            .collect();

        if non_trivia.len() < 3 {
            return vec![];
        }

        // Check if operator is comparison (=, !=, <>)
        let op = non_trivia[1];
        let is_comparison = matches!(op.segment_type(), SegmentType::ComparisonOperator);
        if !is_comparison {
            return vec![];
        }

        // Check if either side is NULL literal
        let lhs_is_null = is_null_literal(non_trivia[0]);
        let rhs_is_null = non_trivia.get(2).is_some_and(|s| is_null_literal(s));

        if lhs_is_null || rhs_is_null {
            // Determine the operator text to decide IS NULL vs IS NOT NULL
            let op_text = op.tokens().first().map(|t| t.text.as_str()).unwrap_or("=");
            let is_negated = op_text == "!=" || op_text == "<>";

            // Build the fix: replace "op NULL" or "NULL op" portion
            // For "expr = NULL" → "expr IS NULL"
            // For "expr != NULL" → "expr IS NOT NULL"
            let op_span = op.span();
            let null_seg = if rhs_is_null {
                non_trivia[2]
            } else {
                non_trivia[0]
            };
            let null_span = null_seg.span();

            let fix = if rhs_is_null {
                // "expr <op> <ws?> NULL" → replace from op start to NULL end
                let replace_span = rigsql_core::Span::new(op_span.start, null_span.end);
                let replacement = if is_negated { "IS NOT NULL" } else { "IS NULL" };
                vec![SourceEdit::replace(replace_span, replacement)]
            } else {
                // "NULL <op> <ws?> expr" → rearrange to "expr IS [NOT] NULL"
                let is_not = if is_negated { "IS NOT NULL" } else { "IS NULL" };
                let expr = non_trivia[2];
                let whole_span = ctx.segment.span();
                let expr_text = ctx
                    .source
                    .get(expr.span().start as usize..expr.span().end as usize)
                    .unwrap_or("?");
                vec![SourceEdit::replace(
                    whole_span,
                    format!("{} {}", expr_text, is_not),
                )]
            };

            return vec![LintViolation::with_fix_and_msg_key(
                self.code(),
                "Use IS NULL or IS NOT NULL instead of comparison operator with NULL.",
                ctx.segment.span(),
                fix,
                "rules.CV05.msg",
                vec![],
            )];
        }

        vec![]
    }
}

fn is_null_literal(seg: &Segment) -> bool {
    match seg {
        Segment::Token(t) => {
            t.segment_type == SegmentType::NullLiteral
                || (t.token.kind == TokenKind::Word && t.token.text.eq_ignore_ascii_case("NULL"))
        }
        Segment::Node(n) => n.segment_type == SegmentType::NullLiteral,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_cv05_flags_equals_null() {
        let violations = lint_sql("SELECT * FROM t WHERE a = NULL", RuleCV05);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "IS NULL");
    }

    #[test]
    fn test_cv05_accepts_is_null() {
        let violations = lint_sql("SELECT * FROM t WHERE a IS NULL", RuleCV05);
        assert_eq!(violations.len(), 0);
    }
}
