use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// ST12: Consecutive semicolons indicate empty statements.
///
/// Scans the entire file for semicolons separated only by whitespace/newlines.
#[derive(Debug, Default)]
pub struct RuleST12;

impl Rule for RuleST12 {
    fn code(&self) -> &'static str {
        "ST12"
    }
    fn name(&self) -> &'static str {
        "structure.consecutive_semicolons"
    }
    fn description(&self) -> &'static str {
        "Consecutive semicolons indicate empty statements."
    }
    fn explanation(&self) -> &'static str {
        "Multiple consecutive semicolons with only whitespace between them indicate \
         empty statements, which are likely unintentional. Remove the extra semicolons."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Structure]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::RootOnly
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut violations = Vec::new();
        let mut last_semicolon_span: Option<rigsql_core::Span> = None;
        let mut only_trivia_since_last = true;

        ctx.segment.walk(&mut |seg| {
            let st = seg.segment_type();
            if st == SegmentType::Semicolon {
                if only_trivia_since_last && last_semicolon_span.is_some() {
                    violations.push(LintViolation::with_msg_key(
                        self.code(),
                        "Consecutive semicolons found (empty statement).",
                        seg.span(),
                        "rules.ST12.msg",
                        vec![],
                    ));
                }
                last_semicolon_span = Some(seg.span());
                only_trivia_since_last = true;
            } else if !st.is_trivia()
                && st != SegmentType::File
                && st != SegmentType::Statement
                && st != SegmentType::Unparsable
            {
                only_trivia_since_last = false;
            }
        });

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_st12_flags_consecutive_semicolons() {
        let violations = lint_sql("SELECT 1;;", RuleST12);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("Consecutive"));
    }

    #[test]
    fn test_st12_accepts_single_semicolon() {
        let violations = lint_sql("SELECT 1;", RuleST12);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_st12_accepts_separate_statements() {
        let violations = lint_sql("SELECT 1; SELECT 2;", RuleST12);
        assert_eq!(violations.len(), 0);
    }
}
