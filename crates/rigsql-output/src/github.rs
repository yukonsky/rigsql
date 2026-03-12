use std::path::Path;

use rigsql_rules::LintViolation;

/// GitHub Actions workflow command formatter.
///
/// Outputs violations as `::warning` commands that GitHub Actions
/// interprets as file annotations visible in pull request diffs.
///
/// Format: `::warning file={path},line={line},col={col}::{message}`
pub struct GithubFormatter;

impl GithubFormatter {
    pub fn format(file_results: &[(&Path, &str, &[LintViolation])]) -> String {
        let mut out = String::new();

        for (path, source, violations) in file_results {
            for v in *violations {
                let (line, col) = v.line_col(source);
                out.push_str(&format!(
                    "::warning file={},line={},col={}::{} {}\n",
                    path.display(),
                    line,
                    col,
                    v.rule_code,
                    v.message,
                ));
            }
        }

        out
    }
}
