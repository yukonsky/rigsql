use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// LT05: Line too long.
///
/// Default max line length: 80 characters.
#[derive(Debug)]
pub struct RuleLT05 {
    pub max_line_length: usize,
}

impl Default for RuleLT05 {
    fn default() -> Self {
        Self {
            max_line_length: 80,
        }
    }
}

impl Rule for RuleLT05 {
    fn code(&self) -> &'static str {
        "LT05"
    }
    fn name(&self) -> &'static str {
        "layout.long_lines"
    }
    fn description(&self) -> &'static str {
        "Line too long."
    }
    fn explanation(&self) -> &'static str {
        "Long lines are harder to read and review. Keep lines under the configured \
         maximum length (default 80 characters). Break long queries across multiple lines."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(val) = settings.get("max_line_length") {
            if let Ok(n) = val.parse() {
                self.max_line_length = n;
            }
        }
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::RootOnly
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut violations = Vec::new();
        let source = ctx.source;
        let mut offset = 0usize;

        for line in source.lines() {
            let line_len = line.len();
            if line_len > self.max_line_length {
                let span = rigsql_core::Span::new(offset as u32, (offset + line_len) as u32);
                violations.push(LintViolation::with_msg_key(
                    self.code(),
                    format!(
                        "Line is too long ({} > {} characters).",
                        line_len, self.max_line_length
                    ),
                    span,
                    "rules.LT05.msg",
                    vec![
                        ("length".to_string(), line_len.to_string()),
                        ("max".to_string(), self.max_line_length.to_string()),
                    ],
                ));
            }
            offset += line_len + 1; // +1 for \n
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_lt05_flags_long_line() {
        let long_sql = format!("SELECT {} FROM t", "a, ".repeat(30));
        let violations = lint_sql(&long_sql, RuleLT05::default());
        assert!(!violations.is_empty());
        assert!(violations.iter().all(|v| v.rule_code == "LT05"));
    }

    #[test]
    fn test_lt05_accepts_short_line() {
        let violations = lint_sql("SELECT * FROM t", RuleLT05::default());
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_lt05_custom_max_length() {
        let rule = RuleLT05 {
            max_line_length: 120,
        };
        let sql_100_chars = format!("SELECT {} FROM t", "x".repeat(88));
        let violations = lint_sql(&sql_100_chars, rule);
        assert_eq!(violations.len(), 0);
    }
}
