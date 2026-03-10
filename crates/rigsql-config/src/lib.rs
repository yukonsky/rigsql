use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file {path}: {source}")]
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Parsed rigsql / sqlfluff configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// SQL dialect name (e.g. "ansi", "tsql", "postgres").
    pub dialect: Option<String>,
    /// Maximum line length for LT05.
    pub max_line_length: Option<usize>,
    /// Exclude rules (comma-separated codes).
    pub exclude_rules: Vec<String>,
    /// Per-rule settings: rule_name -> key -> value.
    pub rules: HashMap<String, HashMap<String, String>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dialect: None,
            max_line_length: None,
            exclude_rules: Vec::new(),
            rules: HashMap::new(),
        }
    }
}

/// Which config file was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigKind {
    RigsqlToml,
    Sqlfluff,
}

impl Config {
    /// Load config by searching upward from the given file/directory path.
    ///
    /// Priority: `rigsql.toml` > `.sqlfluff`.
    /// At each directory level, if `rigsql.toml` exists it is used; otherwise `.sqlfluff`.
    /// Files are merged bottom-up (closest file wins).
    pub fn load_for_path(path: &Path) -> Self {
        let search_dir = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };

        let mut config_files: Vec<(PathBuf, ConfigKind)> = Vec::new();
        let mut dir = Some(search_dir);
        while let Some(d) = dir {
            if let Some(found) = find_config_in_dir(d) {
                config_files.push(found);
            }
            dir = d.parent();
        }

        // Also check home directory (if not already found via traversal)
        if let Some(home) = dirs_home() {
            if !config_files.iter().any(|(p, _)| p.parent() == Some(&home)) {
                if let Some(found) = find_config_in_dir(&home) {
                    config_files.push(found);
                }
            }
        }

        // Reverse so that furthest (most general) is first, closest (most specific) last
        config_files.reverse();

        let mut config = Config::default();
        for (path, kind) in &config_files {
            let parsed = match kind {
                ConfigKind::RigsqlToml => parse_rigsql_toml(path),
                ConfigKind::Sqlfluff => parse_sqlfluff_file(path),
            };
            if let Ok(file_config) = parsed {
                config.merge(file_config);
            }
        }

        config
    }

    /// Merge another config into this one. `other` takes precedence.
    fn merge(&mut self, other: Config) {
        if other.dialect.is_some() {
            self.dialect = other.dialect;
        }
        if other.max_line_length.is_some() {
            self.max_line_length = other.max_line_length;
        }
        if !other.exclude_rules.is_empty() {
            self.exclude_rules = other.exclude_rules;
        }
        for (rule_name, settings) in other.rules {
            let entry = self.rules.entry(rule_name).or_default();
            for (k, v) in settings {
                entry.insert(k, v);
            }
        }
    }

    /// Get a rule-specific setting by rule name (e.g. "capitalisation.keywords") and key.
    pub fn rule_setting(&self, rule_name: &str, key: &str) -> Option<&str> {
        self.rules
            .get(rule_name)
            .and_then(|m| m.get(key))
            .map(|s| s.as_str())
    }
}

/// Check for rigsql.toml or .sqlfluff in a directory (rigsql.toml takes priority).
fn find_config_in_dir(dir: &Path) -> Option<(PathBuf, ConfigKind)> {
    let toml_path = dir.join("rigsql.toml");
    if toml_path.is_file() {
        return Some((toml_path, ConfigKind::RigsqlToml));
    }
    let sqlfluff_path = dir.join(".sqlfluff");
    if sqlfluff_path.is_file() {
        return Some((sqlfluff_path, ConfigKind::Sqlfluff));
    }
    None
}

/// Read a config file's content, mapping IO errors to ConfigError.
fn read_config_file(path: &Path) -> Result<String, ConfigError> {
    fs::read_to_string(path).map_err(|e| ConfigError::ReadError {
        path: path.to_path_buf(),
        source: e,
    })
}

// ── rigsql.toml parser ──────────────────────────────────────────────────

/// Parse a `rigsql.toml` configuration file.
///
/// Expected format:
/// ```toml
/// [core]
/// dialect = "tsql"
/// max_line_length = 120
/// exclude_rules = ["LT09", "CV06"]
///
/// [rules."capitalisation.keywords"]
/// capitalisation_policy = "lower"
/// ```
fn parse_rigsql_toml(path: &Path) -> Result<Config, ConfigError> {
    let content = read_config_file(path)?;

    let table: toml::Table = match content.parse() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Warning: failed to parse {}: {e}", path.display());
            return Ok(Config::default());
        }
    };

    let mut config = Config::default();

    // [core] section
    if let Some(core) = table.get("core").and_then(|v| v.as_table()) {
        if let Some(dialect) = core.get("dialect").and_then(|v| v.as_str()) {
            config.dialect = Some(dialect.to_string());
        }
        if let Some(len) = core.get("max_line_length").and_then(|v| v.as_integer()) {
            config.max_line_length = Some(len as usize);
        }
        if let Some(arr) = core.get("exclude_rules").and_then(|v| v.as_array()) {
            config.exclude_rules = arr
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
        }
    }

    // [rules.*] sections
    if let Some(rules) = table.get("rules").and_then(|v| v.as_table()) {
        for (rule_name, rule_value) in rules {
            if let Some(rule_table) = rule_value.as_table() {
                let mut settings = HashMap::new();
                for (k, v) in rule_table {
                    let val = match v {
                        toml::Value::String(s) => s.clone(),
                        toml::Value::Integer(i) => i.to_string(),
                        toml::Value::Float(f) => f.to_string(),
                        toml::Value::Boolean(b) => b.to_string(),
                        _ => continue,
                    };
                    settings.insert(k.clone(), val);
                }
                if !settings.is_empty() {
                    config.rules.insert(rule_name.clone(), settings);
                }
            }
        }
    }

    Ok(config)
}

// ── .sqlfluff INI parser ────────────────────────────────────────────────

/// Parse a .sqlfluff INI-style config file.
fn parse_sqlfluff_file(path: &Path) -> Result<Config, ConfigError> {
    let content = read_config_file(path)?;

    let mut config = Config::default();
    let mut current_section = String::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        // Section header
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].trim().to_string();
            continue;
        }

        // Key = value
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_lowercase();
            let value = value.trim().to_string();

            match current_section.as_str() {
                "sqlfluff" => match key.as_str() {
                    "dialect" => config.dialect = Some(value),
                    "max_line_length" => {
                        config.max_line_length = value.parse().ok();
                    }
                    "exclude_rules" => {
                        config.exclude_rules = value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    _ => {}
                },
                section if section.starts_with("sqlfluff:rules:") => {
                    let rule_name = section.strip_prefix("sqlfluff:rules:").unwrap();
                    config
                        .rules
                        .entry(rule_name.to_string())
                        .or_default()
                        .insert(key, value);
                }
                _ => {}
            }
        }
    }

    Ok(config)
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Filter out violations on lines that have `-- noqa` comments.
pub fn filter_noqa(source: &str, violations: &mut Vec<rigsql_rules::LintViolation>) {
    if violations.is_empty() {
        return;
    }

    // Build a map of line_number -> noqa spec
    let noqa_lines: HashMap<usize, NoqaSpec> = source
        .lines()
        .enumerate()
        .filter_map(|(i, line)| parse_noqa_comment(line).map(|spec| (i + 1, spec)))
        .collect();

    if noqa_lines.is_empty() {
        return;
    }

    violations.retain(|v| {
        let (line, _) = v.line_col(source);
        match noqa_lines.get(&line) {
            None => true,
            Some(NoqaSpec::All) => false,
            Some(NoqaSpec::Rules(codes)) => !codes.iter().any(|c| c == v.rule_code),
        }
    });
}

#[derive(Debug)]
enum NoqaSpec {
    /// `-- noqa` — suppress all rules on this line.
    All,
    /// `-- noqa: CP01,LT01` — suppress specific rules.
    Rules(Vec<String>),
}

/// Parse a noqa comment from a source line.
fn parse_noqa_comment(line: &str) -> Option<NoqaSpec> {
    // Case-insensitive search without allocating a new string
    let bytes = line.as_bytes();
    let pattern = b"-- noqa";
    let idx = bytes
        .windows(pattern.len())
        .position(|w| w.eq_ignore_ascii_case(pattern))?;
    let after = line[idx + 7..].trim_start();

    if after.is_empty() || after.starts_with("--") {
        return Some(NoqaSpec::All);
    }

    if let Some(rest) = after.strip_prefix(':') {
        let codes: Vec<String> = rest
            .split(',')
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();
        if codes.is_empty() {
            Some(NoqaSpec::All)
        } else {
            Some(NoqaSpec::Rules(codes))
        }
    } else {
        Some(NoqaSpec::All)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_noqa_all() {
        assert!(matches!(
            parse_noqa_comment("SELECT 1 -- noqa"),
            Some(NoqaSpec::All)
        ));
    }

    #[test]
    fn test_parse_noqa_specific() {
        match parse_noqa_comment("SELECT 1 -- noqa: CP01, LT01") {
            Some(NoqaSpec::Rules(codes)) => {
                assert_eq!(codes, vec!["CP01", "LT01"]);
            }
            _ => panic!("Expected NoqaSpec::Rules"),
        }
    }

    #[test]
    fn test_parse_noqa_none() {
        assert!(parse_noqa_comment("SELECT 1").is_none());
    }

    #[test]
    fn test_parse_sqlfluff_config() {
        let content = "\
[sqlfluff]
dialect = tsql
max_line_length = 120

[sqlfluff:rules:capitalisation.keywords]
capitalisation_policy = lower
";
        let dir = std::env::temp_dir().join("rigsql_test_sqlfluff_config");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(".sqlfluff");
        fs::write(&path, content).unwrap();

        let config = parse_sqlfluff_file(&path).unwrap();
        assert_eq!(config.dialect.as_deref(), Some("tsql"));
        assert_eq!(config.max_line_length, Some(120));
        assert_eq!(
            config.rule_setting("capitalisation.keywords", "capitalisation_policy"),
            Some("lower")
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_rigsql_toml() {
        let content = r#"
[core]
dialect = "tsql"
max_line_length = 120
exclude_rules = ["LT09", "CV06"]

[rules."capitalisation.keywords"]
capitalisation_policy = "lower"
"#;
        let dir = std::env::temp_dir().join("rigsql_test_toml_config");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("rigsql.toml");
        fs::write(&path, content).unwrap();

        let config = parse_rigsql_toml(&path).unwrap();
        assert_eq!(config.dialect.as_deref(), Some("tsql"));
        assert_eq!(config.max_line_length, Some(120));
        assert_eq!(config.exclude_rules, vec!["LT09", "CV06"]);
        assert_eq!(
            config.rule_setting("capitalisation.keywords", "capitalisation_policy"),
            Some("lower")
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_rigsql_toml_priority_over_sqlfluff() {
        let dir = std::env::temp_dir().join("rigsql_test_priority");
        let _ = fs::create_dir_all(&dir);

        // Write both config files
        fs::write(
            dir.join(".sqlfluff"),
            "[sqlfluff]\ndialect = postgres\nmax_line_length = 80\n",
        )
        .unwrap();
        fs::write(
            dir.join("rigsql.toml"),
            "[core]\ndialect = \"tsql\"\nmax_line_length = 120\n",
        )
        .unwrap();

        let config = Config::load_for_path(&dir);
        // rigsql.toml should win
        assert_eq!(config.dialect.as_deref(), Some("tsql"));
        assert_eq!(config.max_line_length, Some(120));

        let _ = fs::remove_dir_all(&dir);
    }
}
