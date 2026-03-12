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
    pub fn format_file(&self, path: &Path, source: &str, violations: &[LintViolation]) -> String {
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
            let msg = rigsql_i18n::rule_message(&v.message_key, &v.message_params, &v.message);

            if self.use_color {
                out.push_str(&format!(
                    "  \x1b[90mL{:>4}:{:<3}\x1b[0m | \x1b[33m{}\x1b[0m | {}\n",
                    line, col, v.rule_code, msg
                ));
            } else {
                out.push_str(&format!(
                    "  L{:>4}:{:<3} | {} | {}\n",
                    line, col, v.rule_code, msg
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
            let msg = rigsql_i18n::t("cli.all_checks_passed");
            if self.use_color {
                format!("\x1b[32m{msg}\x1b[0m")
            } else {
                msg
            }
        } else {
            let template = rigsql_i18n::t("cli.found_violations");
            let msg = template
                .replace("%{violations}", &total_violations.to_string())
                .replace("%{files_with}", &files_with_violations.to_string())
                .replace("%{total}", &total_files.to_string());
            if self.use_color {
                format!("\x1b[31m{msg}\x1b[0m")
            } else {
                msg
            }
        };

        format!("\n{}\n", status)
    }
}
