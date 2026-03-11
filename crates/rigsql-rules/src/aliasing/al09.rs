use rigsql_core::{Segment, SegmentType, Span};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// AL09: Self-aliasing of columns.
///
/// Aliasing a column to itself (e.g., `col AS col`) is redundant and
/// should be removed to improve readability.
#[derive(Debug, Default)]
pub struct RuleAL09;

impl Rule for RuleAL09 {
    fn code(&self) -> &'static str {
        "AL09"
    }
    fn name(&self) -> &'static str {
        "aliasing.self_alias"
    }
    fn description(&self) -> &'static str {
        "Self-aliasing of columns is redundant."
    }
    fn explanation(&self) -> &'static str {
        "Writing `col AS col` or `table.col AS col` aliases a column to its own name. \
         This is redundant and adds unnecessary noise. Remove the AS clause to simplify \
         the query."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Aliasing]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::AliasExpression])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        // Only check column aliases (within SelectClause)
        let in_select = ctx
            .parent
            .is_some_and(|p| p.segment_type() == SegmentType::SelectClause);
        if !in_select {
            return vec![];
        }

        let children = ctx.segment.children();

        // Single-pass extraction: source name, alias name, and AS-to-end span
        let Some(info) = extract_self_alias_info(children) else {
            return vec![];
        };

        if !info.alias_name.eq_ignore_ascii_case(&info.source_name) {
            return vec![];
        }

        vec![LintViolation::with_fix(
            self.code(),
            format!("Column '{}' is aliased to itself.", info.source_name),
            ctx.segment.span(),
            vec![SourceEdit::delete(info.remove_span)],
        )]
    }
}

struct SelfAliasInfo {
    source_name: String,
    alias_name: String,
    remove_span: Span,
}

/// Single-pass extraction of source name, alias name, and the span to remove.
fn extract_self_alias_info(children: &[Segment]) -> Option<SelfAliasInfo> {
    let mut source_name: Option<String> = None;
    let mut alias_name: Option<String> = None;
    let mut as_region_start: Option<u32> = None;
    let mut found_as = false;
    let mut prev_trivia_start: Option<u32> = None;

    for child in children {
        let st = child.segment_type();

        if !found_as {
            // Before AS: track source column name
            if st == SegmentType::Keyword {
                if let Segment::Token(t) = child {
                    if t.token.text.as_str().eq_ignore_ascii_case("AS") {
                        found_as = true;
                        // Include preceding whitespace in removal span
                        as_region_start = Some(prev_trivia_start.unwrap_or(child.span().start));
                        continue;
                    }
                }
            }
            if st.is_trivia() {
                if prev_trivia_start.is_none() || source_name.is_some() {
                    prev_trivia_start = Some(child.span().start);
                }
            } else {
                prev_trivia_start = None;
                // Extract source identifier
                if st == SegmentType::ColumnRef || st == SegmentType::QualifiedIdentifier {
                    source_name = find_last_identifier_in(child);
                } else if st == SegmentType::Identifier || st == SegmentType::QuotedIdentifier {
                    if let Segment::Token(t) = child {
                        source_name = Some(t.token.text.to_string());
                    }
                }
            }
        } else {
            // After AS: find alias identifier
            if (st == SegmentType::Identifier || st == SegmentType::QuotedIdentifier)
                && alias_name.is_none()
            {
                if let Segment::Token(t) = child {
                    alias_name = Some(t.token.text.to_string());
                }
            }
        }
    }

    let end = children.last()?.span().end;
    Some(SelfAliasInfo {
        source_name: source_name?,
        alias_name: alias_name?,
        remove_span: Span::new(as_region_start?, end),
    })
}

/// Find the last identifier token within a node (e.g., `table.col` → `col`).
fn find_last_identifier_in(segment: &Segment) -> Option<String> {
    let mut result = None;
    for child in segment.children() {
        let st = child.segment_type();
        if st == SegmentType::Identifier || st == SegmentType::QuotedIdentifier {
            if let Segment::Token(t) = child {
                result = Some(t.token.text.to_string());
            }
        }
    }
    result
}
