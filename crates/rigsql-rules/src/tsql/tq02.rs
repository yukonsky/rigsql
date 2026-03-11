use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// TQ02: IF/WHILE blocks should use BEGIN...END for multi-statement bodies.
///
/// This is a stub implementation. Precise detection requires tracking statement
/// counts after control-flow keywords, which is deferred to a future iteration.
#[derive(Debug, Default)]
pub struct RuleTQ02;

impl Rule for RuleTQ02 {
    fn code(&self) -> &'static str {
        "TQ02"
    }
    fn name(&self) -> &'static str {
        "tsql.block_structure"
    }
    fn description(&self) -> &'static str {
        "IF/WHILE blocks should use BEGIN...END for multi-statement bodies."
    }
    fn explanation(&self) -> &'static str {
        "In T-SQL, IF and WHILE without BEGIN...END only execute the immediately following \
         statement. This is a common source of bugs when additional statements are added later. \
         Always wrapping the body in BEGIN...END makes the intent explicit."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::RootOnly
    }

    fn eval(&self, _ctx: &RuleContext) -> Vec<LintViolation> {
        // Stub: precise multi-statement detection is deferred.
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql_with_dialect;

    #[test]
    fn test_tq02_no_false_positives_simple() {
        let violations = lint_sql_with_dialect("SELECT 1", RuleTQ02, "tsql");
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_tq02_no_false_positives_ansi() {
        let violations = lint_sql_with_dialect("SELECT 1", RuleTQ02, "ansi");
        assert_eq!(violations.len(), 0);
    }
}
