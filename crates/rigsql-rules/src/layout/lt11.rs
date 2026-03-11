use rigsql_core::{Segment, SegmentType, TokenSegment};

use crate::rule::{CrawlType, Rule, RuleContext, RuleGroup};
use crate::violation::LintViolation;

/// LT11: Set operators (UNION, INTERSECT, EXCEPT) should be surrounded by newlines.
///
/// Set operators should have a newline before and after them for readability.
#[derive(Debug, Default)]
pub struct RuleLT11;

impl Rule for RuleLT11 {
    fn code(&self) -> &'static str {
        "LT11"
    }
    fn name(&self) -> &'static str {
        "layout.set_operator_newline"
    }
    fn description(&self) -> &'static str {
        "Set operators should be surrounded by newlines."
    }
    fn explanation(&self) -> &'static str {
        "Set operators such as UNION, INTERSECT, and EXCEPT combine the results of \
         multiple queries. They should be surrounded by newlines to visually separate \
         the individual queries and improve readability."
    }
    fn groups(&self) -> &[RuleGroup] {
        &[RuleGroup::Layout]
    }
    fn is_fixable(&self) -> bool {
        false
    }

    fn crawl_type(&self) -> CrawlType {
        CrawlType::RootOnly
    }

    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation> {
        let mut violations = Vec::new();
        let tokens = collect_leaf_tokens(ctx.segment);

        for (i, t) in tokens.iter().enumerate() {
            if !t.token.text.eq_ignore_ascii_case("UNION")
                && !t.token.text.eq_ignore_ascii_case("INTERSECT")
                && !t.token.text.eq_ignore_ascii_case("EXCEPT")
            {
                continue;
            }

            let op_span = t.token.span;

            let has_newline_before = check_adjacent_newline(&tokens, i, Direction::Before);

            // Determine end of set operator (skip ALL if present)
            let mut end_idx = i;
            let mut j = i + 1;
            while j < tokens.len() {
                if tokens[j].segment_type.is_trivia() {
                    j += 1;
                } else {
                    if tokens[j].token.text.eq_ignore_ascii_case("ALL") {
                        end_idx = j;
                    }
                    break;
                }
            }

            let has_newline_after = check_adjacent_newline(&tokens, end_idx, Direction::After);

            if !has_newline_before {
                violations.push(LintViolation::new(
                    self.code(),
                    format!(
                        "Expected newline before '{}'.",
                        t.token.text.to_ascii_uppercase()
                    ),
                    op_span,
                ));
            }

            if !has_newline_after {
                violations.push(LintViolation::new(
                    self.code(),
                    format!(
                        "Expected newline after '{}'.",
                        t.token.text.to_ascii_uppercase()
                    ),
                    op_span,
                ));
            }
        }

        violations
    }
}

enum Direction {
    Before,
    After,
}

fn check_adjacent_newline(tokens: &[TokenSegment], idx: usize, dir: Direction) -> bool {
    let mut j = match dir {
        Direction::Before => idx.wrapping_sub(1),
        Direction::After => idx + 1,
    };
    loop {
        if j >= tokens.len() {
            return false;
        }
        if tokens[j].segment_type == SegmentType::Newline {
            return true;
        }
        if tokens[j].segment_type != SegmentType::Whitespace {
            return false;
        }
        j = match dir {
            Direction::Before => j.wrapping_sub(1),
            Direction::After => j + 1,
        };
    }
}

/// Collect all leaf TokenSegments by cloning (cheap: Token contains SmolStr).
fn collect_leaf_tokens(segment: &Segment) -> Vec<TokenSegment> {
    let mut out = Vec::new();
    collect_leaf_tokens_inner(segment, &mut out);
    out
}

fn collect_leaf_tokens_inner(segment: &Segment, out: &mut Vec<TokenSegment>) {
    match segment {
        Segment::Token(t) => out.push(t.clone()),
        Segment::Node(n) => {
            for child in &n.children {
                collect_leaf_tokens_inner(child, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::lint_sql;

    #[test]
    fn test_lt11_flags_inline_union() {
        let violations = lint_sql("SELECT 1 UNION SELECT 2", RuleLT11);
        assert!(!violations.is_empty());
        assert!(violations.iter().all(|v| v.rule_code == "LT11"));
    }

    #[test]
    fn test_lt11_accepts_newlines() {
        let violations = lint_sql("SELECT 1\nUNION\nSELECT 2", RuleLT11);
        assert_eq!(violations.len(), 0);
    }
}
