use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// CV06: Statements must end with a semicolon.
#[derive(Debug, Default)]
pub struct RuleCV06;

impl Rule for RuleCV06 {
    fn code(&self) -> &'static str { "CV06" }
    fn name(&self) -> &'static str { "convention.terminator" }
    fn description(&self) -> &'static str { "Statements must end with a semicolon." }
    fn explanation(&self) -> &'static str {
        "All SQL statements should be terminated with a semicolon. While some databases \
         accept statements without terminators, including them is good practice for \
         portability and clarity."
    }
    fn groups(&self) -> &[RuleGroup] { &[RuleGroup::Convention] }
    fn is_fixable(&self) -> bool { true }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::Statement])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();
        if children.is_empty() {
            return vec![];
        }

        // Check if the last non-trivia child is a semicolon
        let has_semicolon = children
            .iter()
            .rev()
            .find(|s| !s.segment_type().is_trivia())
            .is_some_and(|s| s.segment_type() == SegmentType::Semicolon);

        if !has_semicolon {
            let span = ctx.segment.span();
            return vec![LintViolation::new(
                self.code(),
                "Statement is not terminated with a semicolon.",
                rigsql_core::Span::new(span.end, span.end),
            )];
        }

        vec![]
    }
}
