use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AL07: Table aliases should follow a naming convention.
///
/// This is a stub rule that is disabled by default. When enabled via
/// `force_enable`, it flags all table aliases as requiring review against
/// team naming conventions.
#[derive(Debug)]
pub struct RuleAL07 {
    pub force_enable: bool,
}

impl Default for RuleAL07 {
    fn default() -> Self {
        Self {
            force_enable: false,
        }
    }
}

impl Rule for RuleAL07 {
    fn code(&self) -> &'static str { "AL07" }
    fn name(&self) -> &'static str { "aliasing.table_naming" }
    fn description(&self) -> &'static str { "Table aliases should follow a naming convention." }
    fn explanation(&self) -> &'static str {
        "Table aliases should be meaningful and follow a consistent naming convention \
         rather than using single letters or arbitrary abbreviations. This rule is \
         disabled by default as naming conventions vary by team."
    }
    fn groups(&self) -> &[RuleGroup] { &[RuleGroup::Aliasing] }
    fn is_fixable(&self) -> bool { false }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(val) = settings.get("force_enable") {
            self.force_enable = val.eq_ignore_ascii_case("true") || val == "1";
        }
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::AliasExpression])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        if !self.force_enable {
            return vec![];
        }

        // Only apply to table aliases (FROM/JOIN context)
        let in_table_context = ctx.parent.is_some_and(|p| {
            let pt = p.segment_type();
            pt == SegmentType::FromClause || pt == SegmentType::JoinClause
        });

        if !in_table_context {
            return vec![];
        }

        vec![LintViolation::new(
            self.code(),
            "Table alias does not follow naming convention.",
            ctx.segment.span(),
        )]
    }
}
