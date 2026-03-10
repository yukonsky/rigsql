use std::path::Path;

use rigsql_rules::LintViolation;

/// Human-readable terminal output formatter.
pub struct HumanFormatter {
    use_color: bool,
}

impl HumanFormatter {
    pub fn new(use_color: bool) -> Self {
        Self { use_color }
    }

    /// Format violations for a single file.
    pub fn format_file(
        &self,
        path: &Path,
        source: &str,
        violations: &[LintViolation],
    ) -> String {
        if violations.is_empty() {
            return String::new();
        }

        let mut out = String::new();

        // File header
        if self.use_color {
            out.push_str(&format!("\x1b[1m{}\x1b[0m\n", path.display()));
        } else {
            out.push_str(&format!("{}\n", path.display()));
        }

        for v in violations {
            let (line, col) = v.line_col(source);

            if self.use_color {
                out.push_str(&format!(
                    "  \x1b[90mL{:>4}:{:<3}\x1b[0m | \x1b[33m{}\x1b[0m | {}\n",
                    line, col, v.rule_code, v.message
                ));
            } else {
                out.push_str(&format!(
                    "  L{:>4}:{:<3} | {} | {}\n",
                    line, col, v.rule_code, v.message
                ));
            }
        }

        out
    }

    /// Format a summary line.
    pub fn format_summary(
        &self,
        total_files: usize,
        files_with_violations: usize,
        total_violations: usize,
    ) -> String {
        let status = if total_violations == 0 {
            if self.use_color {
                "\x1b[32mAll checks passed!\x1b[0m".to_string()
            } else {
                "All checks passed!".to_string()
            }
        } else {
            let msg = format!(
                "Found {} violation(s) in {} file(s) ({} file(s) scanned).",
                total_violations, files_with_violations, total_files
            );
            if self.use_color {
                format!("\x1b[31m{}\x1b[0m", msg)
            } else {
                msg
            }
        };

        format!("\n{}\n", status)
    }
}
