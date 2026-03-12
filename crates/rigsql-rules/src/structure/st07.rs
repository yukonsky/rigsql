use rigsql_core::SegmentType;

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// ST07: Prefer ON clause over USING clause in joins.
///
/// USING clause is less explicit than ON and can cause ambiguity.
#[derive(Debug, Default)]
pub struct RuleST07;

impl Rule for RuleST07 {
    fn code(&self) -> &'static str {
        "ST07"
    }
    fn name(&self) -> &'static str {
        "structure.using"
    }
    fn description(&self) -> &'static str {
        "Prefer explicit ON clause over USING clause in joins."
    }
    fn explanation(&self) -> &'static str {
        "The USING clause in a JOIN is a shorthand for matching columns with the same name. \
         While concise, it can be less clear than an explicit ON clause. Using ON makes the \
         join condition explicit and avoids potential ambiguity."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Structure]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::JoinClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        for child in ctx.segment.children() {
            if child.segment_type() == SegmentType::UsingClause {
                return vec![LintViolation::with_msg_key(
                    self.code(),
                    "Prefer ON clause over USING clause in joins.",
                    child.span(),
                    "rules.ST07.msg",
                    vec![],
                )];
            }
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_st07_flags_using_clause() {
        let violations = lint_sql("SELECT * FROM a JOIN b USING (id);", RuleST07);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("USING"));
    }

    #[test]
    fn test_st07_accepts_on_clause() {
        let violations = lint_sql("SELECT * FROM a JOIN b ON a.id = b.id;", RuleST07);
        assert_eq!(violations.len(), 0);
    }
}
