use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::has_trailing_newline;
use crate::violation::{LintViolation, SourceEdit};

/// LT14: Keyword clauses should follow a standard for being before/after newlines.
///
/// Major clause keywords (FROM, WHERE, GROUP BY, etc.) should start on a new line.
#[derive(Debug, Default)]
pub struct RuleLT14;

const CLAUSE_TYPES: &[SegmentType] = &[
    SegmentType::FromClause,
    SegmentType::WhereClause,
    SegmentType::GroupByClause,
    SegmentType::HavingClause,
    SegmentType::OrderByClause,
    SegmentType::LimitClause,
];

impl Rule for RuleLT14 {
    fn code(&self) -> &'static str {
        "LT14"
    }
    fn name(&self) -> &'static str {
        "layout.keyword_newline"
    }
    fn description(&self) -> &'static str {
        "Keyword clauses should follow a standard for being before/after newlines."
    }
    fn explanation(&self) -> &'static str {
        "Major SQL clauses (FROM, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT) should \
         start on a new line for readability. Placing them on the same line as the \
         previous clause makes the query harder to scan."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectStatement])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();
        let mut violations = Vec::new();

        for (i, child) in children.iter().enumerate() {
            if !CLAUSE_TYPES.contains(&child.segment_type()) {
                continue;
            }

            // Check if there's a Newline in siblings before this clause.
            // Newlines may also be absorbed as the last child of the preceding clause,
            // so we check both siblings and the trailing children of the previous segment.
            let mut found_newline = false;
            for prev in children[..i].iter().rev() {
                let st = prev.segment_type();
                if st == SegmentType::Newline {
                    found_newline = true;
                    break;
                }
                if st == SegmentType::Whitespace {
                    continue;
                }
                // Non-trivia sibling: also check if it ends with a Newline
                found_newline = has_trailing_newline(prev);
                break;
            }

            if !found_newline && i > 0 {
                let clause_name = match child.segment_type() {
                    SegmentType::FromClause => "FROM",
                    SegmentType::WhereClause => "WHERE",
                    SegmentType::GroupByClause => "GROUP BY",
                    SegmentType::HavingClause => "HAVING",
                    SegmentType::OrderByClause => "ORDER BY",
                    SegmentType::LimitClause => "LIMIT",
                    _ => "Clause",
                };
                // Determine indentation from the statement's line start
                let indent = get_line_indent(ctx.source, ctx.segment.span().start);
                let newline_with_indent = format!("\n{}", indent);

                // Replace preceding whitespace with newline+indent, or insert
                let fix = if i > 0 {
                    let prev = &children[i - 1];
                    if prev.segment_type() == SegmentType::Whitespace {
                        vec![SourceEdit::replace(prev.span(), newline_with_indent)]
                    } else {
                        vec![SourceEdit::insert(child.span().start, &newline_with_indent)]
                    }
                } else {
                    vec![SourceEdit::insert(child.span().start, &newline_with_indent)]
                };
                violations.push(LintViolation::with_fix(
                    self.code(),
                    format!("{} clause should start on a new line.", clause_name),
                    child.span(),
                    fix,
                ));
            }
        }

        violations
    }
}

/// Extract the leading whitespace of the line containing the given byte offset.
fn get_line_indent(source: &str, offset: u32) -> &str {
    let bytes = source.as_bytes();
    let pos = offset as usize;
    // Find start of line
    let mut line_start = pos;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    // Find end of leading whitespace
    let mut indent_end = line_start;
    while indent_end < bytes.len() && (bytes[indent_end] == b' ' || bytes[indent_end] == b'\t') {
        indent_end += 1;
    }
    &source[line_start..indent_end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_lt14_accepts_newlines_before_clauses() {
        let violations = lint_sql("SELECT a\nFROM t\nWHERE x = 1", RuleLT14);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_lt14_flags_inline_clauses() {
        let violations = lint_sql("SELECT a FROM t WHERE x = 1", RuleLT14);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_lt14_accepts_single_clause() {
        let violations = lint_sql("SELECT 1", RuleLT14);
        assert_eq!(violations.len(), 0);
    }
}
