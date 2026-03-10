use rigsql_core::Span;

/// Severity of a lint violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A source-level edit that can be applied to fix a violation.
#[derive(Debug, Clone)]
pub struct SourceEdit {
    /// Span to replace (use empty span for pure insert, non-empty for replace/delete).
    pub span: Span,
    /// Replacement text (empty string for deletion).
    pub new_text: String,
}

impl SourceEdit {
    /// Replace the text at `span` with `new_text`.
    pub fn replace(span: Span, new_text: impl Into<String>) -> Self {
        Self {
            span,
            new_text: new_text.into(),
        }
    }

    /// Insert `text` before byte offset `offset`.
    pub fn insert(offset: u32, text: impl Into<String>) -> Self {
        Self {
            span: Span::new(offset, offset),
            new_text: text.into(),
        }
    }

    /// Delete the text covered by `span`.
    pub fn delete(span: Span) -> Self {
        Self {
            span,
            new_text: String::new(),
        }
    }
}

/// A single lint violation found by a rule.
#[derive(Debug, Clone)]
pub struct LintViolation {
    /// Rule code, e.g. "CP01".
    pub rule_code: &'static str,
    /// Human-readable message describing the violation.
    pub message: String,
    /// Location in source.
    pub span: Span,
    /// Severity level.
    pub severity: Severity,
    /// Suggested fixes (empty if not auto-fixable).
    pub fixes: Vec<SourceEdit>,
}

impl LintViolation {
    pub fn new(rule_code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            rule_code,
            message: message.into(),
            span,
            severity: Severity::Warning,
            fixes: Vec::new(),
        }
    }

    /// Create a violation with a suggested fix.
    pub fn with_fix(
        rule_code: &'static str,
        message: impl Into<String>,
        span: Span,
        fixes: Vec<SourceEdit>,
    ) -> Self {
        Self {
            rule_code,
            message: message.into(),
            span,
            severity: Severity::Warning,
            fixes,
        }
    }

    /// Compute 1-based line and column from source text.
    pub fn line_col(&self, source: &str) -> (usize, usize) {
        let offset = (self.span.start as usize).min(source.len());
        // Ensure we're at a char boundary
        let offset = if source.is_char_boundary(offset) {
            offset
        } else {
            // Walk backwards to find a valid char boundary
            (0..offset)
                .rev()
                .find(|&i| source.is_char_boundary(i))
                .unwrap_or(0)
        };
        let before = &source[..offset];
        let line = before.chars().filter(|&c| c == '\n').count() + 1;
        let col = before.rfind('\n').map_or(offset, |pos| offset - pos - 1) + 1;
        (line, col)
    }
}
