use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// LT07: 'WITH' keyword not followed by single space.
///
/// After the WITH keyword there should be exactly one space before the
/// next non-trivia token (the CTE name).
#[derive(Debug, Default)]
pub struct RuleLT07;

impl Rule for RuleLT07 {
    fn code(&self) -> &'static str {
        "LT07"
    }
    fn name(&self) -> &'static str {
        "layout.with_spacing"
    }
    fn description(&self) -> &'static str {
        "'WITH' keyword not followed by single space."
    }
    fn explanation(&self) -> &'static str {
        "The WITH keyword in a Common Table Expression should be followed by exactly \
         one space before the CTE name. Multiple spaces or newlines between WITH and \
         the CTE name reduce readability."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::WithClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Find the WITH keyword
        let mut with_idx = None;
        for (i, child) in children.iter().enumerate() {
            if let Segment::Token(t) = child {
                if t.token.text.as_str().eq_ignore_ascii_case("WITH") {
                    with_idx = Some(i);
                    break;
                }
            }
        }

        let Some(with_idx) = with_idx else {
            return vec![];
        };

        // Collect all trivia between WITH and the next non-trivia token
        let mut trivia_start = None;
        let mut trivia_end = None;
        let mut raw_trivia = String::new();

        for child in &children[with_idx + 1..] {
            let st = child.segment_type();
            if st.is_trivia() {
                if trivia_start.is_none() {
                    trivia_start = Some(child.span());
                }
                trivia_end = Some(child.span());
                raw_trivia.push_str(&child.raw());
            } else {
                break;
            }
        }

        // If the trivia between WITH and next token is exactly one space, it's fine
        if raw_trivia == " " {
            return vec![];
        }

        // If there's no trivia at all, we need to insert a space
        if trivia_start.is_none() {
            let with_span = children[with_idx].span();
            return vec![LintViolation::with_fix(
                self.code(),
                "Expected single space after WITH keyword.",
                with_span,
                vec![SourceEdit::insert(with_span.end, " ")],
            )];
        }

        let start = trivia_start.unwrap();
        let end = trivia_end.unwrap();
        let full_span = start.merge(end);

        vec![LintViolation::with_fix(
            self.code(),
            "Expected single space after WITH keyword.",
            full_span,
            vec![SourceEdit::replace(full_span, " ")],
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_lt07_accepts_proper_spacing() {
        let violations = lint_sql("WITH cte AS (SELECT 1) SELECT * FROM cte", RuleLT07);
        assert_eq!(violations.len(), 0);
    }
}
