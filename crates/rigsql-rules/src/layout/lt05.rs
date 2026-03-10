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
                violations.push(LintViolation::new(
                    self.code(),
                    format!(
                        "Line is too long ({} > {} characters).",
                        line_len, self.max_line_length
                    ),
                    span,
                ));
            }
            offset += line_len + 1; // +1 for \n
        }

        violations
    }
}
