use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// TQ01: Avoid using the `sp_` prefix on stored procedures.
///
/// SQL Server treats procedures prefixed with `sp_` as system procedures and
/// searches the `master` database first, which degrades performance.
///
/// This rule should check CREATE PROCEDURE statements, but since our parser
/// doesn't yet support CREATE PROCEDURE, this rule is currently a stub that
/// produces no violations. It will be implemented when CREATE PROCEDURE
/// parsing is added.
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
         can lead to unexpected behavior if a system procedure with the same name exists. \
         Use a different prefix such as usp_ for user-defined stored procedures."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        // Stub: needs CreateProcedureStatement support in the parser
        CrawlType::RootOnly
    }

    fn eval(&self, _ctx: &RuleContext) -> Vec<LintViolation> {
        // Stub: CREATE PROCEDURE parsing not yet supported
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql_with_dialect;

    #[test]
    fn test_tq01_stub_no_violations() {
        // Currently a stub until CREATE PROCEDURE parsing is supported
        let violations = lint_sql_with_dialect("EXEC sp_helpdb", RuleTQ01, "tsql");
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_tq01_stub_no_violations_ansi() {
        let violations = lint_sql_with_dialect("EXEC sp_helpdb", RuleTQ01, "ansi");
        assert_eq!(violations.len(), 0);
    }
}
