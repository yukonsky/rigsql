use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AL03: Expression aliases should have explicit AS keyword.
///
/// When a complex expression (not just a column reference) is aliased,
/// the AS keyword should be present.
#[derive(Debug, Default)]
pub struct RuleAL03;

impl Rule for RuleAL03 {
    fn code(&self) -> &'static str { "AL03" }
    fn name(&self) -> &'static str { "aliasing.expression" }
    fn description(&self) -> &'static str { "Column expression without alias. Use explicit alias." }
    fn explanation(&self) -> &'static str {
        "Complex expressions in SELECT should have an explicit alias using AS. \
         An unlabeled expression like 'SELECT a + b FROM t' is harder to work with \
         than 'SELECT a + b AS total FROM t'. This makes result sets self-documenting."
    }
    fn groups(&self) -> &[RuleGroup] { &[RuleGroup::Aliasing] }
    fn is_fixable(&self) -> bool { false }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();
        let mut violations = Vec::new();

        // Check each direct child of SelectClause
        for child in children {
            let st = child.segment_type();

            // Skip trivia, keywords (SELECT, DISTINCT), commas
            if st.is_trivia() || st == SegmentType::Keyword || st == SegmentType::Comma {
                continue;
            }

            // If it's an expression (not column ref, not alias expr, not star),
            // it should be aliased
            if is_complex_expression(child) && !is_wrapped_in_alias(child, ctx) {
                violations.push(LintViolation::new(
                    self.code(),
                    "Column expression should have an explicit alias.",
                    child.span(),
                ));
            }
        }

        violations
    }
}

fn is_complex_expression(seg: &Segment) -> bool {
    matches!(
        seg.segment_type(),
        SegmentType::BinaryExpression
            | SegmentType::FunctionCall
            | SegmentType::CaseExpression
            | SegmentType::CastExpression
            | SegmentType::ParenExpression
            | SegmentType::UnaryExpression
    )
}

fn is_wrapped_in_alias(seg: &Segment, _ctx: &RuleContext) -> bool {
    // If the segment is a direct child of SelectClause and it's a complex expression,
    // check if there's an AliasExpression wrapping it.
    // Actually, if the segment itself IS an alias expression, it's fine.
    // The grammar wraps aliased items as AliasExpression, so if we see a bare expression,
    // it means it wasn't aliased.
    seg.segment_type() == SegmentType::AliasExpression
}
