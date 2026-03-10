use rigsql_core::{Segment, SegmentType, TokenKind};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// CV01: Use consistent not-equal operator.
///
/// By default, prefer `!=` over `<>`.
#[derive(Debug)]
pub struct RuleCV01 {
    pub preferred: NotEqualStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotEqualStyle {
    /// Prefer `!=`
    CStyle,
    /// Prefer `<>`
    AnsiStyle,
}

impl Default for RuleCV01 {
    fn default() -> Self {
        Self {
            preferred: NotEqualStyle::CStyle,
        }
    }
}

impl Rule for RuleCV01 {
    fn code(&self) -> &'static str {
        "CV01"
    }
    fn name(&self) -> &'static str {
        "convention.not_equal"
    }
    fn description(&self) -> &'static str {
        "Consistent not-equal operator."
    }
    fn explanation(&self) -> &'static str {
        "SQL has two not-equal operators: '!=' and '<>'. Using one consistently \
         improves readability. The ANSI standard uses '<>' but '!=' is more common \
         in modern SQL and programming."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Convention]
    }
    fn is_fixable(&self) -> bool {
        true
    }

    fn configure(&mut self, settings: &std::collections::HashMap<String, String>) {
        if let Some(val) = settings.get("preferred_not_equal") {
            self.preferred = match val.as_str() {
                "ansi" | "<>" => NotEqualStyle::AnsiStyle,
                _ => NotEqualStyle::CStyle,
            };
        }
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::Segment(vec![SegmentType::ComparisonOperator])
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let Segment::Token(t) = ctx.segment else {
            return vec![];
        };
        if t.token.kind != TokenKind::Neq {
            return vec![];
        }

        let text = t.token.text.as_str();
        match self.preferred {
            NotEqualStyle::CStyle if text == "<>" => {
                vec![LintViolation::new(
                    self.code(),
                    "Use '!=' instead of '<>'.",
                    t.token.span,
                )]
            }
            NotEqualStyle::AnsiStyle if text == "!=" => {
                vec![LintViolation::new(
                    self.code(),
                    "Use '<>' instead of '!='.",
                    t.token.span,
                )]
            }
            _ => vec![],
        }
    }
}
