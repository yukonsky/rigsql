use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// CV08: Use LEFT JOIN instead of RIGHT JOIN.
///
/// LEFT JOIN is more readable and intuitive. Any RIGHT JOIN can be rewritten as a LEFT JOIN.
#[derive(Debug, Default)]
pub struct RuleCV08;

impl Rule for RuleCV08 {
    fn code(&self) -> &'static str {
        "CV08"
    }
    fn name(&self) -> &'static str {
        "convention.left_join"
    }
    fn description(&self) -> &'static str {
        "Use LEFT JOIN instead of RIGHT JOIN."
    }
    fn explanation(&self) -> &'static str {
        "RIGHT JOIN can always be rewritten as LEFT JOIN by swapping the table order. \
         LEFT JOIN is more intuitive because it reads left-to-right: the 'main' table \
         is on the left, and the 'optional' table is on the right."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::JoinClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Look for RIGHT keyword in the join clause
        for child in children {
            if let Segment::Token(t) = child {
                if t.segment_type == SegmentType::Keyword
                    && t.token.text.eq_ignore_ascii_case("RIGHT")
                {
                    return vec![LintViolation::new(
                        self.code(),
                        "Use LEFT JOIN instead of RIGHT JOIN.",
                        t.token.span,
                    )];
                }
            }
        }

        vec![]
    }
}
