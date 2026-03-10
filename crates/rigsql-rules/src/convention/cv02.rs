use rigsql_core::{Segment, SegmentType};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// CV02: Use COALESCE instead of IFNULL or NVL.
///
/// COALESCE is ANSI standard and portable across databases.
#[derive(Debug, Default)]
pub struct RuleCV02;

impl Rule for RuleCV02 {
    fn code(&self) -> &'static str { "CV02" }
    fn name(&self) -> &'static str { "convention.coalesce" }
    fn description(&self) -> &'static str { "Use COALESCE instead of IFNULL or NVL." }
    fn explanation(&self) -> &'static str {
        "COALESCE is the ANSI SQL standard function for handling NULL values. \
         IFNULL (MySQL) and NVL (Oracle) are database-specific alternatives. \
         Using COALESCE improves portability and consistency."
    }
    fn groups(&self) -> &[RuleGroup] { &[RuleGroup::Convention] }
    fn is_fixable(&self) -> bool { true }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::FunctionCall])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let children = ctx.segment.children();

        // First non-trivia child should be the function name
        let func_name = children.iter().find(|c| !c.segment_type().is_trivia());

        if let Some(Segment::Token(t)) = func_name {
            let name = t.token.text.as_str();
            if name.eq_ignore_ascii_case("IFNULL") || name.eq_ignore_ascii_case("NVL") {
                return vec![LintViolation::new(
                    self.code(),
                    format!("Use COALESCE instead of '{}'.", name),
                    t.token.span,
                )];
            }
        }

        vec![]
    }
}
