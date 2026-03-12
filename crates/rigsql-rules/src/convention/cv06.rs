use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// CV06: Statements must end with a semicolon.
#[derive(Debug, Default)]
pub struct RuleCV06;

impl Rule for RuleCV06 {
    fn code(&self) -> &'static str {
        "CV06"
    }
    fn name(&self) -> &'static str {
        "convention.terminator"
    }
    fn description(&self) -> &'static str {
        "Statements must end with a semicolon."
    }
    fn explanation(&self) -> &'static str {
        "All SQL statements should be terminated with a semicolon. While some databases \
         accept statements without terminators, including them is good practice for \
         portability and clarity."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::Statement])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        // In TSQL, semicolons are optional in most contexts
        if ctx.dialect == "tsql" {
            return vec![];
        }

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
            // Find the end of the last non-trivia child for insertion point
            let insert_pos = children
                .iter()
                .rev()
                .find(|s| !s.segment_type().is_trivia())
                .map(|s| s.span().end)
                .unwrap_or(span.end);
            return vec![LintViolation::with_fix_and_msg_key(
                self.code(),
                "Statement is not terminated with a semicolon.",
                rigsql_core::Span::new(span.end, span.end),
                vec![SourceEdit::insert(insert_pos, ";")],
                "rules.CV06.msg",
                vec![],
            )];
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{lint_sql, lint_sql_with_dialect};

    #[test]
    fn test_cv06_flags_missing_semicolon() {
        let violations = lint_sql("SELECT 1", RuleCV06);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, ";");
    }

    #[test]
    fn test_cv06_accepts_semicolon() {
        let violations = lint_sql("SELECT 1;", RuleCV06);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cv06_skips_tsql() {
        let violations = lint_sql_with_dialect("SELECT 1", RuleCV06, "tsql");
        assert_eq!(violations.len(), 0);
    }
}
