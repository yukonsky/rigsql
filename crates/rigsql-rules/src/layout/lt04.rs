use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// LT04: Leading/trailing commas.
///
/// By default, expects trailing commas (comma at end of line).
#[derive(Debug)]
pub struct RuleLT04 {
    pub style: CommaStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommaStyle {
    Trailing,
    Leading,
}

impl Default for RuleLT04 {
    fn default() -> Self {
        Self {
            style: CommaStyle::Trailing,
        }
    }
}

impl Rule for RuleLT04 {
    fn code(&self) -> &'static str {
        "LT04"
    }
    fn name(&self) -> &'static str {
        "layout.commas"
    }
    fn description(&self) -> &'static str {
        "Commas should be at the end of the line, not the start."
    }
    fn explanation(&self) -> &'static str {
        "Commas in SELECT lists, GROUP BY, and other clauses should consistently appear \
         at the end of the line (trailing) or the start of the next line (leading). \
         Mixing styles reduces readability."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(val) = settings.get("comma_style") {
            self.style = match val.as_str() {
                "leading" => CommaStyle::Leading,
                _ => CommaStyle::Trailing,
            };
        }
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::Comma])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let span = ctx.segment.span();

        match self.style {
            CommaStyle::Trailing => {
                // Comma should NOT be preceded by a newline (that means it's leading)
                // Check: is there a newline immediately before the comma (possibly with whitespace)?
                if is_leading_comma(ctx) {
                    return vec![LintViolation::new(
                        self.code(),
                        "Comma should be at the end of the line, not the start.",
                        span,
                    )];
                }
            }
            CommaStyle::Leading => {
                // Comma should be preceded by a newline (leading style)
                if is_trailing_comma(ctx) {
                    return vec![LintViolation::new(
                        self.code(),
                        "Comma should be at the start of the line, not the end.",
                        span,
                    )];
                }
            }
        }

        vec![]
    }
}

/// Check if comma is in leading position (newline then optional whitespace then comma).
fn is_leading_comma(ctx: &RuleContext) -> bool {
    if ctx.index_in_parent == 0 {
        return false;
    }
    // Walk backwards past whitespace to see if there's a newline
    let mut i = ctx.index_in_parent - 1;
    loop {
        let seg = &ctx.siblings[i];
        match seg.segment_type() {
            SegmentType::Whitespace => {
                if i == 0 {
                    return false;
                }
                i -= 1;
            }
            SegmentType::Newline => return true,
            _ => return false,
        }
    }
}

/// Check if comma is in trailing position (comma then optional whitespace then newline).
fn is_trailing_comma(ctx: &RuleContext) -> bool {
    let mut i = ctx.index_in_parent + 1;
    while i < ctx.siblings.len() {
        let seg = &ctx.siblings[i];
        match seg.segment_type() {
            SegmentType::Whitespace => {
                i += 1;
            }
            SegmentType::Newline => return true,
            _ => return false,
        }
    }
    false
}
