use crate::rule::{lint, Rule};
use crate::violation::LintViolation;
use rigsql_lexer::LexerConfig;
use rigsql_parser::{AnsiGrammar, Parser};

pub fn parse(sql: &str) -> rigsql_core::Segment {
    Parser::new(LexerConfig::ansi(), Box::new(AnsiGrammar))
        .parse(sql)
        .unwrap()
}

pub fn lint_sql(sql: &str, rule: impl Rule + 'static) -> Vec<LintViolation> {
    let cst = parse(sql);
    lint(&cst, sql, &[Box::new(rule)], "ansi")
}

pub fn lint_sql_with_dialect(
    sql: &str,
    rule: impl Rule + 'static,
    dialect: &str,
) -> Vec<LintViolation> {
    let cst = parse(sql);
    lint(&cst, sql, &[Box::new(rule)], dialect)
}
