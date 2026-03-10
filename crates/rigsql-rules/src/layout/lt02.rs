use rigsql_core::{Segment, SegmentType, TokenKind};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// LT02: Incorrect indentation.
///
/// Expects consistent indentation (default 4 spaces per level).
#[derive(Debug)]
pub struct RuleLT02 {
    pub indent_size: usize,
}

impl Default for RuleLT02 {
    fn default() -> Self {
        Self { indent_size: 4 }
    }
}

impl Rule for RuleLT02 {
    fn code(&self) -> &'static str {
        "LT02"
    }
    fn name(&self) -> &'static str {
        "layout.indent"
    }
    fn description(&self) -> &'static str {
        "Incorrect indentation."
    }
    fn explanation(&self) -> &'static str {
        "SQL should use consistent indentation. Each indentation level should use \
         the same number of spaces (default 4). Tabs should not be mixed with spaces."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(val) = settings.get("indent_unit") {
            if val == "tab" {
                self.indent_size = 1; // tab mode
            }
        }
        if let Some(val) = settings.get("tab_space_size") {
            if let Ok(n) = val.parse() {
                self.indent_size = n;
            }
        }
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::Whitespace])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let Segment::Token(t) = ctx.segment else {
            return vec![];
        };
        if t.token.kind != TokenKind::Whitespace {
            return vec![];
        }

        let text = t.token.text.as_str();

        // Only check indentation (whitespace after a newline)
        if ctx.index_in_parent == 0 {
            return vec![];
        }
        let prev = &ctx.siblings[ctx.index_in_parent - 1];
        if prev.segment_type() != SegmentType::Newline {
            return vec![];
        }

        // Flag tabs mixed with spaces
        if text.contains('\t') && text.contains(' ') {
            return vec![LintViolation::new(
                self.code(),
                "Mixed tabs and spaces in indentation.",
                t.token.span,
            )];
        }

        // Flag non-multiple of indent_size (space-only indentation)
        if !text.contains('\t') && text.len() % self.indent_size != 0 {
            return vec![LintViolation::new(
                self.code(),
                format!(
                    "Indentation is not a multiple of {} spaces (found {} spaces).",
                    self.indent_size,
                    text.len()
                ),
                t.token.span,
            )];
        }

        vec![]
    }
}
