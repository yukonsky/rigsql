use rigsql_core::{Segment, SegmentType};

use super::CapitalisationPolicy;
use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::{check_capitalisation, determine_majority_case};
use crate::violation::LintViolation;

/// CP05: Data type names must be consistently capitalised.
///
/// By default expects upper case (INT, VARCHAR, etc.).
#[derive(Debug)]
pub struct RuleCP05 {
    pub policy: CapitalisationPolicy,
}

impl Default for RuleCP05 {
    fn default() -> Self {
        Self {
            policy: CapitalisationPolicy::Upper,
        }
    }
}

/// Check if a token text is purely numeric/punctuation (e.g. "255", "(", ")").
fn is_numeric_or_paren(text: &str) -> bool {
    text.chars()
        .all(|c| c.is_ascii_digit() || c == '(' || c == ')' || c == ',')
}

impl RuleCP05 {
    fn eval_consistent(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut tokens = Vec::new();
        Self::collect_datatype_tokens(ctx.root, &mut tokens);

        if tokens.is_empty() {
            return vec![];
        }

        let majority = determine_majority_case(&tokens);
        let mut violations = Vec::new();
        for (text, span) in &tokens {
            let expected = match majority {
                "upper" => text.to_ascii_uppercase(),
                _ => text.to_ascii_lowercase(),
            };
            if let Some(v) =
                check_capitalisation(self.code(), "Data type", text, &expected, majority, *span)
            {
                violations.push(v);
            }
        }
        violations
    }

    fn collect_datatype_tokens(segment: &Segment, out: &mut Vec<(String, rigsql_core::Span)>) {
        if segment.segment_type() == SegmentType::DataType {
            for token in segment.tokens() {
                let text = token.text.as_str();
                if !is_numeric_or_paren(text) {
                    out.push((text.to_string(), token.span));
                }
            }
        }
        for child in segment.children() {
            Self::collect_datatype_tokens(child, out);
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
        if let Some(policy) = settings.get("capitalisation_policy") {
            self.policy = CapitalisationPolicy::from_config(policy);
        }
    }

    fn crawl_type(&self) -> CrawlType {
        if self.policy == CapitalisationPolicy::Consistent {
            CrawlType::RootOnly
        } else {
            CrawlType::Segment(vec![SegmentType::DataType])
        }
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        if self.policy == CapitalisationPolicy::Consistent {
            return self.eval_consistent(ctx);
        }

        // DataType node may contain keyword tokens (INT, VARCHAR, etc.)
        let tokens = ctx.segment.tokens();
        let mut violations = Vec::new();

        for token in tokens {
            let text = token.text.as_str();
            if is_numeric_or_paren(text) {
                continue;
            }

            let (expected, policy_name) = match self.policy {
                CapitalisationPolicy::Upper => (text.to_ascii_uppercase(), "upper"),
                CapitalisationPolicy::Lower => (text.to_ascii_lowercase(), "lower"),
                CapitalisationPolicy::Capitalise => (crate::utils::capitalise(text), "capitalised"),
                CapitalisationPolicy::Consistent => unreachable!(),
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
            policy: CapitalisationPolicy::Lower,
        };
        let violations = lint_sql("SELECT CAST(1 AS INT)", rule);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_cp05_consistent_all_same_no_violation() {
        let rule = RuleCP05 {
            policy: CapitalisationPolicy::Consistent,
        };
        let violations = lint_sql("SELECT CAST(1 AS INT), CAST(2 AS VARCHAR)", rule);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cp05_consistent_flags_minority() {
        // 2 upper (INT, VARCHAR) vs 1 lower (text) → majority upper, flag "text"
        let rule = RuleCP05 {
            policy: CapitalisationPolicy::Consistent,
        };
        let violations = lint_sql(
            "SELECT CAST(1 AS INT), CAST(2 AS VARCHAR), CAST(3 AS text)",
            rule,
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "TEXT");
    }

    #[test]
    fn test_cp05_consistent_majority_lower() {
        // 2 lower (int, varchar) vs 1 upper (TEXT) → majority lower, flag "TEXT"
        let rule = RuleCP05 {
            policy: CapitalisationPolicy::Consistent,
        };
        let violations = lint_sql(
            "SELECT CAST(1 AS int), CAST(2 AS varchar), CAST(3 AS TEXT)",
            rule,
        );
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "text");
    }
}
