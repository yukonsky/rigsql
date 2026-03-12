use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// CV11: Enforce consistent type casting style.
///
/// By default, prefer CAST(x AS type) over :: syntax.
#[derive(Debug)]
pub struct RuleCV11 {
    pub preferred_style: CastingStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastingStyle {
    /// Prefer CAST(x AS type)
    Cast,
    /// Prefer x::type (PostgreSQL)
    DoubleColon,
}

impl Default for RuleCV11 {
    fn default() -> Self {
        Self {
            preferred_style: CastingStyle::Cast,
        }
    }
}

impl Rule for RuleCV11 {
    fn code(&self) -> &'static str {
        "CV11"
    }
    fn name(&self) -> &'static str {
        "convention.casting_style"
    }
    fn description(&self) -> &'static str {
        "Enforce consistent type casting style."
    }
    fn explanation(&self) -> &'static str {
        "SQL has multiple ways to cast types: CAST(x AS type), x::type, and CONVERT(). \
         Using a consistent style improves readability. By default, the ANSI CAST() \
         syntax is preferred."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(val) = settings.get("preferred_type_casting_style") {
            self.preferred_style = match val.as_str() {
                "::" | "shorthand" => CastingStyle::DoubleColon,
                _ => CastingStyle::Cast,
            };
        }
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![
            SegmentType::CastExpression,
            SegmentType::TypeCastExpression,
        ])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let st = ctx.segment.segment_type();

        match self.preferred_style {
            CastingStyle::Cast if st == SegmentType::TypeCastExpression => {
                vec![LintViolation::with_msg_key(
                    self.code(),
                    "Use CAST(x AS type) instead of :: syntax.",
                    ctx.segment.span(),
                    "rules.CV11.msg.cast",
                    vec![],
                )]
            }
            CastingStyle::DoubleColon if st == SegmentType::CastExpression => {
                vec![LintViolation::with_msg_key(
                    self.code(),
                    "Use :: syntax instead of CAST(x AS type).",
                    ctx.segment.span(),
                    "rules.CV11.msg.shorthand",
                    vec![],
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
    fn test_cv11_accepts_cast_by_default() {
        let violations = lint_sql("SELECT CAST(x AS INT) FROM t", RuleCV11::default());
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cv11_double_colon_policy_flags_cast() {
        let rule = RuleCV11 {
            preferred_style: CastingStyle::DoubleColon,
        };
        let violations = lint_sql("SELECT CAST(x AS INT) FROM t", rule);
        assert_eq!(violations.len(), 1);
    }
}
