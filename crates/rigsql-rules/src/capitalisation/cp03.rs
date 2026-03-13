use rigsql_core::{Segment, SegmentType, TokenKind};

use super::CapitalisationPolicy;
use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::utils::check_capitalisation;
use crate::violation::LintViolation;

/// Built-in SQL function names (sorted alphabetically for binary_search).
const BUILTIN_FUNCTIONS: &[&str] = &[
    "ABS",
    "ACOS",
    "APP_NAME",
    "ASCII",
    "ASIN",
    "ATAN",
    "ATAN2",
    "AVG",
    "CAST",
    "CEILING",
    "CHAR",
    "CHARINDEX",
    "CHOOSE",
    "COALESCE",
    "CONCAT",
    "CONCAT_WS",
    "CONVERT",
    "COS",
    "COT",
    "COUNT",
    "COUNT_BIG",
    "CUME_DIST",
    "CURRENT_TIMESTAMP",
    "CURRENT_USER",
    "CURSOR_STATUS",
    "DATALENGTH",
    "DATEADD",
    "DATEDIFF",
    "DATEDIFF_BIG",
    "DATEFROMPARTS",
    "DATENAME",
    "DATEPART",
    "DATETIME2FROMPARTS",
    "DATETIMEFROMPARTS",
    "DAY",
    "DB_ID",
    "DB_NAME",
    "DENSE_RANK",
    "DIFFERENCE",
    "EOMONTH",
    "ERROR_LINE",
    "ERROR_MESSAGE",
    "ERROR_NUMBER",
    "ERROR_PROCEDURE",
    "ERROR_SEVERITY",
    "ERROR_STATE",
    "EXP",
    "FIRST_VALUE",
    "FLOOR",
    "FORMAT",
    "GETDATE",
    "GETUTCDATE",
    "GREATEST",
    "GROUPING",
    "GROUPING_ID",
    "HAS_PERMS_BY_NAME",
    "HOST_NAME",
    "IDENTITY",
    "IDENT_CURRENT",
    "IFNULL",
    "IIF",
    "ISJSON",
    "ISNULL",
    "ISNUMERIC",
    "JSON_ARRAY",
    "JSON_MODIFY",
    "JSON_OBJECT",
    "JSON_QUERY",
    "JSON_VALUE",
    "LAG",
    "LAST_VALUE",
    "LEAD",
    "LEAST",
    "LEFT",
    "LEN",
    "LENGTH",
    "LOG",
    "LOG10",
    "LOWER",
    "LTRIM",
    "MAX",
    "MIN",
    "MONTH",
    "NCHAR",
    "NEWID",
    "NTILE",
    "NULLIF",
    "NVL",
    "NVL2",
    "OBJECT_ID",
    "OBJECT_NAME",
    "PARSENAME",
    "PATINDEX",
    "PERCENT_RANK",
    "PI",
    "POWER",
    "QUOTENAME",
    "RAND",
    "RANK",
    "REPLACE",
    "REPLICATE",
    "REVERSE",
    "RIGHT",
    "ROUND",
    "ROW_NUMBER",
    "RTRIM",
    "SCHEMA_NAME",
    "SCOPE_IDENTITY",
    "SIGN",
    "SIN",
    "SOUNDEX",
    "SPACE",
    "SQRT",
    "SQUARE",
    "STR",
    "STRING_AGG",
    "STRING_SPLIT",
    "STUFF",
    "SUBSTRING",
    "SUM",
    "SUSER_SNAME",
    "SWITCHOFFSET",
    "SYSDATETIME",
    "SYSUTCDATETIME",
    "TAN",
    "TODATETIMEOFFSET",
    "TRANSLATE",
    "TRIM",
    "TRY_CAST",
    "TRY_CONVERT",
    "TRY_PARSE",
    "TYPE_NAME",
    "UNICODE",
    "UPPER",
    "USER_NAME",
    "YEAR",
];

/// CP03: Function names must be consistently capitalised.
///
/// By default, expects UPPER case function names (sqlfluff-compatible).
#[derive(Debug)]
pub struct RuleCP03 {
    pub policy: CapitalisationPolicy,
}

impl Default for RuleCP03 {
    fn default() -> Self {
        Self {
            policy: CapitalisationPolicy::Upper,
        }
    }
}

impl Rule for RuleCP03 {
    fn code(&self) -> &'static str {
        "CP03"
    }
    fn name(&self) -> &'static str {
        "capitalisation.functions"
    }
    fn description(&self) -> &'static str {
        "Function names must be consistently capitalised."
    }
    fn explanation(&self) -> &'static str {
        "Function names like COUNT, SUM, COALESCE should be consistently capitalised. \
         Whether upper or lower depends on your team's convention."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Capitalisation]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::FunctionCall])
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(policy) = settings.get("capitalisation_policy") {
            self.policy = CapitalisationPolicy::from_config(policy);
        }
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        // FunctionCall's first child should be the function name (Identifier)
        let children = ctx.segment.children();
        if children.is_empty() {
            return vec![];
        }

        // Walk to find the function name token
        let name_seg = Self::find_function_name(children);
        let Some(Segment::Token(t)) = name_seg else {
            return vec![];
        };
        if t.token.kind != TokenKind::Word {
            return vec![];
        }

        let text = t.token.text.as_str();
        let upper = text.to_ascii_uppercase();

        // Only check built-in SQL functions; skip user-defined functions
        if BUILTIN_FUNCTIONS.binary_search(&upper.as_str()).is_err() {
            return vec![];
        }

        let (expected, policy_name) = match self.policy {
            CapitalisationPolicy::Upper => (upper, "upper"),
            CapitalisationPolicy::Lower => (text.to_ascii_lowercase(), "lower"),
            CapitalisationPolicy::Capitalise => (crate::utils::capitalise(text), "capitalised"),
        };

        check_capitalisation(
            self.code(),
            "Function names",
            text,
            &expected,
            policy_name,
            t.token.span,
        )
        .into_iter()
        .collect()
    }
}

impl RuleCP03 {
    fn find_function_name(children: &[Segment]) -> Option<&Segment> {
        for child in children {
            match child.segment_type() {
                SegmentType::Identifier => return Some(child),
                SegmentType::ColumnRef => {
                    // qualified function: schema.func — get last identifier
                    let inner = child.children();
                    return inner
                        .iter()
                        .rev()
                        .find(|s| s.segment_type() == SegmentType::Identifier);
                }
                _ if child.segment_type().is_trivia() => continue,
                _ => break,
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_cp03_flags_lowercase_function() {
        // Default policy is upper, so lowercase should be flagged
        let violations = lint_sql("SELECT count(*) FROM t", RuleCP03::default());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "COUNT");
    }

    #[test]
    fn test_cp03_flags_mixed_case() {
        let violations = lint_sql("SELECT Count(*) FROM t", RuleCP03::default());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "COUNT");
    }

    #[test]
    fn test_cp03_accepts_all_upper() {
        let violations = lint_sql("SELECT COUNT(*) FROM t", RuleCP03::default());
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cp03_lower_policy_flags_upper() {
        let rule = RuleCP03 {
            policy: CapitalisationPolicy::Lower,
        };
        let violations = lint_sql("SELECT COUNT(*) FROM t", rule);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "count");
    }

    #[test]
    fn test_cp03_lower_policy_accepts_lower() {
        let rule = RuleCP03 {
            policy: CapitalisationPolicy::Lower,
        };
        let violations = lint_sql("SELECT count(*) FROM t", rule);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cp03_capitalise_policy() {
        let rule = RuleCP03 {
            policy: CapitalisationPolicy::Capitalise,
        };
        let violations = lint_sql("SELECT count(*) FROM t", rule);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "Count");
    }

    #[test]
    fn test_cp03_skips_user_defined_function() {
        let violations = lint_sql(
            "SELECT GetDropdownOptions('a', 'b') FROM t",
            RuleCP03::default(),
        );
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_cp03_flags_replace_function() {
        // The issue from #32: replace should be flagged and fixed to REPLACE
        let violations = lint_sql("SELECT replace(col, 'a', 'b') FROM t", RuleCP03::default());
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].fixes[0].new_text, "REPLACE");
    }
}
