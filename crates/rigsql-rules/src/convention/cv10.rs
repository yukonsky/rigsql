use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// CV10: Consistent usage of preferred quotes for quoted literals.
///
/// By default, prefer single quotes for string literals.
#[derive(Debug)]
pub struct RuleCV10 {
    pub preferred_style: QuoteStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteStyle {
    Single,
    Double,
}

impl Default for RuleCV10 {
    fn default() -> Self {
        Self {
            preferred_style: QuoteStyle::Single,
        }
    }
}

impl Rule for RuleCV10 {
    fn code(&self) -> &'static str {
        "CV10"
    }
    fn name(&self) -> &'static str {
        "convention.quoted_literals"
    }
    fn description(&self) -> &'static str {
        "Consistent usage of preferred quotes for quoted literals."
    }
    fn explanation(&self) -> &'static str {
        "String literals should use a consistent quoting style. By default, \
         single quotes are preferred as they are the ANSI SQL standard for \
         string literals."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(val) = settings.get("preferred_quoted_literal_style") {
            self.preferred_style = match val.as_str() {
                "double" => QuoteStyle::Double,
                _ => QuoteStyle::Single,
            };
        }
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::StringLiteral])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let Segment::Token(t) = ctx.segment else {
            return vec![];
        };

        let text = t.token.text.as_str();
        if text.len() < 2 {
            return vec![];
        }

        let first_char = text.as_bytes()[0];
        let uses_single = first_char == b'\'';
        let uses_double = first_char == b'"';

        match self.preferred_style {
            QuoteStyle::Single if uses_double => {
                let inner = &text[1..text.len() - 1];
                let replaced = inner.replace('\'', "''").replace("\"\"", "\"");
                let new_text = format!("'{}'", replaced);
                vec![LintViolation::with_fix(
                    self.code(),
                    "Use single quotes for string literals.",
                    t.token.span,
                    vec![SourceEdit::replace(t.token.span, new_text)],
                )]
            }
            QuoteStyle::Double if uses_single => {
                let inner = &text[1..text.len() - 1];
                let replaced = inner.replace('"', "\"\"").replace("''", "'");
                let new_text = format!("\"{}\"", replaced);
                vec![LintViolation::with_fix(
                    self.code(),
                    "Use double quotes for string literals.",
                    t.token.span,
                    vec![SourceEdit::replace(t.token.span, new_text)],
                )]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_cv10_accepts_single_quotes() {
        let violations = lint_sql("SELECT 'hello' FROM t", RuleCV10::default());
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cv10_accepts_non_string() {
        let violations = lint_sql("SELECT 1 FROM t", RuleCV10::default());
        assert_eq!(violations.len(), 0);
    }
}
