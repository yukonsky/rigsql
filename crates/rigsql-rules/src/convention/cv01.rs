use rigsql_core::TokenKind;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::{LintViolation, SourceEdit};

/// CV01: Use consistent not-equal operator.
///
/// By default, flag inconsistent use within a file. When mixed styles are
/// present, the first occurrence wins. Users can pin a specific style via
/// the `preferred_not_equal` setting.
#[derive(Debug)]
pub struct RuleCV01 {
    pub preferred: NotEqualStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotEqualStyle {
    /// Match whichever style appears first in the file.
    Consistent,
    /// Prefer `!=`
    CStyle,
    /// Prefer `<>`
    AnsiStyle,
}

impl NotEqualStyle {
    fn as_str(self) -> Option<&'static str> {
        match self {
            NotEqualStyle::CStyle => Some("!="),
            NotEqualStyle::AnsiStyle => Some("<>"),
            NotEqualStyle::Consistent => None,
        }
    }
}

impl Default for RuleCV01 {
    fn default() -> Self {
        Self {
            preferred: NotEqualStyle::Consistent,
        }
    }
}

impl Rule for RuleCV01 {
    fn code(&self) -> &'static str {
        "CV01"
    }
    fn name(&self) -> &'static str {
        "convention.not_equal"
    }
    fn description(&self) -> &'static str {
        "Consistent not-equal operator."
    }
    fn explanation(&self) -> &'static str {
        "SQL has two not-equal operators: '!=' and '<>'. Using one consistently \
         improves readability. The ANSI standard uses '<>' but '!=' is more common \
         in modern SQL and programming."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(val) = settings.get("preferred_not_equal") {
            self.preferred = match val.as_str() {
                "ansi" | "<>" => NotEqualStyle::AnsiStyle,
                "c_style" | "cstyle" | "!=" => NotEqualStyle::CStyle,
                _ => NotEqualStyle::Consistent,
            };
        }
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::RootOnly
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let neq_tokens: Vec<_> = ctx
            .root
            .tokens()
            .into_iter()
            .filter(|t| t.kind == TokenKind::Neq)
            .collect();

        let target = match self.preferred.as_str() {
            Some(pinned) => pinned,
            None => match neq_tokens.first() {
                Some(first) if first.text.as_str() == "<>" => "<>",
                Some(_) => "!=",
                None => return vec![],
            },
        };

        let (msg, key) = if target == "!=" {
            ("Use '!=' instead of '<>'.", "rules.CV01.msg.use_ne")
        } else {
            ("Use '<>' instead of '!='.", "rules.CV01.msg.use_ltgt")
        };

        neq_tokens
            .into_iter()
            .filter(|t| t.text.as_str() != target)
            .map(|t| {
                LintViolation::with_fix_and_msg_key(
                    self.code(),
                    msg,
                    t.span,
                    vec![SourceEdit::replace(t.span, target)],
                    key,
                    vec![],
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_cv01_consistent_accepts_ansi_only() {
        let violations = lint_sql("SELECT * FROM t WHERE a <> b", RuleCV01::default());
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cv01_consistent_accepts_cstyle_only() {
        let violations = lint_sql("SELECT * FROM t WHERE a != b", RuleCV01::default());
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cv01_consistent_flags_mixed_first_ansi_wins() {
        let violations = lint_sql(
            "SELECT * FROM t WHERE a <> b AND c != d",
            RuleCV01::default(),
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "<>");
    }

    #[test]
    fn test_cv01_consistent_flags_mixed_first_cstyle_wins() {
        let violations = lint_sql(
            "SELECT * FROM t WHERE a != b AND c <> d",
            RuleCV01::default(),
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "!=");
    }

    #[test]
    fn test_cv01_cstyle_policy_flags_ansi() {
        let rule = RuleCV01 {
            preferred: NotEqualStyle::CStyle,
        };
        let violations = lint_sql("SELECT * FROM t WHERE a <> b", rule);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_cv01_ansi_policy_flags_cstyle() {
        let rule = RuleCV01 {
            preferred: NotEqualStyle::AnsiStyle,
        };
        let violations = lint_sql("SELECT * FROM t WHERE a != b", rule);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_cv01_consistent_flags_multiple_mismatches() {
        let violations = lint_sql(
            "SELECT * FROM t WHERE a <> b AND c != d AND e != f",
            RuleCV01::default(),
        );
        assert_eq!(violations.len(), 2);
        assert!(violations.iter().all(|v| v.fixes[0].new_text == "<>"));
    }
}
