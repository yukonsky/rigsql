use rigsql_core::{NodeSegment, Segment, SegmentType, TokenKind};

use crate::context::ParseContext;

use super::{eat_trivia_segments, parse_statement_list, token_segment, Grammar};

/// TSQL grammar — extends ANSI with SQL Server–specific statements.
pub struct TsqlGrammar;

const TSQL_STATEMENT_KEYWORDS: &[&str] = &[
    "ALTER",
    "BEGIN",
    "BREAK",
    "CLOSE",
    "CONTINUE",
    "CREATE",
    "DEALLOCATE",
    "DECLARE",
    "DELETE",
    "DROP",
    "ELSE",
    "END",
    "EXEC",
    "EXECUTE",
    "FETCH",
    "GO",
    "IF",
    "INSERT",
    "MERGE",
    "OPEN",
    "PRINT",
    "RAISERROR",
    "RETURN",
    "SELECT",
    "SET",
    "THROW",
    "TRUNCATE",
    "UPDATE",
    "USE",
    "WHILE",
    "WITH",
];

impl Grammar for TsqlGrammar {
    fn statement_keywords(&self) -> &[&str] {
        TSQL_STATEMENT_KEYWORDS
    }

    fn dispatch_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        // TSQL-specific statements
        if ctx.peek_keyword("DECLARE") {
            self.parse_declare_statement(ctx)
        } else if ctx.peek_keyword("SET") {
            self.parse_set_variable_statement(ctx)
        } else if ctx.peek_keyword("IF") {
            self.parse_if_statement(ctx)
        } else if ctx.peek_keyword("BEGIN") {
            self.parse_begin_block(ctx)
        } else if ctx.peek_keyword("WHILE") {
            self.parse_while_statement(ctx)
        } else if ctx.peek_keyword("EXEC") || ctx.peek_keyword("EXECUTE") {
            self.parse_exec_statement(ctx)
        } else if ctx.peek_keyword("RETURN") {
            self.parse_return_statement(ctx)
        } else if ctx.peek_keyword("PRINT") {
            self.parse_print_statement(ctx)
        } else if ctx.peek_keyword("THROW") {
            self.parse_throw_statement(ctx)
        } else if ctx.peek_keyword("RAISERROR") {
            self.parse_raiserror_statement(ctx)
        } else if ctx.peek_keyword("GO") {
            self.parse_go_statement(ctx)
        } else {
            // Fall back to ANSI dispatch
            self.dispatch_ansi_statement(ctx)
        }
    }
}

// ── TSQL-specific parsing methods ────────────────────────────────

impl TsqlGrammar {
    /// Parse DECLARE statement: `DECLARE @var TYPE [= expr] [, @var2 TYPE [= expr]]`
    fn parse_declare_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("DECLARE")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // First variable declaration
        self.parse_declare_variable(ctx, &mut children);

        // Additional comma-separated declarations
        loop {
            let save = ctx.save();
            let trivia = eat_trivia_segments(ctx);
            if let Some(comma) = ctx.eat_kind(TokenKind::Comma) {
                children.extend(trivia);
                children.push(token_segment(comma, SegmentType::Comma));
                children.extend(eat_trivia_segments(ctx));
                self.parse_declare_variable(ctx, &mut children);
            } else {
                ctx.restore(save);
                break;
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::DeclareStatement,
            children,
        )))
    }

    /// Parse a single variable declaration: `@var TYPE [= expr]` or `@var AS TYPE`
    fn parse_declare_variable(&self, ctx: &mut ParseContext, children: &mut Vec<Segment>) {
        // @variable name or cursor_name
        if ctx.peek_kind() == Some(TokenKind::AtSign) {
            let at = ctx.advance().unwrap();
            children.push(token_segment(at, SegmentType::Identifier));
            children.extend(eat_trivia_segments(ctx));
        } else if ctx.peek_kind() == Some(TokenKind::Word) {
            // Non-@ cursor name: DECLARE cursor_name CURSOR FOR ...
            let save = ctx.save();
            let name = ctx.advance().unwrap();
            let trivia = eat_trivia_segments(ctx);
            if ctx.peek_keyword("CURSOR") {
                children.push(token_segment(name, SegmentType::Identifier));
                children.extend(trivia);
            } else {
                ctx.restore(save);
            }
        }

        // Optional AS keyword
        if ctx.peek_keyword("AS") {
            let as_kw = ctx.advance().unwrap();
            children.push(token_segment(as_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
        }

        // CURSOR declaration: DECLARE @cur CURSOR [LOCAL|GLOBAL] [FORWARD_ONLY|SCROLL]
        //   [STATIC|KEYSET|DYNAMIC|FAST_FORWARD] [READ_ONLY|SCROLL_LOCKS|OPTIMISTIC]
        //   FOR select_statement
        if ctx.peek_keyword("CURSOR") {
            let cursor_kw = ctx.advance().unwrap();
            children.push(token_segment(cursor_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));

            // Consume cursor options until FOR
            while !ctx.at_eof() && !ctx.peek_keyword("FOR") {
                if ctx.peek_kind() == Some(TokenKind::Semicolon) {
                    break;
                }
                if ctx.peek_kind() == Some(TokenKind::Word) {
                    let opt = ctx.advance().unwrap();
                    children.push(token_segment(opt, SegmentType::Keyword));
                    children.extend(eat_trivia_segments(ctx));
                } else {
                    break;
                }
            }

            // FOR select_statement
            if ctx.peek_keyword("FOR") {
                let for_kw = ctx.advance().unwrap();
                children.push(token_segment(for_kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
                if let Some(sel) = self.parse_select_statement(ctx) {
                    children.push(sel);
                }
            }
            return;
        }

        // TABLE variable: DECLARE @t TABLE (...)
        if ctx.peek_keyword("TABLE") {
            let table_kw = ctx.advance().unwrap();
            children.push(token_segment(table_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
            if ctx.peek_kind() == Some(TokenKind::LParen) {
                if let Some(defs) = self.parse_paren_block(ctx) {
                    children.push(defs);
                }
            }
            return;
        }

        // Data type
        if let Some(dt) = self.parse_data_type(ctx) {
            children.push(dt);
            children.extend(eat_trivia_segments(ctx));
        }

        // Optional = expr (default value)
        if ctx.peek_kind() == Some(TokenKind::Eq) {
            let eq = ctx.advance().unwrap();
            children.push(token_segment(eq, SegmentType::ComparisonOperator));
            children.extend(eat_trivia_segments(ctx));
            if let Some(expr) = self.parse_expression(ctx) {
                children.push(expr);
            }
        }
    }

    /// Parse SET @var = expr or SET option ON/OFF
    fn parse_set_variable_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let save = ctx.save();
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("SET")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // @variable or @@variable → SET @var = expr
        if ctx.peek_kind() == Some(TokenKind::AtSign) {
            let at = ctx.advance().unwrap();
            children.push(token_segment(at, SegmentType::Identifier));
            children.extend(eat_trivia_segments(ctx));

            // = or += or -= etc.
            if let Some(kind) = ctx.peek_kind() {
                if matches!(
                    kind,
                    TokenKind::Eq
                        | TokenKind::Plus
                        | TokenKind::Minus
                        | TokenKind::Star
                        | TokenKind::Slash
                ) {
                    let op = ctx.advance().unwrap();
                    children.push(token_segment(op, SegmentType::Operator));
                    // Handle compound assignment: +=, -=, etc.
                    if ctx.peek_kind() == Some(TokenKind::Eq) {
                        let eq = ctx.advance().unwrap();
                        children.push(token_segment(eq, SegmentType::Operator));
                    }
                    children.extend(eat_trivia_segments(ctx));
                    if let Some(expr) = self.parse_expression(ctx) {
                        children.push(expr);
                    }
                }
            }

            return Some(Segment::Node(NodeSegment::new(
                SegmentType::SetVariableStatement,
                children,
            )));
        }

        // SET OPTION ON/OFF (e.g., SET ANSI_NULLS ON, SET NOCOUNT ON)
        if ctx.peek_kind() == Some(TokenKind::Word) {
            self.consume_until_statement_end(ctx, &mut children);
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::SetVariableStatement,
                children,
            )));
        }

        ctx.restore(save);
        None
    }

    /// Parse IF condition statement [ELSE statement]
    fn parse_if_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("IF")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Condition expression
        if let Some(cond) = self.parse_expression(ctx) {
            children.push(cond);
        }
        children.extend(eat_trivia_segments(ctx));

        // Then-branch: a single statement or BEGIN...END block
        if let Some(stmt) = self.parse_statement(ctx) {
            children.push(stmt);
        }

        // ELSE branch (optional)
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("ELSE") {
            let else_kw = ctx.advance().unwrap();
            children.push(token_segment(else_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));

            if let Some(stmt) = self.parse_statement(ctx) {
                children.push(stmt);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::IfStatement,
            children,
        )))
    }

    /// Parse BEGIN...END block or BEGIN TRY...END TRY BEGIN CATCH...END CATCH
    fn parse_begin_block(&self, ctx: &mut ParseContext) -> Option<Segment> {
        // Check for BEGIN TRY / BEGIN CATCH
        if ctx.peek_keywords(&["BEGIN", "TRY"]) {
            return self.parse_try_catch_block(ctx);
        }

        let mut children = Vec::new();
        let begin_kw = ctx.eat_keyword("BEGIN")?;
        children.push(token_segment(begin_kw, SegmentType::Keyword));

        parse_statement_list(self, ctx, &mut children, |c| c.peek_keyword("END"));

        // END
        children.extend(eat_trivia_segments(ctx));
        if let Some(end_kw) = ctx.eat_keyword("END") {
            children.push(token_segment(end_kw, SegmentType::Keyword));
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::BeginEndBlock,
            children,
        )))
    }

    /// Parse BEGIN TRY...END TRY BEGIN CATCH...END CATCH
    fn parse_try_catch_block(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();

        // BEGIN TRY
        let begin_kw = ctx.eat_keyword("BEGIN")?;
        children.push(token_segment(begin_kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));
        let try_kw = ctx.eat_keyword("TRY")?;
        children.push(token_segment(try_kw, SegmentType::Keyword));

        parse_statement_list(self, ctx, &mut children, |c| {
            c.peek_keywords(&["END", "TRY"])
        });

        // END TRY
        children.extend(eat_trivia_segments(ctx));
        if let Some(end_kw) = ctx.eat_keyword("END") {
            children.push(token_segment(end_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
        }
        if let Some(try_kw) = ctx.eat_keyword("TRY") {
            children.push(token_segment(try_kw, SegmentType::Keyword));
        }

        // BEGIN CATCH
        children.extend(eat_trivia_segments(ctx));
        if let Some(begin_kw) = ctx.eat_keyword("BEGIN") {
            children.push(token_segment(begin_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
            if let Some(catch_kw) = ctx.eat_keyword("CATCH") {
                children.push(token_segment(catch_kw, SegmentType::Keyword));
            }

            parse_statement_list(self, ctx, &mut children, |c| {
                c.peek_keywords(&["END", "CATCH"])
            });

            // END CATCH
            children.extend(eat_trivia_segments(ctx));
            if let Some(end_kw) = ctx.eat_keyword("END") {
                children.push(token_segment(end_kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
            }
            if let Some(catch_kw) = ctx.eat_keyword("CATCH") {
                children.push(token_segment(catch_kw, SegmentType::Keyword));
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::TryCatchBlock,
            children,
        )))
    }

    /// Parse WHILE condition statement
    fn parse_while_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("WHILE")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Condition
        if let Some(cond) = self.parse_expression(ctx) {
            children.push(cond);
        }
        children.extend(eat_trivia_segments(ctx));

        // Body: usually BEGIN...END
        if let Some(stmt) = self.parse_statement(ctx) {
            children.push(stmt);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::WhileStatement,
            children,
        )))
    }

    /// Parse EXEC/EXECUTE proc_name [params]
    fn parse_exec_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        // EXEC or EXECUTE
        let kw = if ctx.peek_keyword("EXEC") {
            ctx.eat_keyword("EXEC")
        } else {
            ctx.eat_keyword("EXECUTE")
        };
        let kw = kw?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Optional @retval =
        let save = ctx.save();
        if ctx.peek_kind() == Some(TokenKind::AtSign) {
            let at = ctx.advance().unwrap();
            let trivia = eat_trivia_segments(ctx);
            if ctx.peek_kind() == Some(TokenKind::Eq) {
                children.push(token_segment(at, SegmentType::Identifier));
                children.extend(trivia);
                let eq = ctx.advance().unwrap();
                children.push(token_segment(eq, SegmentType::Operator));
                children.extend(eat_trivia_segments(ctx));
            } else {
                ctx.restore(save);
            }
        }

        // Procedure name (possibly qualified)
        if let Some(name) = self.parse_qualified_name(ctx) {
            children.push(name);
        }
        children.extend(eat_trivia_segments(ctx));

        // Parameters: comma-separated expressions / @param = expr
        self.parse_exec_params(ctx, &mut children);

        Some(Segment::Node(NodeSegment::new(
            SegmentType::ExecStatement,
            children,
        )))
    }

    /// Parse EXEC parameters: comma-separated, optionally @param = expr
    fn parse_exec_params(&self, ctx: &mut ParseContext, children: &mut Vec<Segment>) {
        // Parse first param if present
        if ctx.at_eof()
            || ctx.peek_kind() == Some(TokenKind::Semicolon)
            || self.peek_statement_start(ctx)
        {
            return;
        }

        if let Some(expr) = self.parse_expression(ctx) {
            children.push(expr);
        }

        loop {
            let save = ctx.save();
            let trivia = eat_trivia_segments(ctx);
            if let Some(comma) = ctx.eat_kind(TokenKind::Comma) {
                children.extend(trivia);
                children.push(token_segment(comma, SegmentType::Comma));
                children.extend(eat_trivia_segments(ctx));
                if let Some(expr) = self.parse_expression(ctx) {
                    children.push(expr);
                }
            } else {
                ctx.restore(save);
                break;
            }
        }
    }

    /// Parse RETURN [expr]
    fn parse_return_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("RETURN")?;
        children.push(token_segment(kw, SegmentType::Keyword));

        // Optional return value
        let save = ctx.save();
        let trivia = eat_trivia_segments(ctx);
        if !ctx.at_eof()
            && ctx.peek_kind() != Some(TokenKind::Semicolon)
            && !self.peek_statement_start(ctx)
        {
            children.extend(trivia);
            if let Some(expr) = self.parse_expression(ctx) {
                children.push(expr);
            }
        } else {
            ctx.restore(save);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::ReturnStatement,
            children,
        )))
    }

    /// Parse PRINT expr
    fn parse_print_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("PRINT")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        if let Some(expr) = self.parse_expression(ctx) {
            children.push(expr);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::PrintStatement,
            children,
        )))
    }

    /// Parse THROW [number, message, state]
    fn parse_throw_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("THROW")?;
        children.push(token_segment(kw, SegmentType::Keyword));

        // THROW with no arguments (re-throw in CATCH block)
        let save = ctx.save();
        let trivia = eat_trivia_segments(ctx);
        if ctx.at_eof()
            || ctx.peek_kind() == Some(TokenKind::Semicolon)
            || self.peek_statement_start(ctx)
        {
            ctx.restore(save);
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::ThrowStatement,
                children,
            )));
        }

        // THROW error_number, message, state
        children.extend(trivia);
        if let Some(expr) = self.parse_expression(ctx) {
            children.push(expr);
        }
        // Consume remaining comma-separated args
        for _ in 0..2 {
            let save2 = ctx.save();
            let trivia2 = eat_trivia_segments(ctx);
            if let Some(comma) = ctx.eat_kind(TokenKind::Comma) {
                children.extend(trivia2);
                children.push(token_segment(comma, SegmentType::Comma));
                children.extend(eat_trivia_segments(ctx));
                if let Some(expr) = self.parse_expression(ctx) {
                    children.push(expr);
                }
            } else {
                ctx.restore(save2);
                break;
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::ThrowStatement,
            children,
        )))
    }

    /// Parse RAISERROR(msg, severity, state) [WITH option]
    fn parse_raiserror_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("RAISERROR")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Arguments in parens
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            if let Some(args) = self.parse_paren_block(ctx) {
                children.push(args);
            }
        }

        // Optional WITH option
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("WITH") {
            let with_kw = ctx.advance().unwrap();
            children.push(token_segment(with_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
            // Options: NOWAIT, LOG, SETERROR
            while ctx.peek_kind() == Some(TokenKind::Word) {
                let opt = ctx.advance().unwrap();
                children.push(token_segment(opt, SegmentType::Keyword));
                let save = ctx.save();
                let trivia = eat_trivia_segments(ctx);
                if ctx.peek_kind() == Some(TokenKind::Comma) {
                    children.extend(trivia);
                    let comma = ctx.advance().unwrap();
                    children.push(token_segment(comma, SegmentType::Comma));
                    children.extend(eat_trivia_segments(ctx));
                } else {
                    ctx.restore(save);
                    break;
                }
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::RaiserrorStatement,
            children,
        )))
    }

    /// Parse GO batch separator
    fn parse_go_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("GO")?;
        children.push(token_segment(kw, SegmentType::Keyword));

        // Optional count: GO 5
        let save = ctx.save();
        let trivia = eat_trivia_segments(ctx);
        if ctx.peek_kind() == Some(TokenKind::NumberLiteral) {
            children.extend(trivia);
            let num = ctx.advance().unwrap();
            children.push(token_segment(num, SegmentType::NumericLiteral));
        } else {
            ctx.restore(save);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::GoStatement,
            children,
        )))
    }
}
