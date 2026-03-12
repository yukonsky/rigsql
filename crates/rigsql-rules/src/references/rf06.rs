use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// RF06: Unnecessary quoting of identifiers.
///
/// If a quoted identifier (e.g., `"my_col"`) contains only alphanumeric
/// characters and underscores, and starts with a letter or underscore,
/// it could be written as a bare identifier. The quotes are unnecessary.
#[derive(Debug, Default)]
pub struct RuleRF06;

impl Rule for RuleRF06 {
    fn code(&self) -> &'static str {
        "RF06"
    }
    fn name(&self) -> &'static str {
        "references.quoting"
    }
    fn description(&self) -> &'static str {
        "Unnecessary quoting of identifiers."
    }
    fn explanation(&self) -> &'static str {
        "Quoted identifiers that contain only alphanumeric characters, underscores, \
         and start with a letter or underscore do not need to be quoted. Removing \
         unnecessary quotes improves readability. Quoting should be reserved for \
         identifiers that genuinely require it (e.g., reserved words, spaces, special characters)."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::References]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::QuotedIdentifier])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let Segment::Token(t) = ctx.segment else {
            return vec![];
        };

        let text = &t.token.text;

        // Strip surrounding quotes (double quotes, backticks, or brackets)
        let inner = strip_quotes(text);
        let Some(inner) = inner else {
            return vec![];
        };

        if inner.is_empty() {
            return vec![];
        }

        // Check if the inner text could be a bare identifier:
        // starts with letter or underscore, and only contains alphanumeric + underscore
        let first = inner.chars().next().unwrap();
        if !(first.is_ascii_alphabetic() || first == '_') {
            return vec![];
        }

        let is_simple = inner.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');

        if is_simple {
            vec![LintViolation::with_fix_and_msg_key(
                self.code(),
                format!("Identifier '{}' does not need quoting.", text),
                t.token.span,
                vec![SourceEdit::replace(t.token.span, inner.to_string())],
                "rules.RF06.msg",
                vec![("name".to_string(), text.to_string())],
            )]
        } else {
            vec![]
        }
    }
}

fn strip_quotes(text: &str) -> Option<&str> {
    if text.len() < 2 {
        return None;
    }
    let bytes = text.as_bytes();
    match (bytes[0], bytes[bytes.len() - 1]) {
        (b'"', b'"') | (b'`', b'`') => Some(&text[1..text.len() - 1]),
        (b'[', b']') => Some(&text[1..text.len() - 1]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_rf06_flags_unnecessary_quoting() {
        let violations = lint_sql("SELECT \"my_col\" FROM t", RuleRF06);
        assert!(
            !violations.is_empty(),
            "Should flag unnecessarily quoted identifier"
        );
        assert!(violations[0].message.contains("my_col"));
        assert!(!violations[0].fixes.is_empty(), "Should provide a fix");
    }

    #[test]
    fn test_rf06_accepts_necessary_quoting() {
        let violations = lint_sql("SELECT \"my-col\" FROM t", RuleRF06);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_rf06_accepts_bare_identifiers() {
        let violations = lint_sql("SELECT my_col FROM t", RuleRF06);
        assert_eq!(violations.len(), 0);
    }
}
