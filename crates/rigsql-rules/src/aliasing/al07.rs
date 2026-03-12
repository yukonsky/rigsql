use rigsql_core::{Segment, SegmentType, Span};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::is_in_table_context;
use crate::violation::{LintViolation, SourceEdit};

/// AL07: Avoid table aliases in FROM clauses and JOIN conditions.
///
/// Disabled by default. When enabled, flags all table aliases and suggests
/// using the full table name instead.
#[derive(Debug, Default)]
pub struct RuleAL07 {
    pub force_enable: bool,
}

impl Rule for RuleAL07 {
    fn code(&self) -> &'static str {
        "AL07"
    }
    fn name(&self) -> &'static str {
        "aliasing.forbid"
    }
    fn description(&self) -> &'static str {
        "Avoid table aliases in FROM clauses and JOIN conditions."
    }
    fn explanation(&self) -> &'static str {
        "Table aliases can reduce readability, especially initialisms. Using the \
         full table name makes it clear where each column comes from. This rule \
         is disabled by default as it is controversial for larger databases."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Aliasing]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(val) = settings.get("force_enable") {
            self.force_enable = val.eq_ignore_ascii_case("true") || val == "1";
        }
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::AliasExpression])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        if !self.force_enable {
            return vec![];
        }

        if !is_in_table_context(ctx) {
            return vec![];
        }

        // Find the span of the alias part (AS keyword + alias name) to delete
        let children = ctx.segment.children();
        let mut alias_start: Option<Span> = None;
        let mut alias_end: Option<Span> = None;
        let mut found_table = false;

        for child in children {
            let st = child.segment_type();
            if !found_table {
                if st == SegmentType::Identifier
                    || st == SegmentType::QuotedIdentifier
                    || st == SegmentType::Keyword
                {
                    found_table = true;
                }
                continue;
            }

            // Everything after the table name is the alias part
            if alias_start.is_none() {
                // Include preceding whitespace
                if let Segment::Token(_) = child {
                    alias_start = Some(child.span());
                }
            }
            alias_end = Some(child.span());
        }

        if let (Some(start), Some(end)) = (alias_start, alias_end) {
            let delete_span = start.merge(end);
            vec![LintViolation::with_fix_and_msg_key(
                self.code(),
                "Avoid using table aliases. Use the full table name instead.",
                ctx.segment.span(),
                vec![SourceEdit::delete(delete_span)],
                "rules.AL07.msg",
                vec![],
            )]
        } else {
            vec![LintViolation::with_msg_key(
                self.code(),
                "Avoid using table aliases. Use the full table name instead.",
                ctx.segment.span(),
                "rules.AL07.msg",
                vec![],
            )]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_al07_disabled_by_default() {
        let violations = lint_sql("SELECT * FROM users AS u", RuleAL07::default());
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_al07_enabled_flags_table_alias() {
        let rule = RuleAL07 { force_enable: true };
        let violations = lint_sql("SELECT * FROM users AS u", rule);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_al07_skips_column_alias() {
        let rule = RuleAL07 { force_enable: true };
        let violations = lint_sql("SELECT col AS c FROM t", rule);
        // col AS c is in SelectClause, not FROM/JOIN, so no violation
        assert_eq!(violations.len(), 0);
    }
}
