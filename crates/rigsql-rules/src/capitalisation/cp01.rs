use rigsql_core::{Segment, SegmentType, TokenKind};
use rigsql_lexer::is_keyword;

use super::CapitalisationPolicy;
use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::{check_capitalisation, collect_matching_tokens, determine_majority_case};
use crate::violation::LintViolation;

/// CP01: Keywords must be consistently capitalised.
///
/// By default, expects UPPER case keywords.
#[derive(Debug)]
pub struct RuleCP01 {
    pub policy: CapitalisationPolicy,
}

impl Default for RuleCP01 {
    fn default() -> Self {
        Self {
            policy: CapitalisationPolicy::Upper,
        }
    }
}

impl Rule for RuleCP01 {
    fn code(&self) -> &'static str {
        "CP01"
    }
    fn name(&self) -> &'static str {
        "capitalisation.keywords"
    }
    fn description(&self) -> &'static str {
        "Keywords must be consistently capitalised."
    }
    fn explanation(&self) -> &'static str {
        "SQL keywords like SELECT, FROM, WHERE should use consistent capitalisation. \
         Mixed case reduces readability. Most style guides recommend UPPER case keywords \
         to distinguish them from identifiers."
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
            CrawlType::Segment(vec![SegmentType::Keyword, SegmentType::Unparsable])
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

        let Segment::Token(t) = ctx.segment else {
            return vec![];
        };
        if t.token.kind != TokenKind::Word || !is_keyword(&t.token.text) {
            return vec![];
        }

        let text = t.token.text.as_str();
        let (expected, policy_name) = match self.policy {
            CapitalisationPolicy::Upper => (text.to_ascii_uppercase(), "upper"),
            CapitalisationPolicy::Lower => (text.to_ascii_lowercase(), "lower"),
            CapitalisationPolicy::Capitalise => (crate::utils::capitalise(text), "capitalised"),
            CapitalisationPolicy::Consistent => unreachable!(),
        };

        check_capitalisation(
            self.code(),
            "Keywords",
            text,
            &expected,
            policy_name,
            t.token.span,
        )
        .into_iter()
        .collect()
    }
}

impl RuleCP01 {
    fn eval_consistent(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut tokens = Vec::new();
        collect_matching_tokens(
            ctx.root,
            &|seg| {
                if let Segment::Token(t) = seg {
                    if t.segment_type == SegmentType::Keyword
                        && t.token.kind == TokenKind::Word
                        && is_keyword(&t.token.text)
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
            if let Some(v) =
                check_capitalisation(self.code(), "Keywords", text, &expected, majority, *span)
            {
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
    fn test_cp01_flags_lowercase_keyword() {
        let violations = lint_sql("select 1", RuleCP01::default());
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_cp01_accepts_uppercase_keyword() {
        let violations = lint_sql("SELECT 1", RuleCP01::default());
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cp01_fix_replaces_to_upper() {
        let violations = lint_sql("select 1", RuleCP01::default());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "SELECT");
    }

    #[test]
    fn test_cp01_lower_policy() {
        let rule = RuleCP01 {
            policy: CapitalisationPolicy::Lower,
        };
        let violations = lint_sql("SELECT 1", rule);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_cp01_consistent_flags_minority() {
        // 3 upper (SELECT, FROM, WHERE) vs 1 lower (and) → majority upper, flag "and"
        let rule = RuleCP01 {
            policy: CapitalisationPolicy::Consistent,
        };
        let violations = lint_sql("SELECT id FROM users where id = 1 AND name = 'a'", rule);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "WHERE");
    }

    #[test]
    fn test_cp01_consistent_all_same_no_violation() {
        let rule = RuleCP01 {
            policy: CapitalisationPolicy::Consistent,
        };
        let violations = lint_sql("SELECT id FROM users WHERE id = 1", rule);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cp01_consistent_majority_lower() {
        // 3 lower (select, from, where) vs 1 upper (AND) → majority lower, flag "AND"
        let rule = RuleCP01 {
            policy: CapitalisationPolicy::Consistent,
        };
        let violations = lint_sql("select id from users where id = 1 AND name = 'a'", rule);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "and");
    }

    #[test]
    fn test_cp01_multiple_keywords() {
        let violations = lint_sql("select * from users where id = 1", RuleCP01::default());
        let codes: Vec<&str> = violations.iter().map(|v| v.rule_code).collect();
        assert!(codes.iter().all(|&c| c == "CP01"));
        assert!(violations.len() >= 3);
        let fix_texts: Vec<&str> = violations
            .iter()
            .map(|v| v.fixes[0].new_text.as_str())
            .collect();
        assert!(fix_texts.contains(&"SELECT"));
        assert!(fix_texts.contains(&"FROM"));
        assert!(fix_texts.contains(&"WHERE"));
    }
}
