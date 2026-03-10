use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// LT15: DISTINCT should not be followed by a bracket/parenthesis.
///
/// `SELECT DISTINCT(col)` should be `SELECT DISTINCT col`.
#[derive(Debug, Default)]
pub struct RuleLT15;

impl Rule for RuleLT15 {
    fn code(&self) -> &'static str {
        "LT15"
    }
    fn name(&self) -> &'static str {
        "layout.distinct"
    }
    fn description(&self) -> &'static str {
        "DISTINCT used with parentheses."
    }
    fn explanation(&self) -> &'static str {
        "DISTINCT is not a function and should not be used with parentheses. \
         'SELECT DISTINCT(col)' is misleading because the parentheses don't do anything. \
         Write 'SELECT DISTINCT col' instead."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::SelectClause])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // Find DISTINCT keyword
        for (i, child) in children.iter().enumerate() {
            if let Segment::Token(t) = child {
                if t.segment_type == SegmentType::Keyword
                    && t.token.text.eq_ignore_ascii_case("DISTINCT")
                {
                    // Check if next non-trivia is LParen
                    for next in &children[i + 1..] {
                        if next.segment_type().is_trivia() {
                            continue;
                        }
                        if next.segment_type() == SegmentType::LParen
                            || next.segment_type() == SegmentType::ParenExpression
                        {
                            return vec![LintViolation::new(
                                self.code(),
                                "DISTINCT should not be followed by parentheses.",
                                t.token.span,
                            )];
                        }
                        break;
                    }
                }
            }
        }

        vec![]
    }
}
