use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// LT06: Function name not followed immediately by parenthesis.
///
/// There should be no whitespace between the function name and the opening
/// parenthesis, e.g. `COUNT(*)` not `COUNT (*)`.
#[derive(Debug, Default)]
pub struct RuleLT06;

impl Rule for RuleLT06 {
    fn code(&self) -> &'static str {
        "LT06"
    }
    fn name(&self) -> &'static str {
        "layout.functions"
    }
    fn description(&self) -> &'static str {
        "Function name not followed immediately by parenthesis."
    }
    fn explanation(&self) -> &'static str {
        "In SQL, function calls should have the opening parenthesis immediately after \
         the function name with no intervening whitespace. For example, write COUNT(*) \
         rather than COUNT (*)."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::FunctionCall])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Find the function name identifier (first non-trivia child)
        let mut name_idx = None;
        for (i, child) in children.iter().enumerate() {
            if !child.segment_type().is_trivia() {
                name_idx = Some(i);
                break;
            }
        }

        let Some(name_idx) = name_idx else {
            return vec![];
        };

        // Collect whitespace segments between the function name and LParen
        let mut whitespace_spans = Vec::new();
        let mut found_lparen = false;

        for child in &children[name_idx + 1..] {
            let st = child.segment_type();
            if st == SegmentType::Whitespace || st == SegmentType::Newline {
                whitespace_spans.push(child.span());
            } else if st == SegmentType::LParen || st == SegmentType::FunctionArgs {
                found_lparen = true;
                break;
            } else {
                // Non-trivia, non-lparen — not the pattern we're looking for
                break;
            }
        }

        if !found_lparen || whitespace_spans.is_empty() {
            return vec![];
        }

        let fixes: Vec<SourceEdit> = whitespace_spans
            .iter()
            .map(|span| SourceEdit::delete(*span))
            .collect();

        let first_ws = whitespace_spans[0];
        vec![LintViolation::with_fix(
            self.code(),
            "No whitespace allowed between function name and parenthesis.",
            first_ws,
            fixes,
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_lt06_space_before_paren_not_detected_yet() {
        let violations = lint_sql("SELECT COUNT (*) FROM t", RuleLT06);
        // NOTE: Parser does not recognize "COUNT (...)" (with space) as a FunctionCall,
        // so the rule cannot fire. This is a known parser limitation.
        // When the parser is improved to handle this case, this test should be updated.
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_lt06_accepts_no_space() {
        let violations = lint_sql("SELECT COUNT(*) FROM t", RuleLT06);
        assert_eq!(violations.len(), 0);
    }
}
