use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// TQ01: Avoid using the `sp_` prefix on stored procedures.
///
/// SQL Server treats procedures prefixed with `sp_` as system procedures and
/// searches the `master` database first, which degrades performance.
#[derive(Debug, Default)]
pub struct RuleTQ01;

impl Rule for RuleTQ01 {
    fn code(&self) -> &'static str {
        "TQ01"
    }
    fn name(&self) -> &'static str {
        "tsql.sp_prefix"
    }
    fn description(&self) -> &'static str {
        "Avoid using the sp_ prefix for stored procedures."
    }
    fn explanation(&self) -> &'static str {
        "Stored procedures with the sp_ prefix cause SQL Server to search the master database \
         first before checking the current database. This lookup adds unnecessary overhead and \
         can lead to unexpected behavior if a system procedure with the same name exists."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::ExecStatement])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        if ctx.dialect != "tsql" {
            return vec![];
        }

        let mut violations = Vec::new();

        for child in ctx.segment.children() {
            if child.segment_type() == SegmentType::Identifier {
                let raw = child.raw();
                if raw.to_lowercase().starts_with("sp_") {
                    violations.push(LintViolation::new(
                        self.code(),
                        format!(
                            "Procedure '{}' uses the sp_ prefix which causes SQL Server to \
                             search the master database first.",
                            raw
                        ),
                        child.span(),
                    ));
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql_with_dialect;

    #[test]
    fn test_tq01_flags_sp_prefix() {
        let violations = lint_sql_with_dialect("EXEC sp_helpdb", RuleTQ01, "tsql");
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("sp_helpdb"));
    }

    #[test]
    fn test_tq01_accepts_usp_prefix() {
        let violations = lint_sql_with_dialect("EXEC usp_GetUsers", RuleTQ01, "tsql");
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_tq01_skips_non_tsql() {
        let violations = lint_sql_with_dialect("EXEC sp_helpdb", RuleTQ01, "ansi");
        assert_eq!(violations.len(), 0);
    }
}
