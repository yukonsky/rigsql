use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// CV07: Prefer COALESCE over CASE with IS NULL.
///
/// A CASE expression that tests a single column with IS NULL in the WHEN
/// clause and returns that column in the ELSE clause can be simplified
/// to COALESCE.
///
/// Example:
/// ```sql
/// -- Flagged:
/// CASE WHEN x IS NULL THEN y ELSE x END
/// -- Preferred:
/// COALESCE(x, y)
/// ```
#[derive(Debug, Default)]
pub struct RuleCV07;

impl Rule for RuleCV07 {
    fn code(&self) -> &'static str { "CV07" }
    fn name(&self) -> &'static str { "convention.prefer_coalesce" }
    fn description(&self) -> &'static str { "Prefer COALESCE over CASE with IS NULL pattern." }
    fn explanation(&self) -> &'static str {
        "A CASE expression like 'CASE WHEN x IS NULL THEN y ELSE x END' can be \
         simplified to 'COALESCE(x, y)'. COALESCE is more concise and clearly \
         expresses the intent of providing a fallback value for NULL."
    }
    fn groups(&self) -> &[RuleGroup] { &[RuleGroup::Convention] }
    fn is_fixable(&self) -> bool { false }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::CaseExpression])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // We're looking for the pattern:
        //   CASE WHEN <expr> IS NULL THEN <val> ELSE <expr> END
        //
        // Structure: CaseExpression children typically include:
        //   Keyword(CASE), WhenClause, ElseClause, Keyword(END), plus trivia

        let non_trivia: Vec<_> = children
            .iter()
            .filter(|c| !c.segment_type().is_trivia())
            .collect();

        // Expect: CASE, one WhenClause, one ElseClause, END
        // (at minimum 4 non-trivia children)
        if non_trivia.len() < 4 {
            return vec![];
        }

        // Count WHEN clauses — must be exactly one
        let when_clauses: Vec<_> = non_trivia
            .iter()
            .filter(|c| c.segment_type() == SegmentType::WhenClause)
            .collect();

        if when_clauses.len() != 1 {
            return vec![];
        }

        // Must have an ELSE clause
        let else_clauses: Vec<_> = non_trivia
            .iter()
            .filter(|c| c.segment_type() == SegmentType::ElseClause)
            .collect();

        if else_clauses.len() != 1 {
            return vec![];
        }

        let when_clause = when_clauses[0];
        let else_clause = else_clauses[0];

        // Check if the WHEN clause contains an IS NULL pattern
        // WhenClause children: WHEN, <condition>, THEN, <result>
        let when_children: Vec<_> = when_clause
            .children()
            .iter()
            .filter(|c| !c.segment_type().is_trivia())
            .collect();

        // Look for IsNullExpression in the WHEN condition
        let has_is_null = when_children.iter().any(|c| {
            c.segment_type() == SegmentType::IsNullExpression
        });

        if !has_is_null {
            return vec![];
        }

        // Extract the column being tested for NULL
        let is_null_expr = when_children
            .iter()
            .find(|c| c.segment_type() == SegmentType::IsNullExpression);

        let Some(is_null_expr) = is_null_expr else {
            return vec![];
        };

        let tested_col = get_is_null_subject(is_null_expr);

        // Extract the ELSE expression
        let else_expr = get_else_expression(else_clause);

        // If the column tested for NULL is the same as the ELSE expression,
        // this is a COALESCE pattern
        if let (Some(tested), Some(else_val)) = (tested_col, else_expr) {
            if tested.eq_ignore_ascii_case(&else_val) {
                return vec![LintViolation::new(
                    self.code(),
                    "Use COALESCE instead of CASE WHEN IS NULL pattern.",
                    ctx.segment.span(),
                )];
            }
        }

        vec![]
    }
}

/// Extract the subject of an IS NULL expression (the part before IS NULL).
fn get_is_null_subject(segment: &Segment) -> Option<String> {
    let children = segment.children();
    let non_trivia: Vec<_> = children
        .iter()
        .filter(|c| !c.segment_type().is_trivia())
        .collect();

    // Pattern: <expr> IS NULL
    // First non-trivia child should be the tested expression
    non_trivia.first().map(|s| s.raw().trim().to_string())
}

/// Extract the expression from an ELSE clause (skip ELSE keyword).
fn get_else_expression(segment: &Segment) -> Option<String> {
    let children = segment.children();
    let non_trivia: Vec<_> = children
        .iter()
        .filter(|c| !c.segment_type().is_trivia())
        .collect();

    // First non-trivia is ELSE keyword, rest is the expression
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
