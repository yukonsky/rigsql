use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

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
                if is_leading_comma(ctx) {
                    let fixes = build_leading_to_trailing_fix(ctx);
                    return vec![LintViolation::with_fix(
                        self.code(),
                        "Comma should be at the end of the line, not the start.",
                        span,
                        fixes,
                    )];
                }
            }
            CommaStyle::Leading => {
                if is_trailing_comma(ctx) {
                    let fixes = build_trailing_to_leading_fix(ctx);
                    return vec![LintViolation::with_fix(
                        self.code(),
                        "Comma should be at the start of the line, not the end.",
                        span,
                        fixes,
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

/// Build fix edits to convert leading comma to trailing comma.
///
/// Pattern: `col1\n    , col2` → `col1,\n    col2`
///
/// 1. Delete the comma and any whitespace immediately after it
/// 2. Insert comma after the last non-trivia element before the newline
fn build_leading_to_trailing_fix(ctx: &RuleContext) -> Vec<SourceEdit> {
    let comma_span = ctx.segment.span();

    // Find the end of the delete range (comma + whitespace after it)
    let mut delete_end = comma_span.end;
    let mut i = ctx.index_in_parent + 1;
    while i < ctx.siblings.len() {
        let seg = &ctx.siblings[i];
        if seg.segment_type() == SegmentType::Whitespace {
            delete_end = seg.span().end;
            i += 1;
        } else {
            break;
        }
    }

    // Also include any whitespace before the comma (between newline and comma)
    let mut delete_start = comma_span.start;
    if ctx.index_in_parent > 0 {
        let mut j = ctx.index_in_parent - 1;
        loop {
            let seg = &ctx.siblings[j];
            if seg.segment_type() == SegmentType::Whitespace {
                delete_start = seg.span().start;
                if j == 0 {
                    break;
                }
                j -= 1;
            } else {
                break;
            }
        }
    }

    // Find the last non-trivia element before the newline (to insert comma after it)
    let mut insert_pos = comma_span.start;
    if ctx.index_in_parent > 0 {
        let mut j = ctx.index_in_parent - 1;
        loop {
            let seg = &ctx.siblings[j];
            match seg.segment_type() {
                SegmentType::Whitespace | SegmentType::Newline => {
                    if j == 0 {
                        break;
                    }
                    j -= 1;
                }
                _ => {
                    insert_pos = seg.span().end;
                    break;
                }
            }
        }
    }

    // Reconstruct proper indentation after the comma removal
    // Keep the newline and original indentation, just without the comma
    let indent_size = (delete_end - comma_span.end) as usize;
    let original_indent_size = (comma_span.start - delete_start) as usize;
    let total_indent = original_indent_size + indent_size;
    let indent = " ".repeat(total_indent);

    vec![
        // Insert comma after the previous element
        SourceEdit::insert(insert_pos, ","),
        // Replace the whitespace + comma + whitespace with just whitespace
        SourceEdit::replace(
            rigsql_core::Span::new(delete_start, delete_end),
            indent,
        ),
    ]
}

/// Build fix edits to convert trailing comma to leading comma.
fn build_trailing_to_leading_fix(ctx: &RuleContext) -> Vec<SourceEdit> {
    let comma_span = ctx.segment.span();

    // Find the newline after the comma (skip whitespace)
    let mut newline_end = comma_span.end;
    let mut i = ctx.index_in_parent + 1;
    while i < ctx.siblings.len() {
        let seg = &ctx.siblings[i];
        match seg.segment_type() {
            SegmentType::Whitespace => {
                i += 1;
            }
            SegmentType::Newline => {
                newline_end = seg.span().end;
                break;
            }
            _ => break,
        }
    }

    // Find the position of the next element after the newline
    let insert_pos = if i + 1 < ctx.siblings.len() {
        ctx.siblings[i + 1].span().start
    } else {
        newline_end
    };

    vec![
        // Delete the trailing comma
        SourceEdit::delete(comma_span),
        // Insert comma before the next line's content
        SourceEdit::insert(insert_pos, ", "),
    ]
}
