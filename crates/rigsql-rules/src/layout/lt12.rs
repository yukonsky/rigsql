use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// LT12: Files must end with a single trailing newline.
#[derive(Debug, Default)]
pub struct RuleLT12;

impl Rule for RuleLT12 {
    fn code(&self) -> &'static str { "LT12" }
    fn name(&self) -> &'static str { "layout.end_of_file" }
    fn description(&self) -> &'static str { "Files must end with a single trailing newline." }
    fn explanation(&self) -> &'static str {
        "Files should end with exactly one newline character. Missing trailing newlines \
         can cause issues with some tools, and multiple trailing newlines are untidy."
    }
    fn groups(&self) -> &[RuleGroup] { &[RuleGroup::Layout] }
    fn is_fixable(&self) -> bool { true }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::RootOnly
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let source = ctx.source;
        if source.is_empty() {
            return vec![];
        }

        let end = source.len() as u32;

        if !source.ends_with('\n') {
            return vec![LintViolation::with_fix(
                self.code(),
                "File does not end with a trailing newline.",
                rigsql_core::Span::new(end, end),
                vec![SourceEdit::insert(end, "\n")],
            )];
        }

        // Check for multiple trailing newlines (handle both \n and \r\n)
        let trimmed = source.trim_end_matches(&['\n', '\r'][..]);
        let trailing_newlines = source[trimmed.len()..].bytes().filter(|&b| b == b'\n').count();
        if trailing_newlines > 1 {
            let span_start = trimmed.len() as u32;
            return vec![LintViolation::with_fix(
                self.code(),
                format!("File ends with {} trailing newlines instead of 1.", trailing_newlines),
                rigsql_core::Span::new(span_start, end),
                vec![SourceEdit::replace(rigsql_core::Span::new(span_start, end), "\n")],
            )];
        }

        vec![]
    }
}
