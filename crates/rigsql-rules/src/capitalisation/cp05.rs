use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::check_capitalisation;
use crate::violation::LintViolation;

/// CP05: Data type names must be consistently capitalised.
///
/// By default expects upper case (INT, VARCHAR, etc.).
#[derive(Debug)]
pub struct RuleCP05 {
    pub policy: DataTypeCapPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTypeCapPolicy {
    Upper,
    Lower,
}

impl Default for RuleCP05 {
    fn default() -> Self {
        Self {
            policy: DataTypeCapPolicy::Upper,
        }
    }
}

impl Rule for RuleCP05 {
    fn code(&self) -> &'static str {
        "CP05"
    }
    fn name(&self) -> &'static str {
        "capitalisation.types"
    }
    fn description(&self) -> &'static str {
        "Data type names must be consistently capitalised."
    }
    fn explanation(&self) -> &'static str {
        "Data type names (INT, VARCHAR, TEXT, etc.) should use consistent capitalisation. \
         Most style guides recommend upper case for data types to distinguish them from column names."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Capitalisation]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(val) = settings.get("capitalisation_policy") {
            self.policy = match val.as_str() {
                "lower" => DataTypeCapPolicy::Lower,
                _ => DataTypeCapPolicy::Upper,
            };
        }
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::DataType])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        // DataType node may contain keyword tokens (INT, VARCHAR, etc.)
        let tokens = ctx.segment.tokens();
        let mut violations = Vec::new();

        for token in tokens {
            let text = token.text.as_str();
            // Skip numeric parts like VARCHAR(255)
            if text
                .chars()
                .all(|c| c.is_ascii_digit() || c == '(' || c == ')' || c == ',')
            {
                continue;
            }

            let (expected, policy_name) = match self.policy {
                DataTypeCapPolicy::Upper => (text.to_ascii_uppercase(), "upper"),
                DataTypeCapPolicy::Lower => (text.to_ascii_lowercase(), "lower"),
            };

            if let Some(v) = check_capitalisation(
                self.code(),
                "Data type",
                text,
                &expected,
                policy_name,
                token.span,
            ) {
                violations.push(v);
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_cp05_flags_lowercase_type() {
        let violations = lint_sql("SELECT CAST(1 AS int)", RuleCP05::default());
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_cp05_accepts_uppercase_type() {
        let violations = lint_sql("SELECT CAST(1 AS INT)", RuleCP05::default());
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cp05_lower_policy() {
        let rule = RuleCP05 {
            policy: DataTypeCapPolicy::Lower,
        };
        let violations = lint_sql("SELECT CAST(1 AS INT)", rule);
        assert_eq!(violations.len(), 1);
    }
}
