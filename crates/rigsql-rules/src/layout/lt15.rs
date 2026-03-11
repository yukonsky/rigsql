use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// LT15: Too many consecutive blank lines.
///
/// Files should not have more than max_blank_lines consecutive blank lines.
#[derive(Debug)]
pub struct RuleLT15 {
    pub max_blank_lines: usize,
}

impl Default for RuleLT15 {
    fn default() -> Self {
        Self { max_blank_lines: 1 }
    }
}

impl Rule for RuleLT15 {
    fn code(&self) -> &'static str {
        "LT15"
    }
    fn name(&self) -> &'static str {
        "layout.newlines"
    }
    fn description(&self) -> &'static str {
        "Too many consecutive blank lines."
    }
    fn explanation(&self) -> &'static str {
        "Files should not contain too many consecutive blank lines. Multiple \
         blank lines waste vertical space and reduce readability."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(val) = settings.get("max_blank_lines") {
            if let Ok(n) = val.parse() {
                self.max_blank_lines = n;
            }
        }
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::RootOnly
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut violations = Vec::new();
        let mut consecutive_newlines = 0usize;
        let mut newline_spans: Vec<rigsql_core::Span> = Vec::new();

        let max = self.max_blank_lines;
        let code = self.code();

        let flush = |consecutive: &mut usize,
                     spans: &mut Vec<rigsql_core::Span>,
                     violations: &mut Vec<LintViolation>| {
            // N consecutive newlines = N-1 blank lines
            if *consecutive > max + 1 {
                let keep = max + 1; // keep this many newlines
                                    // Delete from right after the last kept newline to the end of the
                                    // last excess newline. This captures any whitespace-only content on
                                    // blank lines between them, preventing leftover indent.
                let delete_start = spans[keep - 1].end;
                let delete_end = spans.last().unwrap().end;
                let delete_span = rigsql_core::Span::new(delete_start, delete_end);
                violations.push(LintViolation::with_fix(
                    code,
                    format!("Too many blank lines ({}, max {}).", *consecutive - 1, max),
                    spans[0],
                    vec![SourceEdit::delete(delete_span)],
                ));
            }
            *consecutive = 0;
            spans.clear();
        };

        ctx.root.walk(&mut |seg| {
            if seg.segment_type() == SegmentType::Newline {
                consecutive_newlines += 1;
                newline_spans.push(seg.span());
            } else if seg.segment_type() == SegmentType::Whitespace {
                // Whitespace between newlines doesn't reset the count
            } else if seg.children().is_empty() {
                flush(
                    &mut consecutive_newlines,
                    &mut newline_spans,
                    &mut violations,
                );
            }
        });

        // Check trailing newlines
        flush(
            &mut consecutive_newlines,
            &mut newline_spans,
            &mut violations,
        );

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_lt15_accepts_single_blank_line() {
        let violations = lint_sql("SELECT 1;\n\nSELECT 2;\n", RuleLT15::default());
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_lt15_accepts_no_blank_lines() {
        let violations = lint_sql("SELECT 1;\n", RuleLT15::default());
        assert_eq!(violations.len(), 0);
    }
}
