use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// LT14: Redundant/multiple semicolons at end of statement.
#[derive(Debug, Default)]
pub struct RuleLT14;

impl Rule for RuleLT14 {
    fn code(&self) -> &'static str {
        "LT14"
    }
    fn name(&self) -> &'static str {
        "layout.semicolons"
    }
    fn description(&self) -> &'static str {
        "Statements should not end with multiple semicolons."
    }
    fn explanation(&self) -> &'static str {
        "Each SQL statement should end with exactly one semicolon. Multiple consecutive \
         semicolons (;;) indicate a redundant terminator that should be removed."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::Semicolon])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        // Check if next non-trivia sibling is also a semicolon
        let mut i = ctx.index_in_parent + 1;
        while i < ctx.siblings.len() {
            let seg = &ctx.siblings[i];
            if seg.segment_type().is_trivia() {
                i += 1;
                continue;
            }
            if seg.segment_type() == SegmentType::Semicolon {
                return vec![LintViolation::with_fix(
                    self.code(),
                    "Found multiple consecutive semicolons.",
                    ctx.segment.span(),
                    vec![SourceEdit::delete(ctx.segment.span())],
                )];
            }
            break;
        }

        vec![]
    }
}
