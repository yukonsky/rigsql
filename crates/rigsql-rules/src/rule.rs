use rigsql_core::{Segment, SegmentType};

use crate::violation::{LintViolation, SourceEdit};

/// Rule group / category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleGroup {
    Capitalisation,
    Layout,
    Convention,
    Aliasing,
    Ambiguous,
    References,
    Structure,
}

/// Controls which CST nodes a rule visits.
#[derive(Debug, Clone)]
pub enum CrawlType {
    /// Visit every segment of the listed types.
    Segment(Vec<SegmentType>),
    /// Visit the root segment only (whole-file rules).
    RootOnly,
}

/// Context passed to a rule during evaluation.
pub struct RuleContext<'a> {
    /// The segment being evaluated.
    pub segment: &'a Segment,
    /// The parent segment (if any).
    pub parent: Option<&'a Segment>,
    /// The root file segment.
    pub root: &'a Segment,
    /// Direct children of the parent, for sibling access.
    pub siblings: &'a [Segment],
    /// Index of `segment` within `siblings`.
    pub index_in_parent: usize,
    /// Full source text.
    pub source: &'a str,
    /// SQL dialect name (e.g. "ansi", "postgres", "tsql").
    pub dialect: &'a str,
}

/// Trait that all lint rules must implement.
pub trait Rule: Send + Sync {
    /// Rule code, e.g. "LT01".
    fn code(&self) -> &'static str;

    /// Human-readable name, e.g. "layout.spacing".
    fn name(&self) -> &'static str;

    /// One-line description.
    fn description(&self) -> &'static str;

    /// Multi-sentence explanation for AI consumers.
    fn explanation(&self) -> &'static str;

    /// Rule group.
    fn groups(&self) -> &[RuleGroup];

    /// Can this rule auto-fix violations?
    fn is_fixable(&self) -> bool;

    /// Which segments should be visited.
    fn crawl_type(&self) -> CrawlType;

    /// Evaluate the rule at the given context, returning violations.
    fn eval(&self, ctx: &RuleContext) -> Vec<LintViolation>;

    /// Configure the rule with key-value settings from config.
    /// Default implementation is a no-op.
    fn configure(&mut self, _settings: &std::collections::HashMap<String, String>) {}
}

/// Run all rules against a parsed CST.
pub fn lint(
    root: &Segment,
    source: &str,
    rules: &[Box<dyn Rule>],
    dialect: &str,
) -> Vec<LintViolation> {
    let mut violations = Vec::new();

    for rule in rules {
        match rule.crawl_type() {
            CrawlType::RootOnly => {
                let ctx = RuleContext {
                    segment: root,
                    parent: None,
                    root,
                    siblings: std::slice::from_ref(root),
                    index_in_parent: 0,
                    source,
                    dialect,
                };
                violations.extend(rule.eval(&ctx));
            }
            CrawlType::Segment(ref types) => {
                walk_and_lint_indexed(
                    root,
                    0,
                    None,
                    root,
                    source,
                    dialect,
                    rule.as_ref(),
                    types,
                    &mut violations,
                );
            }
        }
    }

    violations.sort_by_key(|v| (v.span.start, v.span.end));
    violations
}

#[allow(clippy::too_many_arguments)]
fn walk_and_lint_indexed(
    segment: &Segment,
    index_in_parent: usize,
    parent: Option<&Segment>,
    root: &Segment,
    source: &str,
    dialect: &str,
    rule: &dyn Rule,
    types: &[SegmentType],
    violations: &mut Vec<LintViolation>,
) {
    if types.contains(&segment.segment_type()) {
        let siblings = parent
            .map(|p| p.children())
            .unwrap_or(std::slice::from_ref(segment));

        let ctx = RuleContext {
            segment,
            parent,
            root,
            siblings,
            index_in_parent,
            source,
            dialect,
        };
        violations.extend(rule.eval(&ctx));
    }

    let children = segment.children();
    for (i, child) in children.iter().enumerate() {
        walk_and_lint_indexed(
            child,
            i,
            Some(segment),
            root,
            source,
            dialect,
            rule,
            types,
            violations,
        );
    }
}

/// Apply source edits to produce a fixed source string.
///
/// Edits are sorted by span start (descending) and applied back-to-front
/// so that earlier offsets remain valid. Overlapping edits are skipped.
pub fn apply_fixes(source: &str, violations: &[LintViolation]) -> String {
    // Collect all edits from all violations
    let mut edits: Vec<&SourceEdit> = violations.iter().flat_map(|v| v.fixes.iter()).collect();

    if edits.is_empty() {
        return source.to_string();
    }

    // Sort by span start descending, then by span end descending (apply from back)
    edits.sort_by(|a, b| {
        b.span
            .start
            .cmp(&a.span.start)
            .then(b.span.end.cmp(&a.span.end))
    });

    // Deduplicate edits with identical spans
    edits.dedup_by(|a, b| a.span == b.span);

    let mut result = source.to_string();
    let mut last_applied_start = u32::MAX;

    for edit in &edits {
        let start = edit.span.start as usize;
        let end = edit.span.end as usize;

        // Skip overlapping edits: any edit whose range touches the previously applied one
        if edit.span.end > last_applied_start {
            continue;
        }
        // Also skip inserts at the same offset as a previously applied edit
        if edit.span.start >= last_applied_start {
            continue;
        }

        if start <= result.len() && end <= result.len() {
            result.replace_range(start..end, &edit.new_text);
            last_applied_start = edit.span.start;
        }
    }

    result
}
