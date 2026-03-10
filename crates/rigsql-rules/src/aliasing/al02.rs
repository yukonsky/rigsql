use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::{has_as_keyword, is_false_alias};
use crate::violation::{LintViolation, SourceEdit};

/// AL02: Implicit aliasing of columns is not allowed.
///
/// Table aliases are checked by AL01; this rule checks column-level aliases.
/// When a column expression has an alias, AS must be explicit.
#[derive(Debug, Default)]
pub struct RuleAL02;

impl Rule for RuleAL02 {
    fn code(&self) -> &'static str { "AL02" }
    fn name(&self) -> &'static str { "aliasing.column" }
    fn description(&self) -> &'static str { "Implicit column aliasing is not allowed." }
    fn explanation(&self) -> &'static str {
        "Column aliases should use the explicit AS keyword. \
         'SELECT col alias' is harder to read than 'SELECT col AS alias'. \
         This is especially important for complex expressions."
    }
    fn groups(&self) -> &[RuleGroup] { &[RuleGroup::Aliasing] }
    fn is_fixable(&self) -> bool { true }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::AliasExpression])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        // Only flag if parent is SelectClause (column aliases, not table aliases)
        let is_in_select = ctx
            .parent
            .is_some_and(|p| p.segment_type() == SegmentType::SelectClause);

        if !is_in_select {
            return vec![];
        }

        // Skip if the "alias" is actually a misidentified keyword (e.g. OVER)
        if is_false_alias(ctx.segment.children()) {
            return vec![];
        }

        if !has_as_keyword(ctx.segment.children()) {
            let children = ctx.segment.children();
            let fix = children.iter().rev()
                .find(|c| !c.segment_type().is_trivia())
                .map(|alias| SourceEdit::insert(alias.span().start, "AS "));

            return vec![LintViolation::with_fix(
                self.code(),
                "Implicit column aliasing not allowed. Use explicit AS keyword.",
                ctx.segment.span(),
                fix.into_iter().collect(),
            )];
        }

        vec![]
    }
}
