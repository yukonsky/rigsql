use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::{has_as_keyword, insert_as_keyword_fix, is_false_alias};
use crate::violation::LintViolation;

/// AL02: Implicit aliasing of columns is not allowed.
///
/// Table aliases are checked by AL01; this rule checks column-level aliases.
/// When a column expression has an alias, AS must be explicit.
#[derive(Debug, Default)]
pub struct RuleAL02;

impl Rule for RuleAL02 {
    fn code(&self) -> &'static str {
        "AL02"
    }
    fn name(&self) -> &'static str {
        "aliasing.column"
    }
    fn description(&self) -> &'static str {
        "Implicit column aliasing is not allowed."
    }
    fn explanation(&self) -> &'static str {
        "Column aliases should use the explicit AS keyword. \
         'SELECT col alias' is harder to read than 'SELECT col AS alias'. \
         This is especially important for complex expressions."
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
        let is_in_select = ctx
            .parent
            .is_some_and(|p| p.segment_type() == SegmentType::SelectClause);
        if !is_in_select {
            return vec![];
        }

        let children = ctx.segment.children();
        if is_false_alias(children) || has_as_keyword(children) {
            return vec![];
        }

        vec![LintViolation::with_fix(
            self.code(),
            "Implicit column aliasing not allowed. Use explicit AS keyword.",
            ctx.segment.span(),
            insert_as_keyword_fix(children),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_al02_flags_implicit_column_alias() {
        let violations = lint_sql("SELECT col alias_name FROM t", RuleAL02);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_al02_accepts_explicit_as() {
        let violations = lint_sql("SELECT col AS alias_name FROM t", RuleAL02);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_al02_skips_non_select() {
        let violations = lint_sql("SELECT * FROM t1 t2", RuleAL02);
        assert_eq!(violations.len(), 0);
    }
}
