use rigsql_core::{Segment, SegmentType, TokenKind};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// CP03: Function names must be consistently capitalised.
///
/// By default, expects lower case function names.
#[derive(Debug, Default)]
pub struct RuleCP03;

impl Rule for RuleCP03 {
    fn code(&self) -> &'static str {
        "CP03"
    }
    fn name(&self) -> &'static str {
        "capitalisation.functions"
    }
    fn description(&self) -> &'static str {
        "Function names must be consistently capitalised."
    }
    fn explanation(&self) -> &'static str {
        "Function names like COUNT, SUM, COALESCE should be consistently capitalised. \
         Whether upper or lower depends on your team's convention."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Capitalisation]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::FunctionCall])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        // FunctionCall's first child should be the function name (Identifier)
        let children = ctx.segment.children();
        if children.is_empty() {
            return vec![];
        }

        // Walk to find the function name token
        let name_seg = Self::find_function_name(children);
        let Some(Segment::Token(t)) = name_seg else {
            return vec![];
        };
        if t.token.kind != TokenKind::Word {
            return vec![];
        }

        // Check: function names should be consistent (default: lower)
        let text = t.token.text.as_str();
        // Skip if it's all upper or all lower (both are acceptable in many configs)
        // Default: we don't enforce function name case (many projects use either)
        // Only flag mixed case
        let is_all_upper = text
            .chars()
            .all(|c| !c.is_ascii_alphabetic() || c.is_ascii_uppercase());
        let is_all_lower = text
            .chars()
            .all(|c| !c.is_ascii_alphabetic() || c.is_ascii_lowercase());
        if is_all_upper || is_all_lower {
            return vec![];
        }

        vec![LintViolation::with_fix(
            self.code(),
            format!(
                "Function name '{}' has inconsistent capitalisation. Use all upper or all lower case.",
                text
            ),
            t.token.span,
            vec![SourceEdit::replace(t.token.span, text.to_ascii_uppercase())],
        )]
    }
}

impl RuleCP03 {
    fn find_function_name(children: &[Segment]) -> Option<&Segment> {
        for child in children {
            match child.segment_type() {
                SegmentType::Identifier => return Some(child),
                SegmentType::ColumnRef => {
                    // qualified function: schema.func — get last identifier
                    let inner = child.children();
                    return inner
                        .iter()
                        .rev()
                        .find(|s| s.segment_type() == SegmentType::Identifier);
                }
                _ if child.segment_type().is_trivia() => continue,
                _ => break,
            }
        }
        None
    }
}
