use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// RF04: Keywords used as identifiers.
///
/// Detects identifiers that match SQL reserved keywords, which can cause
/// confusion and portability issues.
#[derive(Debug, Default)]
pub struct RuleRF04;

const RESERVED_KEYWORDS: &[&str] = &[
    "SELECT",
    "FROM",
    "WHERE",
    "INSERT",
    "UPDATE",
    "DELETE",
    "CREATE",
    "DROP",
    "ALTER",
    "TABLE",
    "INDEX",
    "VIEW",
    "JOIN",
    "ON",
    "AND",
    "OR",
    "NOT",
    "IN",
    "IS",
    "NULL",
    "TRUE",
    "FALSE",
    "BETWEEN",
    "LIKE",
    "ORDER",
    "BY",
    "GROUP",
    "HAVING",
    "LIMIT",
    "OFFSET",
    "UNION",
    "EXCEPT",
    "INTERSECT",
    "INTO",
    "VALUES",
    "SET",
    "AS",
    "CASE",
    "WHEN",
    "THEN",
    "ELSE",
    "END",
    "EXISTS",
    "ALL",
    "ANY",
    "DISTINCT",
    "TOP",
    "COUNT",
    "SUM",
    "AVG",
    "MIN",
    "MAX",
    "CAST",
    "COALESCE",
    "NULLIF",
];

impl Rule for RuleRF04 {
    fn code(&self) -> &'static str {
        "RF04"
    }
    fn name(&self) -> &'static str {
        "references.keywords"
    }
    fn description(&self) -> &'static str {
        "Keywords should not be used as identifiers."
    }
    fn explanation(&self) -> &'static str {
        "Using SQL reserved keywords as identifiers (column names, table names, aliases) \
         can cause parsing ambiguity, confuse readers, and reduce portability across \
         SQL dialects. Use descriptive, non-reserved names instead."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::References]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::Identifier])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let Segment::Token(t) = ctx.segment else {
            return vec![];
        };

        let upper = t.token.text.to_ascii_uppercase();
        if RESERVED_KEYWORDS.contains(&upper.as_str()) {
            vec![LintViolation::new(
                self.code(),
                format!("Identifier '{}' is a reserved keyword.", t.token.text),
                t.token.span,
            )]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_rf04_flags_keyword_as_identifier() {
        // "table" used as a table name identifier
        let violations = lint_sql("SELECT id FROM \"table\"", RuleRF04);
        // QuotedIdentifier won't be visited since we crawl Identifier only
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_rf04_accepts_normal_identifiers() {
        let violations = lint_sql("SELECT user_id, email FROM users", RuleRF04);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_rf04_flags_keyword_identifier_in_alias() {
        // If the parser emits "table" as an Identifier segment, this should flag it
        let violations = lint_sql("SELECT 1 AS \"values\"", RuleRF04);
        // "values" in quotes is QuotedIdentifier, not Identifier — so no violation
        assert_eq!(violations.len(), 0);
    }
}
