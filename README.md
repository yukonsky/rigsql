# Rigid SQL (rigsql)

Fast SQL linter written in Rust. sqlfluff-compatible rule codes with AI-friendly JSON output.

## Features

- Custom CST (Concrete Syntax Tree) parser — preserves all whitespace and comments
- sqlfluff-compatible rule code system (CP01, LT01, AL01, etc.)
- AI-friendly JSON output with rule explanations, context, and source lines
- Multi-dialect support: ANSI, PostgreSQL, SQL Server (TSQL)
- Recursive directory scanning for `.sql` files
- Full Unicode support (Japanese identifiers, comments, etc.)

## Installation

```bash
cargo install --path crates/rigsql-cli
```

Or build from source:

```bash
cargo build --release
# Binary: ./target/release/rigsql
```

## Quick Start

```bash
# Lint a single file
rigsql lint query.sql

# Lint a directory recursively
rigsql lint ./queries/

# Lint with SQL Server dialect
rigsql lint ./queries/ --dialect tsql

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

| Code | Name | Description |
|------|------|-------------|
| CP01 | capitalisation.keywords | Keywords must be consistently capitalised |
| CP02 | capitalisation.identifiers | Unquoted identifiers must be consistently capitalised |
| CP03 | capitalisation.functions | Function names must be consistently capitalised |
| CP04 | capitalisation.literals | Boolean/Null literals must be consistently capitalised |
| LT01 | layout.spacing | Inappropriate spacing found |
| LT05 | layout.long_lines | Line too long (default: 80 characters) |
| LT12 | layout.end_of_file | Files must end with a single trailing newline |
| LT13 | layout.start_of_file | Files must not begin with newlines or whitespace |
| CV06 | convention.terminator | Statements must end with a semicolon |
| AL01 | aliasing.table | Implicit aliasing of table/column is not allowed |

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
  rigsql-config/    # Configuration loading (planned)
  rigsql-output/    # Human + JSON formatters
  rigsql-cli/       # CLI binary
```

## License

MIT
