# rigsql

Fast SQL linter written in Rust. sqlfluff-compatible rule codes with AI-friendly JSON output.

## Features

- Custom CST (Concrete Syntax Tree) parser — preserves all whitespace and comments
- sqlfluff-compatible rule code system (CP01, LT01, AL01, etc.)
- AI-friendly JSON output with rule explanations, context, and source lines
- Multi-dialect support: ANSI, PostgreSQL, SQL Server (TSQL)
- Auto-fix for many rules (`rigsql fix`)
- Configuration via `rigsql.toml` or `.sqlfluff` (compatible)
- Recursive directory scanning with `.gitignore` support
- Full Unicode support (Japanese identifiers, comments, etc.)

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

# Auto-fix violations
rigsql fix ./queries/

# JSON output (for CI/AI tools)
rigsql lint ./queries/ --format json

# View the CST of a SQL file
rigsql parse query.sql
```

## Usage

```
rigsql <COMMAND>

Commands:
  lint    Lint SQL files for style violations
  fix     Auto-fix SQL files
  parse   Parse SQL files and display the Concrete Syntax Tree
  rules   List available lint rules
```

### `rigsql lint`

```
rigsql lint [OPTIONS] [FILES]...

Arguments:
  [FILES]...  SQL files or directories to lint

Options:
  --dialect <DIALECT>  SQL dialect [default: ansi] [possible values: ansi, postgres, tsql]
  --format <FORMAT>    Output format [default: human] [possible values: human, json]
  --no-color           Disable colored output
```

### `rigsql fix`

```
rigsql fix [OPTIONS] [FILES]...

Arguments:
  [FILES]...  SQL files or directories to fix

Options:
  --dialect <DIALECT>  SQL dialect [default: ansi] [possible values: ansi, postgres, tsql]
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

## Rules

38 rules across 4 categories, compatible with sqlfluff rule codes.

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
| LT09 | layout.select_targets | Select targets should be on a new line unless there is only one |
| LT10 | layout.select_modifier | SELECT modifiers (DISTINCT, ALL) must be on same line as SELECT |
| LT11 | layout.set_operator_newline | Set operators should be surrounded by newlines |
| LT12 | layout.end_of_file | Files must end with a single trailing newline |
| LT13 | layout.start_of_file | Files must not begin with newlines or whitespace |
| LT14 | layout.semicolons | Statements should not end with multiple semicolons |
| LT15 | layout.distinct | DISTINCT used with parentheses |

### Convention (CV)

| Code | Name | Description |
|------|------|-------------|
| CV01 | convention.not_equal | Consistent not-equal operator |
| CV02 | convention.coalesce | Use COALESCE instead of IFNULL or NVL |
| CV03 | convention.trailing_comma_select | Trailing comma in SELECT clause |
| CV04 | convention.count | Use consistent syntax to count all rows |
| CV05 | convention.is_null | Comparisons with NULL should use IS or IS NOT |
| CV06 | convention.terminator | Statements must end with a semicolon |
| CV07 | convention.prefer_coalesce | Prefer COALESCE over CASE with IS NULL pattern |
| CV08 | convention.left_join | Use LEFT JOIN instead of RIGHT JOIN |
| CV09 | convention.blocked_words | Use of blocked words |
| CV10 | convention.union_null | Consistent use of NULL in UNION |
| CV11 | convention.no_between | Use of BETWEEN operator |
| CV12 | convention.having_without_group_by | Use of HAVING without GROUP BY |

### Aliasing (AL)

| Code | Name | Description |
|------|------|-------------|
| AL01 | aliasing.table | Implicit table aliasing is not allowed |
| AL02 | aliasing.column | Implicit column aliasing is not allowed |
| AL03 | aliasing.expression | Column expression without alias |
| AL04 | aliasing.unique_table | Table aliases should be unique within a statement |
| AL05 | aliasing.unused | Tables/CTEs should not be unused |
| AL07 | aliasing.table_naming | Table aliases should follow a naming convention |

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
    "version": "0.1.0"
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
  rigsql-rules/     # All lint rules (CP, LT, CV, AL)
  rigsql-config/    # Configuration loading (.sqlfluff, rigsql.toml)
  rigsql-output/    # Human + JSON formatters
  rigsql-cli/       # CLI binary
```

## License

MIT
