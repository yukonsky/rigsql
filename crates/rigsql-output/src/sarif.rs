use std::path::Path;

use rigsql_rules::{LintViolation, Rule};
use serde::Serialize;
use std::collections::HashMap;

/// SARIF v2.1.0 output formatter.
///
/// Produces Static Analysis Results Interchange Format (SARIF) JSON,
/// compatible with GitHub Code Scanning, VS Code SARIF Viewer, and
/// other SARIF-consuming tools.
pub struct SarifFormatter;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifLog {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifDriver {
    name: &'static str,
    version: &'static str,
    information_uri: &'static str,
    rules: Vec<SarifRuleDescriptor>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRuleDescriptor {
    id: String,
    name: String,
    short_description: SarifMessage,
    full_description: SarifMessage,
    default_configuration: SarifRuleConfig,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRuleConfig {
    level: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult {
    rule_id: String,
    rule_index: usize,
    level: &'static str,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifLocation {
    physical_location: SarifPhysicalLocation,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifPhysicalLocation {
    artifact_location: SarifArtifactLocation,
    region: SarifRegion,
}

#[derive(Serialize)]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRegion {
    start_line: usize,
    start_column: usize,
}

impl SarifFormatter {
    pub fn format_with_rules(
        file_results: &[(&Path, &str, &[LintViolation])],
        rules: &[Box<dyn Rule>],
    ) -> String {
        // Build rule descriptors and index map
        let mut rule_indices: HashMap<&str, usize> = HashMap::new();
        let rule_descriptors: Vec<SarifRuleDescriptor> = rules
            .iter()
            .enumerate()
            .map(|(i, r)| {
                rule_indices.insert(r.code(), i);
                SarifRuleDescriptor {
                    id: r.code().to_string(),
                    name: r.name().to_string(),
                    short_description: SarifMessage {
                        text: r.description().to_string(),
                    },
                    full_description: SarifMessage {
                        text: r.explanation().to_string(),
                    },
                    default_configuration: SarifRuleConfig { level: "warning" },
                }
            })
            .collect();

        // Build results
        let results: Vec<SarifResult> = file_results
            .iter()
            .flat_map(|(path, source, violations)| {
                let indices = &rule_indices;
                violations.iter().map(move |v| {
                    let (line, col) = v.line_col(source);
                    let rule_index = indices.get(v.rule_code).copied().unwrap_or(0);

                    SarifResult {
                        rule_id: v.rule_code.to_string(),
                        rule_index,
                        level: "warning",
                        message: SarifMessage {
                            text: v.message.clone(),
                        },
                        locations: vec![SarifLocation {
                            physical_location: SarifPhysicalLocation {
                                artifact_location: SarifArtifactLocation {
                                    uri: path.display().to_string(),
                                },
                                region: SarifRegion {
                                    start_line: line,
                                    start_column: col,
                                },
                            },
                        }],
                    }
                })
            })
            .collect();

        let log = SarifLog {
            schema:
                "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
            version: "2.1.0",
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "rigsql",
                        version: env!("CARGO_PKG_VERSION"),
                        information_uri: "https://github.com/yukonsky/rigsql",
                        rules: rule_descriptors,
                    },
                },
                results,
            }],
        };

        serde_json::to_string_pretty(&log).unwrap()
    }
}
