use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use rayon::prelude::*;
use rigsql_config::{filter_noqa, Config};
use rigsql_core::Segment;
use rigsql_dialects::DialectKind;
use rigsql_output::HumanFormatter;
use rigsql_rules::{
    default_rules,
    rule::{apply_fixes, lint},
    Rule,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser)]
#[command(name = "rigsql", version, about = "Fast SQL linter written in Rust")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse SQL files and display the Concrete Syntax Tree
    Parse {
        /// SQL file to parse (use - for stdin)
        file: String,

        /// SQL dialect [default: ansi] (overrides config file)
        #[arg(long)]
        dialect: Option<DialectArg>,

        /// Output format
        #[arg(long, default_value = "tree")]
        format: ParseFormat,
    },

    /// Lint SQL files for style violations
    Lint {
        /// SQL files or directories to lint
        files: Vec<String>,

        /// SQL dialect [default: ansi] (overrides config file)
        #[arg(long)]
        dialect: Option<DialectArg>,

        /// Output format
        #[arg(long, default_value = "human")]
        format: LintFormat,

        /// Disable colored output
        #[arg(long)]
        no_color: bool,
    },

    /// Auto-fix SQL files
    Fix {
        /// SQL files or directories to fix
        files: Vec<String>,

        /// SQL dialect [default: ansi] (overrides config file)
        #[arg(long)]
        dialect: Option<DialectArg>,

        /// Don't write changes, just show what would be fixed
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(long, short)]
        force: bool,
    },

    /// List available lint rules
    Rules,

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Clone, ValueEnum)]
enum DialectArg {
    Ansi,
    Postgres,
    Tsql,
}

impl From<DialectArg> for DialectKind {
    fn from(arg: DialectArg) -> Self {
        match arg {
            DialectArg::Ansi => DialectKind::Ansi,
            DialectArg::Postgres => DialectKind::Postgres,
            DialectArg::Tsql => DialectKind::Tsql,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum ParseFormat {
    Tree,
    Json,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum LintFormat {
    Human,
    Json,
    Sarif,
    Github,
}

/// Resolve the effective dialect: CLI flag > config file > "ansi" default.
fn resolve_dialect(cli_dialect: Option<DialectArg>, config: &Config) -> DialectKind {
    if let Some(arg) = cli_dialect {
        return arg.into();
    }
    if let Some(ref name) = config.dialect {
        match name.parse::<DialectKind>() {
            Ok(d) => return d,
            Err(_) => {
                eprintln!("Warning: unknown dialect '{name}' in config, using 'ansi'");
            }
        }
    }
    DialectKind::Ansi
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse {
            file,
            dialect,
            format,
        } => {
            let config = Config::load_for_path(Path::new(&file));
            let dialect = resolve_dialect(dialect, &config);
            cmd_parse(&file, dialect, format);
        }

        Commands::Lint {
            files,
            dialect,
            format,
            no_color,
        } => {
            let (rules, config) = build_rules(&files);
            let dialect = resolve_dialect(dialect, &config);
            let exit_code = cmd_lint(&files, dialect, format, no_color, rules);
            process::exit(exit_code);
        }

        Commands::Fix {
            files,
            dialect,
            dry_run,
            force,
        } => {
            let (rules, config) = build_rules(&files);
            let dialect = resolve_dialect(dialect, &config);
            let exit_code = cmd_fix(&files, dialect, dry_run, force, rules);
            process::exit(exit_code);
        }

        Commands::Rules => cmd_rules(),

        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "rigsql", &mut std::io::stdout());
        }
    }
}

fn cmd_parse(file: &str, dialect: DialectKind, format: ParseFormat) {
    let source = read_file_or_stdin(file);
    let parser = dialect.parser();
    let cst = parser.parse(&source).unwrap_or_else(|e| {
        eprintln!("Parse error: {e}");
        process::exit(2);
    });

    match format {
        ParseFormat::Tree => print_tree(&cst, 0),
        ParseFormat::Json => {
            let json = cst_to_json(&cst);
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
    }
}

/// Build configured rules from config file found near the given paths.
/// Returns the rules and the loaded config (so callers can use `config.dialect`).
fn build_rules(files: &[String]) -> (Vec<Box<dyn Rule>>, Config) {
    let config = files
        .first()
        .map(|f| Config::load_for_path(Path::new(f)))
        .unwrap_or_default();

    let mut rules = default_rules();

    for rule in &mut rules {
        if let Some(settings) = config.rules.get(rule.name()) {
            rule.configure(settings);
        }
    }

    if let Some(max_len) = config.max_line_length {
        for rule in &mut rules {
            if rule.code() == "LT05" {
                let mut settings = std::collections::HashMap::new();
                settings.insert("max_line_length".to_string(), max_len.to_string());
                rule.configure(&settings);
            }
        }
    }

    if !config.exclude_rules.is_empty() {
        rules.retain(|r| !config.exclude_rules.iter().any(|e| e == r.code()));
    }

    (rules, config)
}

/// Per-file lint result for aggregation after parallel processing.
struct FileLintResult {
    path: PathBuf,
    source: String,
    violations: Vec<rigsql_rules::LintViolation>,
    /// Pre-formatted output for human format (avoids post-processing).
    human_output: Option<String>,
}

fn cmd_lint(
    files: &[String],
    dialect: DialectKind,
    format: LintFormat,
    no_color: bool,
    rules: Vec<Box<dyn Rule>>,
) -> i32 {
    let sql_files = collect_sql_files(files);
    if sql_files.is_empty() {
        eprintln!("No SQL files found.");
        return 2;
    }

    let dialect_str = dialect.as_str();
    let formatter = HumanFormatter::new(!no_color);

    // Parallel lint: each file is parsed + linted independently
    let results: Vec<FileLintResult> = sql_files
        .par_iter()
        .filter_map(|path| {
            let source = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading {}: {}", path.display(), e);
                    return None;
                }
            };

            let parser = dialect.parser();
            let cst = match parser.parse(&source) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Parse error in {}: {}", path.display(), e);
                    return None;
                }
            };

            let mut violations = lint(&cst, &source, &rules, dialect_str);
            filter_noqa(&source, &mut violations);

            let human_output = if format == LintFormat::Human {
                let out = formatter.format_file(path, &source, &violations);
                Some(out)
            } else {
                None
            };

            Some(FileLintResult {
                path: path.clone(),
                source,
                violations,
                human_output,
            })
        })
        .collect();

    // Aggregate results (sequential, for deterministic output order)
    let file_count = sql_files.len();
    let mut total_violations = 0;
    let mut files_with_violations = 0;

    for r in &results {
        if !r.violations.is_empty() {
            files_with_violations += 1;
            total_violations += r.violations.len();
        }
    }

    match format {
        LintFormat::Human => {
            for r in &results {
                if let Some(out) = &r.human_output {
                    if !out.is_empty() {
                        print!("{out}");
                    }
                }
            }
            let summary =
                formatter.format_summary(file_count, files_with_violations, total_violations);
            print!("{summary}");
        }
        LintFormat::Json | LintFormat::Sarif | LintFormat::Github => {
            let refs: Vec<(&Path, &str, &[rigsql_rules::LintViolation])> = results
                .iter()
                .map(|r| (r.path.as_path(), r.source.as_str(), r.violations.as_slice()))
                .collect();
            match format {
                LintFormat::Json => {
                    println!(
                        "{}",
                        rigsql_output::JsonFormatter::format_with_rules(&refs, &rules)
                    );
                }
                LintFormat::Sarif => {
                    println!(
                        "{}",
                        rigsql_output::SarifFormatter::format_with_rules(&refs, &rules)
                    );
                }
                LintFormat::Github => {
                    print!("{}", rigsql_output::GithubFormatter::format(&refs));
                }
                LintFormat::Human => unreachable!(),
            }
        }
    }

    if total_violations > 0 {
        1
    } else {
        0
    }
}

/// Per-file fix result.
struct FileFixResult {
    path: PathBuf,
    fixed: String,
    fix_count: usize,
}

fn cmd_fix(
    files: &[String],
    dialect: DialectKind,
    dry_run: bool,
    _force: bool,
    all_rules: Vec<Box<dyn Rule>>,
) -> i32 {
    let sql_files = collect_sql_files(files);
    if sql_files.is_empty() {
        eprintln!("No SQL files found.");
        return 2;
    }
    let dialect_str = dialect.as_str();

    // Parallel fix: each file runs its own iterative fix loop
    let results: Vec<FileFixResult> = sql_files
        .par_iter()
        .filter_map(|path| {
            let source = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading {}: {}", path.display(), e);
                    return None;
                }
            };

            let parser = dialect.parser();
            let mut current = source;
            let mut total_fixed = 0;
            let max_rounds = 10;

            for _ in 0..max_rounds {
                let cst = match parser.parse(&current) {
                    Ok(c) => c,
                    Err(_) => break,
                };

                let mut violations = lint(&cst, &current, &all_rules, dialect_str);
                filter_noqa(&current, &mut violations);

                let fixable: Vec<_> = violations
                    .into_iter()
                    .filter(|v| !v.fixes.is_empty())
                    .collect();
                if fixable.is_empty() {
                    break;
                }

                let new_source = apply_fixes(&current, &fixable);
                if new_source == current {
                    break;
                }

                total_fixed += fixable.len();
                current = new_source;
            }

            if total_fixed == 0 {
                return None;
            }

            Some(FileFixResult {
                path: path.clone(),
                fixed: current,
                fix_count: total_fixed,
            })
        })
        .collect();

    let file_count = results.len();
    let mut total_fixed = 0;

    for r in &results {
        total_fixed += r.fix_count;
        if dry_run {
            println!("Would fix: {}", r.path.display());
        } else if let Err(e) = fs::write(&r.path, &r.fixed) {
            eprintln!("Error writing {}: {}", r.path.display(), e);
        } else {
            println!("Fixed: {}", r.path.display());
        }
    }

    if file_count == 0 {
        eprintln!("No fixable violations found.");
    } else {
        eprintln!(
            "{} {} in {} file{}.",
            if dry_run { "Would apply" } else { "Applied" },
            total_fixed,
            file_count,
            if file_count == 1 { "" } else { "s" },
        );
    }

    0
}

fn cmd_rules() {
    let rules = default_rules();
    println!("{:<6} {:<30} Description", "Code", "Name");
    println!("{}", "-".repeat(80));
    for rule in &rules {
        println!(
            "{:<6} {:<30} {}",
            rule.code(),
            rule.name(),
            rule.description()
        );
    }
}

fn collect_sql_files(paths: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path_str in paths {
        let path = PathBuf::from(path_str);
        if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            let walker = ignore::WalkBuilder::new(&path)
                .hidden(true) // skip hidden files
                .git_ignore(true) // respect .gitignore
                .build();
            for entry in walker.flatten() {
                let p = entry.path().to_path_buf();
                if p.is_file() && p.extension().is_some_and(|ext| ext == "sql") {
                    files.push(p);
                }
            }
        }
    }
    files.sort();
    files
}

fn read_file_or_stdin(file: &str) -> String {
    if file == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .unwrap_or_else(|e| {
                eprintln!("Error reading stdin: {e}");
                process::exit(2);
            });
        buf
    } else {
        fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading {file}: {e}");
            process::exit(2);
        })
    }
}

fn print_tree(segment: &Segment, depth: usize) {
    let indent = "  ".repeat(depth);
    match segment {
        Segment::Token(t) => {
            let text = t.token.text.replace('\n', "\\n").replace('\r', "\\r");
            println!(
                "{indent}[{:?}] {:?} {:?}",
                t.segment_type, t.token.kind, text
            );
        }
        Segment::Node(n) => {
            println!("{indent}[{:?}]", n.segment_type);
            for child in &n.children {
                print_tree(child, depth + 1);
            }
        }
    }
}

fn cst_to_json(segment: &Segment) -> serde_json::Value {
    match segment {
        Segment::Token(t) => {
            serde_json::json!({
                "type": format!("{:?}", t.segment_type),
                "token_kind": format!("{:?}", t.token.kind),
                "text": t.token.text.as_str(),
                "span": {
                    "start": t.token.span.start,
                    "end": t.token.span.end,
                }
            })
        }
        Segment::Node(n) => {
            let children: Vec<serde_json::Value> = n.children.iter().map(cst_to_json).collect();
            serde_json::json!({
                "type": format!("{:?}", n.segment_type),
                "children": children,
                "span": {
                    "start": n.span.start,
                    "end": n.span.end,
                }
            })
        }
    }
}
