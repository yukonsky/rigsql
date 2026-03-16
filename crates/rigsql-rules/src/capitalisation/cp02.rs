use rigsql_core::{Segment, SegmentType, TokenKind};
use rigsql_lexer::is_keyword;

use super::CapitalisationPolicy;
use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::{check_capitalisation, collect_matching_tokens, determine_majority_case};
use crate::violation::{LintViolation, SourceEdit};

/// CP02: Identifiers (non-keywords) must be consistently capitalised.
///
/// By default, expects consistent case identifiers.
#[derive(Debug)]
pub struct RuleCP02 {
    pub policy: CapitalisationPolicy,
}

impl Default for RuleCP02 {
    fn default() -> Self {
        Self {
            policy: CapitalisationPolicy::Consistent,
        }
    }
}

impl RuleCP02 {
    /// Check if an identifier token should be skipped.
    fn should_skip(seg: &Segment, parent: Option<&Segment>) -> bool {
        let Segment::Token(t) = seg else {
            return true;
        };
        if t.token.kind != TokenKind::Word {
            return true;
        }
        if is_keyword(&t.token.text) {
            return true;
        }
        if let Some(p) = parent {
            if p.segment_type() == SegmentType::FunctionCall {
                return true;
            }
        }
        if !t.token.text.is_ascii() {
            return true;
        }
        false
    }

    fn eval_consistent(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut tokens = Vec::new();
        collect_matching_tokens(
            ctx.root,
            &|seg| {
                if let Segment::Token(t) = seg {
                    if t.segment_type == SegmentType::Identifier
                        && t.token.kind == TokenKind::Word
                        && !is_keyword(&t.token.text)
                        && t.token.text.is_ascii()
                    {
                        return Some((t.token.text.to_string(), t.token.span));
                    }
                }
                None
            },
            &mut tokens,
        );

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
            if let Some(v) = check_capitalisation(
                self.code(),
                "Unquoted identifiers",
                text,
                &expected,
                majority,
                *span,
            ) {
                violations.push(v);
            }
        }
        violations
    }
}

impl Rule for RuleCP02 {
    fn code(&self) -> &'static str {
        "CP02"
    }
    fn name(&self) -> &'static str {
        "capitalisation.identifiers"
    }
    fn description(&self) -> &'static str {
        "Unquoted identifiers must be consistently capitalised."
    }
    fn explanation(&self) -> &'static str {
        "Unquoted identifiers (table names, column names) should use consistent capitalisation. \
         Most SQL style guides recommend lower_snake_case for identifiers."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Capitalisation]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        if self.policy == CapitalisationPolicy::Consistent {
            CrawlType::RootOnly
        } else {
            CrawlType::Segment(vec![SegmentType::Identifier])
        }
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(policy) = settings.get("capitalisation_policy") {
            self.policy = CapitalisationPolicy::from_config(policy);
        }
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        if self.policy == CapitalisationPolicy::Consistent {
            return self.eval_consistent(ctx);
        }

        if Self::should_skip(ctx.segment, ctx.parent) {
            return vec![];
        }

        let Segment::Token(t) = ctx.segment else {
            return vec![];
        };
        let text = t.token.text.as_str();

        let (expected, policy_name) = match self.policy {
            CapitalisationPolicy::Upper => (text.to_ascii_uppercase(), "upper"),
            CapitalisationPolicy::Lower => (text.to_ascii_lowercase(), "lower"),
            CapitalisationPolicy::Capitalise => (crate::utils::capitalise(text), "capitalised"),
            CapitalisationPolicy::Consistent => unreachable!(),
        };

        if text != expected {
            vec![LintViolation::with_fix_and_msg_key(
                self.code(),
                format!(
                    "Unquoted identifiers must be {} case. Found '{}'.",
                    policy_name, text
                ),
                t.token.span,
                vec![SourceEdit::replace(t.token.span, expected.clone())],
                "rules.CP02.msg",
                vec![
                    ("policy".to_string(), policy_name.to_string()),
                    ("found".to_string(), text.to_string()),
                ],
            )]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_cp02_lower_policy_flags_upper() {
        let rule = RuleCP02 {
            policy: CapitalisationPolicy::Lower,
        };
        let violations = lint_sql("SELECT Users FROM t", rule);
        assert!(!violations.is_empty());
    }

    #[test]
    fn test_cp02_skips_keywords() {
        let rule = RuleCP02 {
            policy: CapitalisationPolicy::Lower,
        };
        let violations = lint_sql("SELECT id FROM users", rule);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cp02_skips_function_parent() {
        let rule = RuleCP02 {
            policy: CapitalisationPolicy::Lower,
        };
        let violations = lint_sql("SELECT COUNT(id) FROM users", rule);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cp02_consistent_all_lower_no_violation() {
        let rule = RuleCP02 {
            policy: CapitalisationPolicy::Consistent,
        };
        let violations = lint_sql("SELECT id, name FROM users", rule);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cp02_consistent_flags_minority() {
        // 3 lower (id, name, users) vs 1 upper (AGE) → majority lower, flag "AGE"
        let rule = RuleCP02 {
            policy: CapitalisationPolicy::Consistent,
        };
        let violations = lint_sql("SELECT id, name, AGE FROM users", rule);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "age");
    }

    #[test]
    fn test_cp02_consistent_majority_upper() {
        // 3 upper (ID, NAME, USERS) vs 1 lower (age) → majority upper, flag "age"
        let rule = RuleCP02 {
            policy: CapitalisationPolicy::Consistent,
        };
        let violations = lint_sql("SELECT ID, NAME, age FROM USERS", rule);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "AGE");
    }
}
