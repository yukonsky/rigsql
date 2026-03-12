use rigsql_core::{Segment, SegmentType, TokenKind};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

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

impl RuleLT02 {
    /// Round `value` up to the nearest multiple of `indent_size`.
    fn round_to_indent(&self, value: usize) -> usize {
        if value == 0 {
            self.indent_size
        } else {
            ((value + self.indent_size - 1) / self.indent_size) * self.indent_size
        }
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

        // Flag tabs mixed with spaces — convert tabs to spaces
        if text.contains('\t') && text.contains(' ') {
            let visual_width: usize = text
                .chars()
                .map(|c| if c == '\t' { self.indent_size } else { 1 })
                .sum();
            let rounded = self.round_to_indent(visual_width);
            let fixed = " ".repeat(rounded);
            return vec![LintViolation::with_fix_and_msg_key(
                self.code(),
                "Mixed tabs and spaces in indentation.",
                t.token.span,
                vec![SourceEdit::replace(t.token.span, fixed)],
                "rules.LT02.msg.mixed",
                vec![],
            )];
        }

        // Flag non-multiple of indent_size (space-only indentation)
        // Round up to the nearest multiple
        if !text.contains('\t') && text.len() % self.indent_size != 0 {
            let rounded = self.round_to_indent(text.len());
            let fixed = " ".repeat(rounded);
            return vec![LintViolation::with_fix_and_msg_key(
                self.code(),
                format!(
                    "Indentation is not a multiple of {} spaces (found {} spaces).",
                    self.indent_size,
                    text.len()
                ),
                t.token.span,
                vec![SourceEdit::replace(t.token.span, fixed)],
                "rules.LT02.msg.not_multiple",
                vec![
                    ("size".to_string(), self.indent_size.to_string()),
                    ("found".to_string(), text.len().to_string()),
                ],
            )];
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_lt02_flags_odd_indent() {
        let violations = lint_sql("SELECT *\n   FROM t", RuleLT02::default());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule_code, "LT02");
        // 3 spaces → rounded up to 4 spaces
        assert_eq!(violations[0].fixes.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "    ");
    }

    #[test]
    fn test_lt02_accepts_4space_indent() {
        let violations = lint_sql("SELECT *\n    FROM t", RuleLT02::default());
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_lt02_flags_mixed_tabs_spaces() {
        let violations = lint_sql("SELECT *\n\t FROM t", RuleLT02::default());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule_code, "LT02");
        // Tab(4) + space(1) = 5 visual width → rounded up to 8 spaces
        assert_eq!(violations[0].fixes.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "        ");
    }

    #[test]
    fn test_lt02_fix_5_spaces_rounds_to_8() {
        let violations = lint_sql("SELECT *\n     FROM t", RuleLT02::default());
        assert_eq!(violations.len(), 1);
        // 5 spaces → rounded up to 8
        assert_eq!(violations[0].fixes[0].new_text, "        ");
    }
}
