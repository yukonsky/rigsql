use std::collections::HashMap;
use std::path::Path;

use rigsql_rules::{LintViolation, Rule};
use serde::Serialize;

/// AI-friendly JSON output formatter.
pub struct JsonFormatter;

#[derive(Serialize)]
pub struct JsonReport {
    pub version: &'static str,
    pub tool: ToolInfo,
    pub summary: Summary,
    pub files: Vec<FileReport>,
}

#[derive(Serialize)]
pub struct ToolInfo {
    pub name: &'static str,
    pub version: &'static str,
}

#[derive(Serialize)]
pub struct Summary {
    pub files_scanned: usize,
    pub files_with_violations: usize,
    pub total_violations: usize,
    pub by_rule: HashMap<String, usize>,
}

#[derive(Serialize)]
pub struct FileReport {
    pub path: String,
    pub violations: Vec<ViolationReport>,
}

#[derive(Serialize)]
pub struct ViolationReport {
    pub rule: RuleInfo,
    pub message: String,
    pub severity: String,
    pub location: Location,
    pub context: Context,
}

#[derive(Serialize)]
pub struct RuleInfo {
    pub code: String,
    pub name: String,
    pub description: String,
    pub explanation: String,
}

#[derive(Serialize)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

#[derive(Serialize)]
pub struct Context {
    pub source_line: String,
}

impl JsonFormatter {
    pub fn format(
        file_results: &[(&Path, &str, &[LintViolation])],
    ) -> String {
        Self::format_with_rules(file_results, &rigsql_rules::default_rules())
    }

    pub fn format_with_rules(
        file_results: &[(&Path, &str, &[LintViolation])],
        rules: &[Box<dyn Rule>],
    ) -> String {
        let rule_map: HashMap<&str, &dyn Rule> = rules
            .iter()
            .map(|r| (r.code(), r.as_ref()))
            .collect();

        let mut by_rule: HashMap<String, usize> = HashMap::new();
        let mut total_violations = 0;
        let mut files_with_violations = 0;

        let files: Vec<FileReport> = file_results
            .iter()
            .map(|(path, source, violations)| {
                if !violations.is_empty() {
                    files_with_violations += 1;
                }
                total_violations += violations.len();

                let violation_reports: Vec<ViolationReport> = violations
                    .iter()
                    .map(|v| {
                        *by_rule.entry(v.rule_code.to_string()).or_insert(0) += 1;
                        let (line, col) = v.line_col(source);

                        let source_line = source
                            .lines()
                            .nth(line.saturating_sub(1))
                            .unwrap_or("")
                            .to_string();

                        let rule_info = rule_map.get(v.rule_code);

                        ViolationReport {
                            rule: RuleInfo {
                                code: v.rule_code.to_string(),
                                name: rule_info.map_or("", |r| r.name()).to_string(),
                                description: rule_info.map_or("", |r| r.description()).to_string(),
                                explanation: rule_info.map_or("", |r| r.explanation()).to_string(),
                            },
                            message: v.message.clone(),
                            severity: "warning".to_string(),
                            location: Location { line, column: col },
                            context: Context { source_line },
                        }
                    })
                    .collect();

                FileReport {
                    path: path.display().to_string(),
                    violations: violation_reports,
                }
            })
            .collect();

        let report = JsonReport {
            version: "1.0",
            tool: ToolInfo {
                name: "rigsql",
                version: env!("CARGO_PKG_VERSION"),
            },
            summary: Summary {
                files_scanned: file_results.len(),
                files_with_violations,
                total_violations,
                by_rule,
            },
            files,
        };

        serde_json::to_string_pretty(&report).unwrap()
    }
}
