# rigsql

Fast SQL linter written in Rust. sqlfluff-compatible rule codes with AI-friendly JSON output.

## Features

- Custom CST (Concrete Syntax Tree) parser — preserves all whitespace and comments
- Error recovery — produces partial CSTs even for invalid SQL
- sqlfluff-compatible rule code system (CP01, LT01, AL01, etc.)
- 75 rules across 9 categories: Capitalisation, Layout, Convention, Aliasing, Ambiguous, References, Structure, TSQL, Rigsql
- AI-friendly JSON output with rule explanations, context, and source lines
- Multi-dialect support: ANSI, PostgreSQL, SQL Server (TSQL)
- i18n support — violation messages in English and Japanese (`--locale en|ja`)
- Auto-fix for many rules (`rigsql fix`)
- Parallel file processing via rayon
- Configuration via `rigsql.toml` or `.sqlfluff` (compatible)
- Inline `-- noqa` comments to suppress violations
- Recursive directory scanning with `.gitignore` support
- Shell completions for bash, zsh, fish, elvish, powershell
- GitHub Action for CI integration
- Multiple output formats: human, JSON, SARIF, GitHub Annotations

## Installation

From crates.io:

```bash
cargo install rigsql-cli
```

From source:

```bash
cargo install --path crates/rigsql-cli
```

## Quick Start

```bash
# Lint a single file
rigsql lint query.sql

# Lint a directory recursively
rigsql lint ./queries/

# Lint with SQL Server dialect
rigsql lint ./queries/ --dialect tsql

# Lint with Japanese violation messages
rigsql lint ./queries/ --locale ja

# Auto-fix violations
rigsql fix ./queries/

# JSON output (for CI/AI tools)
rigsql lint ./queries/ --format json

# SARIF output (for IDE integration)
rigsql lint ./queries/ --format sarif

# GitHub Annotations (for CI)
rigsql lint ./queries/ --format github

# View the CST of a SQL file
rigsql parse query.sql

# Generate shell completions
rigsql completions zsh > _rigsql
```

## Usage

```
rigsql <COMMAND>

Commands:
  lint         Lint SQL files for style violations
  fix          Auto-fix SQL files
  parse        Parse SQL files and display the Concrete Syntax Tree
  rules        List available lint rules
  completions  Generate shell completions
```

### `rigsql lint`

```
rigsql lint [OPTIONS] [FILES]...

Arguments:
  [FILES]...  SQL files or directories to lint

Options:
  --dialect <DIALECT>  SQL dialect [default: ansi] [possible values: ansi, postgres, tsql]
  --locale <LOCALE>    Output locale (e.g. "en", "ja") — overrides config and system locale
  --format <FORMAT>    Output format [default: human] [possible values: human, json, sarif, github]
  --no-color           Disable colored output
```

### `rigsql fix`

```
rigsql fix [OPTIONS] [FILES]...

Arguments:
  [FILES]...  SQL files or directories to fix

Options:
  --dialect <DIALECT>  SQL dialect [default: ansi] [possible values: ansi, postgres, tsql]
  --locale <LOCALE>    Output locale (e.g. "en", "ja") — overrides config and system locale
  --dry-run            Don't write changes, just show what would be fixed
  -f, --force          Skip confirmation prompt
```

### `rigsql parse`

```
rigsql parse [OPTIONS] <FILE>

Arguments:
  <FILE>  SQL file to parse (use - for stdin)

Options:
  --dialect <DIALECT>  SQL dialect [default: ansi] [possible values: ansi, postgres, tsql]
  --format <FORMAT>    Output format [default: tree] [possible values: tree, json]
```

### `rigsql completions`

```
rigsql completions <SHELL>

Arguments:
  <SHELL>  Shell to generate completions for [possible values: bash, elvish, fish, powershell, zsh]
```

## Rules

75 rules across 9 categories, compatible with sqlfluff rule codes.

### Capitalisation (CP)

| Code | Name | Description |
|------|------|-------------|
| CP01 | capitalisation.keywords | Keywords must be consistently capitalised |
| CP02 | capitalisation.identifiers | Unquoted identifiers must be consistently capitalised |
| CP03 | capitalisation.functions | Function names must be consistently capitalised |
| CP04 | capitalisation.literals | Boolean/Null literals must be consistently capitalised |
| CP05 | capitalisation.types | Data type names must be consistently capitalised |

### Layout (LT)

| Code | Name | Description |
|------|------|-------------|
| LT01 | layout.spacing | Inappropriate spacing found |
| LT02 | layout.indent | Incorrect indentation |
| LT03 | layout.operators | Operators should be surrounded by single spaces |
| LT04 | layout.commas | Commas should be at the end of the line, not the start |
| LT05 | layout.long_lines | Line too long (default: 80 characters) |
| LT06 | layout.function_paren | Function name not followed immediately by parenthesis |
| LT07 | layout.with_spacing | WITH keyword not followed by single space |
| LT08 | layout.cte_newline | Blank line expected but not found before CTE definition |
| LT09 | layout.select_targets | Select targets should be on a new line unless there is only one |
| LT10 | layout.select_modifier | SELECT modifiers (DISTINCT, ALL) must be on same line as SELECT |
| LT11 | layout.set_operator_newline | Set operators should be surrounded by newlines |
| LT12 | layout.end_of_file | Files must end with a single trailing newline |
| LT13 | layout.start_of_file | Files must not begin with newlines or whitespace |
| LT14 | layout.keyword_newline | Keyword clauses should follow a standard for being before/after newlines |
| LT15 | layout.newlines | Too many consecutive blank lines |

### Convention (CV)

| Code | Name | Description |
|------|------|-------------|
| CV01 | convention.not_equal | Consistent not-equal operator |
| CV02 | convention.coalesce | Use COALESCE instead of IFNULL or NVL |
| CV03 | convention.trailing_comma_select | Trailing comma in SELECT clause |
| CV04 | convention.count | Use consistent syntax to count all rows |
| CV05 | convention.is_null | Comparisons with NULL should use IS or IS NOT |
| CV06 | convention.terminator | Statements must end with a semicolon |
| CV07 | convention.statement_brackets | Top-level statements should not be wrapped in brackets |
| CV08 | convention.left_join | Use LEFT JOIN instead of RIGHT JOIN |
| CV09 | convention.blocked_words | Use of blocked words |
| CV10 | convention.quoted_literals | Preferred quoting style for string literals |
| CV11 | convention.type_casting | Type casting style preference |
| CV12 | convention.implicit_join | Implicit joins should use explicit JOIN syntax |

### Aliasing (AL)

| Code | Name | Description |
|------|------|-------------|
| AL01 | aliasing.table | Implicit table aliasing is not allowed |
| AL02 | aliasing.column | Implicit column aliasing is not allowed |
| AL03 | aliasing.expression | Column expression without alias |
| AL04 | aliasing.unique_table | Table aliases should be unique within a statement |
| AL05 | aliasing.unused | Tables/CTEs should not be unused |
| AL06 | aliasing.length | Table alias length should be within configured bounds |
| AL07 | aliasing.forbid | Table aliasing should not be used |
| AL08 | aliasing.unique_column | Column aliases should be unique within each statement |
| AL09 | aliasing.self_alias | Self-aliasing of columns is redundant |

### Ambiguous (AM)

| Code | Name | Description |
|------|------|-------------|
| AM01 | ambiguous.distinct | DISTINCT used with GROUP BY |
| AM02 | ambiguous.union | UNION without DISTINCT or ALL |
| AM03 | ambiguous.order_by | Inconsistent ORDER BY direction |
| AM04 | ambiguous.column_count | SELECT * should list columns explicitly |
| AM05 | ambiguous.join | JOIN without qualifier |
| AM06 | ambiguous.column_references | Inconsistent column references in GROUP BY/ORDER BY |
| AM07 | ambiguous.set_column_count | Set operation column count mismatch |
| AM08 | ambiguous.join_condition | Implicit cross join in FROM clause |
| AM09 | ambiguous.order_by_limit | LIMIT without ORDER BY |

### References (RF)

| Code | Name | Description |
|------|------|-------------|
| RF01 | references.from | References cannot reference objects not present in FROM clause |
| RF02 | references.qualification | Columns should be qualified when multiple tables are referenced |
| RF03 | references.consistent | Column qualification should be consistent |
| RF04 | references.keywords | Keywords should not be used as identifiers |
| RF05 | references.special_chars | Identifiers should not contain special characters |
| RF06 | references.quoting | Unnecessary quoting of identifiers |

### Structure (ST)

| Code | Name | Description |
|------|------|-------------|
| ST01 | structure.else_null | Do not specify redundant ELSE NULL in a CASE expression |
| ST02 | structure.simple_case | Unnecessary CASE expression |
| ST03 | structure.unused_cte | Query defines a CTE but does not use it |
| ST04 | structure.nested_case | Nested CASE expressions should be avoided |
| ST05 | structure.subquery | Derived tables should use CTEs instead |
| ST06 | structure.column_order | Select column order convention |
| ST07 | structure.using | Prefer explicit ON clause over USING clause in joins |
| ST08 | structure.distinct | DISTINCT used with parentheses is misleading |
| ST09 | structure.join_condition_order | Join condition column order convention |
| ST10 | structure.where_constant | WHERE clause contains a constant/tautological expression |
| ST11 | structure.unused_join | Joined table is not referenced in the query |
| ST12 | structure.consecutive_semicolons | Consecutive semicolons indicate empty statements |

### TSQL (TQ)

| Code | Name | Description |
|------|------|-------------|
| TQ01 | tsql.sp_prefix | Avoid using the sp_ prefix for stored procedures |
| TQ02 | tsql.block_structure | IF/WHILE blocks should use BEGIN...END for multi-statement bodies |
| TQ03 | tsql.empty_batch | Avoid empty batches (consecutive GO statements) |

### Rigsql (RG)

| Code | Name | Description |
|------|------|-------------|
| RG02 | rigsql.union_null | Consistent use of NULL in UNION |
| RG03 | rigsql.no_between | Use of BETWEEN operator |
| RG04 | rigsql.having_without_group_by | Use of HAVING without GROUP BY |
| RG05 | rigsql.subquery_alias | Subqueries in FROM clause should have an alias |

## Configuration

rigsql looks for configuration in the following order (closest to the file wins):

1. `rigsql.toml` (preferred)
2. `.sqlfluff` (compatible fallback)

### rigsql.toml

```toml
[core]
dialect = "tsql"
max_line_length = 120
exclude_rules = ["LT09", "CV06"]

[rules."capitalisation.keywords"]
capitalisation_policy = "lower"

[rules."capitalisation.identifiers"]
capitalisation_policy = "consistent"
```

### .sqlfluff

```ini
[sqlfluff]
dialect = tsql
max_line_length = 120
exclude_rules = LT09,CV06

[sqlfluff:rules:capitalisation.keywords]
capitalisation_policy = lower
```

### Inline noqa

```sql
SELECT
    col1,  -- noqa: CP01
    col2
FROM my_table;  -- noqa
```

## Output Examples

### Human (default)

```
query.sql
  L   1:1   | CP01 | Keywords must be upper case. Found 'select' instead of 'SELECT'.
  L   2:1   | CP01 | Keywords must be upper case. Found 'from' instead of 'FROM'.
  L   2:6   | AL01 | Implicit aliasing not allowed. Use explicit AS keyword.
  L   7:13  | LT01 | Expected single space, found 2 spaces.
  L   8:1   | CV06 | Statement is not terminated with a semicolon.

Found 5 violation(s) in 1 file(s) (1 file(s) scanned).
```

### JSON (`--format json`)

```json
{
  "version": "1.0",
  "tool": {
    "name": "rigsql",
    "version": "0.6.0"
  },
  "summary": {
    "files_scanned": 1,
    "files_with_violations": 1,
    "total_violations": 5,
    "by_rule": { "CP01": 2, "AL01": 1, "LT01": 1, "CV06": 1 }
  },
  "files": [
    {
      "path": "query.sql",
      "violations": [
        {
          "rule": {
            "code": "CP01",
            "name": "capitalisation.keywords",
            "description": "Keywords must be consistently capitalised.",
            "explanation": "SQL keywords like SELECT, FROM, WHERE should use consistent capitalisation..."
          },
          "message": "Keywords must be upper case. Found 'select' instead of 'SELECT'.",
          "severity": "warning",
          "location": { "line": 1, "column": 1 },
          "context": { "source_line": "select u.id, u.Name, count(*) cnt" }
        }
      ]
    }
  ]
}
```

## GitHub Action

```yaml
- uses: yukonsky/rigsql@v0.6.0
  with:
    paths: "./queries/"
    dialect: "ansi"
    format: "github"
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | No violations found |
| 1 | Violations found |
| 2 | Error (file not found, parse error, etc.) |

## Project Structure

```
crates/
  rigsql-core/      # Token, Span, Segment types, tree traversal
  rigsql-lexer/     # Multi-dialect tokenizer
  rigsql-parser/    # Grammar combinators + CST builder
  rigsql-dialects/  # ANSI, PostgreSQL, TSQL dialect definitions
  rigsql-rules/     # All lint rules (CP, LT, CV, AL, AM, RF, ST, TQ, RG)
  rigsql-config/    # Configuration loading (.sqlfluff, rigsql.toml)
  rigsql-output/    # Human, JSON, SARIF, GitHub Annotation formatters
  rigsql-i18n/      # Internationalization (en, ja locale files)
  rigsql-cli/       # CLI binary
```

## License

MIT
