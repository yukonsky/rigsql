use rigsql_core::{Segment, SegmentType, TokenKind};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// CV05: Comparisons with NULL should use IS or IS NOT, not = or !=.
///
/// `WHERE col = NULL` is always false in SQL; use `WHERE col IS NULL` instead.
#[derive(Debug, Default)]
pub struct RuleCV05;

impl Rule for RuleCV05 {
    fn code(&self) -> &'static str { "CV05" }
    fn name(&self) -> &'static str { "convention.is_null" }
    fn description(&self) -> &'static str { "Comparisons with NULL should use IS or IS NOT." }
    fn explanation(&self) -> &'static str {
        "In SQL, NULL is not a value but the absence of one. Comparison operators \
         (=, !=, <>) with NULL always return NULL (which is falsy). Use 'IS NULL' or \
         'IS NOT NULL' instead of '= NULL' or '!= NULL'."
    }
    fn groups(&self) -> &[RuleGroup] { &[RuleGroup::Convention] }
    fn is_fixable(&self) -> bool { true }

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
        let is_comparison = matches!(
            op.segment_type(),
            SegmentType::ComparisonOperator
        );
        if !is_comparison {
            return vec![];
        }

        // Check if either side is NULL literal
        let lhs_is_null = is_null_literal(non_trivia[0]);
        let rhs_is_null = non_trivia.get(2).is_some_and(|s| is_null_literal(s));

        if lhs_is_null || rhs_is_null {
            return vec![LintViolation::new(
                self.code(),
                "Use IS NULL or IS NOT NULL instead of comparison operator with NULL.",
                ctx.segment.span(),
            )];
        }

        vec![]
    }
}

fn is_null_literal(seg: &Segment) -> bool {
    match seg {
        Segment::Token(t) => {
            t.segment_type == SegmentType::NullLiteral
                || (t.token.kind == TokenKind::Word
                    && t.token.text.eq_ignore_ascii_case("NULL"))
        }
        Segment::Node(n) => {
            n.segment_type == SegmentType::NullLiteral
        }
    }
}
