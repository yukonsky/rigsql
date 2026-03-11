use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// LT13: Files must not begin with newlines or whitespace.
#[derive(Debug, Default)]
pub struct RuleLT13;

impl Rule for RuleLT13 {
    fn code(&self) -> &'static str {
        "LT13"
    }
    fn name(&self) -> &'static str {
        "layout.start_of_file"
    }
    fn description(&self) -> &'static str {
        "Files must not begin with newlines or whitespace."
    }
    fn explanation(&self) -> &'static str {
        "Files should start with actual content, not blank lines or whitespace. \
         Leading whitespace is likely unintentional and should be removed."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::RootOnly
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let source = ctx.source;
        if source.is_empty() {
            return vec![];
        }

        let first_non_ws = source.find(|c: char| !c.is_whitespace());

        match first_non_ws {
            Some(0) => vec![], // starts with content
            Some(pos) => {
                vec![LintViolation::with_fix(
                    self.code(),
                    "File starts with whitespace or blank lines.",
                    rigsql_core::Span::new(0, pos as u32),
                    vec![SourceEdit::delete(rigsql_core::Span::new(0, pos as u32))],
                )]
            }
            None => vec![], // all whitespace file — other rules handle this
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_lt13_flags_leading_whitespace() {
        let violations = lint_sql("  SELECT 1", RuleLT13);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_lt13_accepts_no_leading_whitespace() {
        let violations = lint_sql("SELECT 1", RuleLT13);
        assert_eq!(violations.len(), 0);
    }
}
