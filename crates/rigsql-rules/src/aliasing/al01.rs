use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::{has_as_keyword, insert_as_keyword_fix, is_false_alias};
use crate::violation::LintViolation;

/// AL01: Implicit aliasing of table/column is not allowed.
///
/// Use explicit `AS` keyword for all aliases.
#[derive(Debug, Default)]
pub struct RuleAL01;

impl Rule for RuleAL01 {
    fn code(&self) -> &'static str {
        "AL01"
    }
    fn name(&self) -> &'static str {
        "aliasing.table"
    }
    fn description(&self) -> &'static str {
        "Implicit aliasing of table/column is not allowed."
    }
    fn explanation(&self) -> &'static str {
        "Aliases should use the explicit AS keyword for clarity. \
         'SELECT a alias_name' is harder to read than 'SELECT a AS alias_name'. \
         Explicit aliasing makes the intent clear and prevents ambiguity."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Aliasing]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::AliasExpression])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();
        if is_false_alias(children) || has_as_keyword(children) {
            return vec![];
        }

        vec![LintViolation::with_fix(
            self.code(),
            "Implicit aliasing not allowed. Use explicit AS keyword.",
            ctx.segment.span(),
            insert_as_keyword_fix(children),
        )]
    }
}
