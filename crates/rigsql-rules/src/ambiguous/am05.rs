use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// AM05: JOIN without qualifier (INNER/LEFT/RIGHT/FULL/CROSS).
///
/// A bare JOIN is implicitly an INNER JOIN, but this should be explicit.
#[derive(Debug, Default)]
pub struct RuleAM05;

impl Rule for RuleAM05 {
    fn code(&self) -> &'static str {
        "AM05"
    }
    fn name(&self) -> &'static str {
        "ambiguous.join"
    }
    fn description(&self) -> &'static str {
        "JOIN without qualifier."
    }
    fn explanation(&self) -> &'static str {
        "A bare JOIN keyword without a qualifier (INNER, LEFT, RIGHT, FULL, CROSS, NATURAL) \
         implicitly means INNER JOIN. Making the join type explicit improves readability \
         and makes the query's intent clear."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Ambiguous]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::JoinClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Look for the JOIN keyword
        let join_keyword = children.iter().find(|c| {
            if let Segment::Token(t) = c {
                t.segment_type == SegmentType::Keyword && t.token.text.eq_ignore_ascii_case("JOIN")
            } else {
                false
            }
        });

        let join_kw = match join_keyword {
            Some(kw) => kw,
            None => return vec![],
        };

        // Check if there's a qualifier keyword BEFORE the JOIN keyword
        let qualifiers = [
            "INNER", "LEFT", "RIGHT", "FULL", "CROSS", "NATURAL", "OUTER",
        ];

        let has_qualifier = children
            .iter()
            .take_while(|c| !std::ptr::eq(*c, join_kw))
            .any(|c| {
                if let Segment::Token(t) = c {
                    t.segment_type == SegmentType::Keyword
                        && qualifiers
                            .iter()
                            .any(|q| t.token.text.eq_ignore_ascii_case(q))
                } else {
                    false
                }
            });

        if !has_qualifier {
            return vec![LintViolation::new(
                self.code(),
                "JOIN without qualifier. Use INNER JOIN, LEFT JOIN, etc.",
                join_kw.span(),
            )];
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_am05_flags_bare_join() {
        let violations = lint_sql("SELECT a FROM t JOIN u ON t.id = u.id", RuleAM05);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_am05_accepts_inner_join() {
        let violations = lint_sql("SELECT a FROM t INNER JOIN u ON t.id = u.id", RuleAM05);
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_am05_accepts_left_join() {
        let violations = lint_sql("SELECT a FROM t LEFT JOIN u ON t.id = u.id", RuleAM05);
        assert_eq!(violations.len(), 0);
    }
}
