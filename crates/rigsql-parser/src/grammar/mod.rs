mod ansi;
mod postgres;
mod tsql;

pub use ansi::AnsiGrammar;
pub use postgres::PostgresGrammar;
pub use tsql::TsqlGrammar;

use rigsql_core::{NodeSegment, Segment, SegmentType, Token, TokenKind, TokenSegment};

use crate::context::ParseContext;

// ── Grammar trait ────────────────────────────────────────────────

/// Trait for dialect-specific SQL grammar.
///
/// Dialects implement `dispatch_statement` and `statement_keywords`.
/// All shared ANSI parsing logic lives in default methods.
pub trait Grammar: Send + Sync {
    /// Return the set of keywords that can start a statement in this dialect.
    fn statement_keywords(&self) -> &[&str];

    /// Dispatch a single statement based on the current token.
    /// Called from `parse_statement` after consuming leading trivia.
    fn dispatch_statement(&self, ctx: &mut ParseContext) -> Option<Segment>;

    /// ANSI-only statement dispatch.  Dialect impls can call this as fallback.
    fn dispatch_ansi_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        if ctx.peek_keyword("SELECT") || ctx.peek_keyword("WITH") {
            self.parse_select_statement(ctx)
        } else if ctx.peek_keyword("INSERT") {
            self.parse_insert_statement(ctx)
        } else if ctx.peek_keyword("UPDATE") {
            self.parse_update_statement(ctx)
        } else if ctx.peek_keyword("DELETE") {
            self.parse_delete_statement(ctx)
        } else if ctx.peek_keyword("CREATE") {
            self.parse_create_statement(ctx)
        } else if ctx.peek_keyword("DROP") {
            self.parse_drop_statement(ctx)
        } else if ctx.peek_keyword("ALTER") {
            self.parse_alter_statement(ctx)
        } else if ctx.peek_keyword("USE")
            || ctx.peek_keyword("TRUNCATE")
            || ctx.peek_keyword("OPEN")
            || ctx.peek_keyword("CLOSE")
            || ctx.peek_keyword("DEALLOCATE")
            || ctx.peek_keyword("FETCH")
            || ctx.peek_keyword("BREAK")
            || ctx.peek_keyword("CONTINUE")
            || ctx.peek_keyword("MERGE")
        {
            self.parse_simple_statement(ctx)
        } else {
            None
        }
    }

    // ── Top-level ────────────────────────────────────────────────

    /// Parse a complete SQL file: zero or more statements.
    fn parse_file(&self, ctx: &mut ParseContext) -> Segment {
        let mut children = Vec::new();
        while !ctx.at_eof() {
            children.extend(eat_trivia_segments(ctx));
            if ctx.at_eof() {
                break;
            }
            if let Some(stmt) = self.parse_statement(ctx) {
                children.push(stmt);
            } else {
                // Error recovery: skip to the next statement boundary
                // (semicolon or a recognised statement keyword) and wrap
                // all skipped tokens in a single Unparsable node.
                let error_offset = ctx
                    .peek()
                    .map(|t| t.span.start)
                    .unwrap_or(ctx.source().len() as u32);
                let mut unparsable_children = Vec::new();
                // Always consume at least one token to guarantee forward
                // progress and avoid infinite loops (e.g. when a statement
                // keyword like WITH is not actually a valid statement start
                // in this context, such as WITH(NOLOCK) table hints).
                if let Some(token) = ctx.advance() {
                    unparsable_children.push(any_token_segment(token));
                }
                while !ctx.at_eof() {
                    // Stop before a semicolon — consume it as part of the
                    // unparsable node so the next iteration starts cleanly.
                    if ctx.peek_kind() == Some(TokenKind::Semicolon) {
                        if let Some(semi) = ctx.advance() {
                            unparsable_children.push(token_segment(semi, SegmentType::Semicolon));
                        }
                        break;
                    }
                    // Stop before a token that looks like it starts a new statement.
                    if self.peek_statement_start(ctx) {
                        break;
                    }
                    if let Some(token) = ctx.advance() {
                        unparsable_children.push(any_token_segment(token));
                    }
                }
                if !unparsable_children.is_empty() {
                    children.push(Segment::Node(NodeSegment::new(
                        SegmentType::Unparsable,
                        unparsable_children,
                    )));
                    ctx.record_error_at(
                        error_offset,
                        "Unparsable segment: could not match any statement",
                    );
                }
            }
        }
        Segment::Node(NodeSegment::new(SegmentType::File, children))
    }

    /// Parse a single statement (terminated by `;` or EOF).
    fn parse_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let save = ctx.save();
        let mut children = Vec::new();
        children.extend(eat_trivia_segments(ctx));

        let inner = self.dispatch_statement(ctx);

        match inner {
            Some(stmt_seg) => {
                children.push(stmt_seg);
                // Optional trailing semicolon
                children.extend(eat_trivia_segments(ctx));
                if let Some(semi) = ctx.eat_kind(TokenKind::Semicolon) {
                    children.push(token_segment(semi, SegmentType::Semicolon));
                }
                Some(Segment::Node(NodeSegment::new(
                    SegmentType::Statement,
                    children,
                )))
            }
            None => {
                ctx.restore(save);
                None
            }
        }
    }

    // ── SELECT ───────────────────────────────────────────────────

    fn parse_select_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();

        // WITH clause (optional)
        if ctx.peek_keyword("WITH") {
            if let Some(with) = self.parse_with_clause(ctx) {
                children.push(with);
                children.extend(eat_trivia_segments(ctx));
            }
        }

        // SELECT clause (required)
        let select = self.parse_select_clause(ctx)?;
        children.push(select);

        // FROM clause (optional)
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("FROM") {
            if let Some(from) = self.parse_from_clause(ctx) {
                children.push(from);
            }
        }

        // WHERE clause (optional)
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("WHERE") {
            if let Some(wh) = self.parse_where_clause(ctx) {
                children.push(wh);
            }
        }

        // GROUP BY clause (optional)
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keywords(&["GROUP", "BY"]) {
            if let Some(gb) = self.parse_group_by_clause(ctx) {
                children.push(gb);
            }
        }

        // HAVING clause (optional)
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("HAVING") {
            if let Some(hav) = self.parse_having_clause(ctx) {
                children.push(hav);
            }
        }

        // ORDER BY clause (optional)
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keywords(&["ORDER", "BY"]) {
            if let Some(ob) = self.parse_order_by_clause(ctx) {
                children.push(ob);
            }
        }

        // LIMIT clause (optional)
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("LIMIT") {
            if let Some(lim) = self.parse_limit_clause(ctx) {
                children.push(lim);
            }
        }

        // OFFSET clause (optional)
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("OFFSET") {
            if let Some(off) = self.parse_offset_clause(ctx) {
                children.push(off);
            }
        }

        // UNION / INTERSECT / EXCEPT (optional, recursive)
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("UNION") || ctx.peek_keyword("INTERSECT") || ctx.peek_keyword("EXCEPT")
        {
            if let Some(set_op) = self.parse_set_operation(ctx) {
                children.push(set_op);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::SelectStatement,
            children,
        )))
    }

    fn parse_select_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();

        let kw = ctx.eat_keyword("SELECT")?;
        children.push(token_segment(kw, SegmentType::Keyword));

        children.extend(eat_trivia_segments(ctx));

        // DISTINCT / ALL (optional)
        if ctx.peek_keyword("DISTINCT") || ctx.peek_keyword("ALL") {
            if let Some(token) = ctx.advance() {
                children.push(token_segment(token, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
            }
        }

        // TOP (N) / TOP N (TSQL)
        if ctx.peek_keyword("TOP") {
            if let Some(top_kw) = ctx.advance() {
                children.push(token_segment(top_kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
                // TOP (expr) or TOP N
                if let Some(lparen) = ctx.eat_kind(TokenKind::LParen) {
                    children.push(token_segment(lparen, SegmentType::LParen));
                    children.extend(eat_trivia_segments(ctx));
                    if let Some(expr) = self.parse_expression(ctx) {
                        children.push(expr);
                    }
                    children.extend(eat_trivia_segments(ctx));
                    if let Some(rparen) = ctx.eat_kind(TokenKind::RParen) {
                        children.push(token_segment(rparen, SegmentType::RParen));
                    }
                } else if let Some(num) = ctx.eat_kind(TokenKind::NumberLiteral) {
                    children.push(token_segment(num, SegmentType::Literal));
                }
                children.extend(eat_trivia_segments(ctx));
            }
        }

        // Select targets (comma-separated expressions)
        parse_comma_separated(ctx, &mut children, |c| self.parse_select_target(c));

        Some(Segment::Node(NodeSegment::new(
            SegmentType::SelectClause,
            children,
        )))
    }

    fn parse_select_target(&self, ctx: &mut ParseContext) -> Option<Segment> {
        // Parse an expression, optionally followed by alias (AS alias_name or just alias_name)
        let expr = self.parse_expression(ctx)?;

        let save = ctx.save();
        let trivia = eat_trivia_segments(ctx);

        // Check for alias: AS name, or just a word that's not a keyword
        if ctx.peek_keyword("AS") {
            let mut children = vec![expr];
            children.extend(trivia);
            let as_kw = ctx.advance().unwrap();
            children.push(token_segment(as_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
            if let Some(alias) = self.parse_identifier(ctx) {
                children.push(alias);
            }
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::AliasExpression,
                children,
            )));
        }

        // Implicit alias: a bare word that's not a clause keyword
        if let Some(t) = ctx.peek_non_trivia() {
            if t.kind == TokenKind::Word && !is_clause_keyword(&t.text) {
                let mut children = vec![expr];
                children.extend(trivia);
                if let Some(alias) = self.parse_identifier(ctx) {
                    children.push(alias);
                }
                return Some(Segment::Node(NodeSegment::new(
                    SegmentType::AliasExpression,
                    children,
                )));
            }
        }

        ctx.restore(save);
        // Re-eat trivia would happen naturally at calling site
        Some(expr)
    }

    fn parse_from_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("FROM")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Table references (comma-separated)
        parse_comma_separated(ctx, &mut children, |c| self.parse_table_reference(c));

        // JOIN clauses
        loop {
            children.extend(eat_trivia_segments(ctx));
            if peek_join_keyword(ctx) {
                if let Some(join) = self.parse_join_clause(ctx) {
                    children.push(join);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::FromClause,
            children,
        )))
    }

    fn parse_table_reference(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let save = ctx.save();

        // Subquery in parens
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            if let Some(subq) = self.parse_paren_subquery(ctx) {
                // Optional alias
                let save2 = ctx.save();
                let trivia = eat_trivia_segments(ctx);
                if ctx.peek_keyword("AS")
                    || ctx
                        .peek_non_trivia()
                        .is_some_and(|t| t.kind == TokenKind::Word && !is_clause_keyword(&t.text))
                {
                    let mut children = vec![subq];
                    children.extend(trivia);
                    if ctx.peek_keyword("AS") {
                        let kw = ctx.advance().unwrap();
                        children.push(token_segment(kw, SegmentType::Keyword));
                        children.extend(eat_trivia_segments(ctx));
                    }
                    if let Some(alias) = self.parse_identifier(ctx) {
                        children.push(alias);
                    }
                    return Some(Segment::Node(NodeSegment::new(
                        SegmentType::AliasExpression,
                        children,
                    )));
                }
                ctx.restore(save2);
                return Some(subq);
            }
        }

        // Table name (possibly qualified: schema.table)
        let name = self.parse_qualified_name(ctx);
        if name.is_none() {
            ctx.restore(save);
            return None;
        }
        let name = name.unwrap();

        // Optional alias
        let save2 = ctx.save();
        let trivia = eat_trivia_segments(ctx);
        if ctx.peek_keyword("AS") {
            let mut children = vec![name];
            children.extend(trivia);
            let kw = ctx.advance().unwrap();
            children.push(token_segment(kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
            if let Some(alias) = self.parse_identifier(ctx) {
                children.push(alias);
            }
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::AliasExpression,
                children,
            )));
        }
        if let Some(t) = ctx.peek_non_trivia() {
            if t.kind == TokenKind::Word && !is_clause_keyword(&t.text) && !is_join_keyword(&t.text)
            {
                let mut children = vec![name];
                children.extend(trivia);
                if let Some(alias) = self.parse_identifier(ctx) {
                    children.push(alias);
                }
                return Some(Segment::Node(NodeSegment::new(
                    SegmentType::AliasExpression,
                    children,
                )));
            }
        }
        ctx.restore(save2);
        Some(name)
    }

    fn parse_where_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("WHERE")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));
        if let Some(expr) = self.parse_expression(ctx) {
            children.push(expr);
        }
        Some(Segment::Node(NodeSegment::new(
            SegmentType::WhereClause,
            children,
        )))
    }

    fn parse_group_by_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let group_kw = ctx.eat_keyword("GROUP")?;
        children.push(token_segment(group_kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));
        let by_kw = ctx.eat_keyword("BY")?;
        children.push(token_segment(by_kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Comma-separated expressions
        parse_comma_separated(ctx, &mut children, |c| self.parse_expression(c));

        Some(Segment::Node(NodeSegment::new(
            SegmentType::GroupByClause,
            children,
        )))
    }

    fn parse_having_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("HAVING")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));
        if let Some(expr) = self.parse_expression(ctx) {
            children.push(expr);
        }
        Some(Segment::Node(NodeSegment::new(
            SegmentType::HavingClause,
            children,
        )))
    }

    fn parse_order_by_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let order_kw = ctx.eat_keyword("ORDER")?;
        children.push(token_segment(order_kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));
        let by_kw = ctx.eat_keyword("BY")?;
        children.push(token_segment(by_kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Comma-separated order expressions
        parse_comma_separated(ctx, &mut children, |c| self.parse_order_expression(c));

        Some(Segment::Node(NodeSegment::new(
            SegmentType::OrderByClause,
            children,
        )))
    }

    fn parse_order_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let expr = self.parse_expression(ctx)?;
        children.push(expr);

        let save = ctx.save();
        let trivia = eat_trivia_segments(ctx);
        if ctx.peek_keyword("ASC") || ctx.peek_keyword("DESC") {
            children.extend(trivia);
            let kw = ctx.advance().unwrap();
            children.push(token_segment(kw, SegmentType::Keyword));
        } else {
            ctx.restore(save);
        }

        // NULLS FIRST / NULLS LAST
        let save = ctx.save();
        let trivia = eat_trivia_segments(ctx);
        if ctx.peek_keyword("NULLS") {
            children.extend(trivia);
            let kw = ctx.advance().unwrap();
            children.push(token_segment(kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
            if ctx.peek_keyword("FIRST") || ctx.peek_keyword("LAST") {
                let kw = ctx.advance().unwrap();
                children.push(token_segment(kw, SegmentType::Keyword));
            }
        } else {
            ctx.restore(save);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::OrderByExpression,
            children,
        )))
    }

    fn parse_limit_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("LIMIT")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));
        if let Some(expr) = self.parse_expression(ctx) {
            children.push(expr);
        }
        Some(Segment::Node(NodeSegment::new(
            SegmentType::LimitClause,
            children,
        )))
    }

    fn parse_offset_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("OFFSET")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));
        if let Some(expr) = self.parse_expression(ctx) {
            children.push(expr);
        }
        Some(Segment::Node(NodeSegment::new(
            SegmentType::OffsetClause,
            children,
        )))
    }

    fn parse_with_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("WITH")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // RECURSIVE (optional)
        if ctx.peek_keyword("RECURSIVE") {
            let kw = ctx.advance().unwrap();
            children.push(token_segment(kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
        }

        // CTE definitions (comma-separated)
        parse_comma_separated(ctx, &mut children, |c| self.parse_cte_definition(c));

        Some(Segment::Node(NodeSegment::new(
            SegmentType::WithClause,
            children,
        )))
    }

    fn parse_cte_definition(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let name = self.parse_identifier(ctx)?;
        children.push(name);
        children.extend(eat_trivia_segments(ctx));

        // Optional column list
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            if let Some(cols) = self.parse_paren_block(ctx) {
                children.push(cols);
                children.extend(eat_trivia_segments(ctx));
            }
        }

        let as_kw = ctx.eat_keyword("AS")?;
        children.push(token_segment(as_kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // ( subquery )
        if let Some(subq) = self.parse_paren_subquery(ctx) {
            children.push(subq);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::CteDefinition,
            children,
        )))
    }

    fn parse_set_operation(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();

        // UNION / INTERSECT / EXCEPT
        let kw = ctx.advance()?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // ALL / DISTINCT (optional)
        if ctx.peek_keyword("ALL") || ctx.peek_keyword("DISTINCT") {
            let kw = ctx.advance().unwrap();
            children.push(token_segment(kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
        }

        // Following select
        if let Some(sel) = self.parse_select_statement(ctx) {
            children.push(sel);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::SelectStatement,
            children,
        )))
    }

    // ── JOIN ─────────────────────────────────────────────────────

    fn parse_join_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();

        // Optional: INNER / LEFT / RIGHT / FULL / CROSS
        if ctx.peek_keyword("INNER")
            || ctx.peek_keyword("LEFT")
            || ctx.peek_keyword("RIGHT")
            || ctx.peek_keyword("FULL")
            || ctx.peek_keyword("CROSS")
        {
            let kw = ctx.advance().unwrap();
            children.push(token_segment(kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));

            // Optional: OUTER
            if ctx.peek_keyword("OUTER") {
                let kw = ctx.advance().unwrap();
                children.push(token_segment(kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
            }
        }

        let join_kw = ctx.eat_keyword("JOIN")?;
        children.push(token_segment(join_kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Table reference
        if let Some(tref) = self.parse_table_reference(ctx) {
            children.push(tref);
        }

        // ON or USING
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("ON") {
            let kw = ctx.advance().unwrap();
            let mut on_children = vec![token_segment(kw, SegmentType::Keyword)];
            on_children.extend(eat_trivia_segments(ctx));
            if let Some(expr) = self.parse_expression(ctx) {
                on_children.push(expr);
            }
            children.push(Segment::Node(NodeSegment::new(
                SegmentType::OnClause,
                on_children,
            )));
        } else if ctx.peek_keyword("USING") {
            let kw = ctx.advance().unwrap();
            let mut using_children = vec![token_segment(kw, SegmentType::Keyword)];
            using_children.extend(eat_trivia_segments(ctx));
            if let Some(paren) = self.parse_paren_block(ctx) {
                using_children.push(paren);
            }
            children.push(Segment::Node(NodeSegment::new(
                SegmentType::UsingClause,
                using_children,
            )));
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::JoinClause,
            children,
        )))
    }

    // ── INSERT ───────────────────────────────────────────────────

    fn parse_insert_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("INSERT")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        let into_kw = ctx.eat_keyword("INTO")?;
        children.push(token_segment(into_kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Table name
        if let Some(name) = self.parse_qualified_name(ctx) {
            children.push(name);
        }
        children.extend(eat_trivia_segments(ctx));

        // Optional column list
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            if let Some(cols) = self.parse_paren_block(ctx) {
                children.push(cols);
                children.extend(eat_trivia_segments(ctx));
            }
        }

        // VALUES or SELECT
        if ctx.peek_keyword("VALUES") {
            if let Some(vals) = self.parse_values_clause(ctx) {
                children.push(vals);
            }
        } else if ctx.peek_keyword("SELECT") || ctx.peek_keyword("WITH") {
            if let Some(sel) = self.parse_select_statement(ctx) {
                children.push(sel);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::InsertStatement,
            children,
        )))
    }

    fn parse_values_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("VALUES")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Comma-separated (expr, expr, ...)
        parse_comma_separated(ctx, &mut children, |c| self.parse_paren_block(c));

        Some(Segment::Node(NodeSegment::new(
            SegmentType::ValuesClause,
            children,
        )))
    }

    // ── UPDATE ───────────────────────────────────────────────────

    fn parse_update_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("UPDATE")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Table name
        if let Some(name) = self.parse_table_reference(ctx) {
            children.push(name);
        }
        children.extend(eat_trivia_segments(ctx));

        // SET clause
        if ctx.peek_keyword("SET") {
            if let Some(set) = self.parse_set_clause(ctx) {
                children.push(set);
            }
        }

        // WHERE clause
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("WHERE") {
            if let Some(wh) = self.parse_where_clause(ctx) {
                children.push(wh);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::UpdateStatement,
            children,
        )))
    }

    fn parse_set_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("SET")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // col = expr, ...
        parse_comma_separated(ctx, &mut children, |c| self.parse_expression(c));

        Some(Segment::Node(NodeSegment::new(
            SegmentType::SetClause,
            children,
        )))
    }

    // ── DELETE ────────────────────────────────────────────────────

    fn parse_delete_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("DELETE")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // FROM
        if ctx.peek_keyword("FROM") {
            let from_kw = ctx.advance().unwrap();
            children.push(token_segment(from_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
        }

        // Table name
        if let Some(name) = self.parse_qualified_name(ctx) {
            children.push(name);
        }

        // WHERE clause
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("WHERE") {
            if let Some(wh) = self.parse_where_clause(ctx) {
                children.push(wh);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::DeleteStatement,
            children,
        )))
    }

    // ── DDL ──────────────────────────────────────────────────────

    fn parse_create_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("CREATE")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        if ctx.peek_keyword("TABLE") {
            return self.parse_create_table_body(ctx, children);
        }

        // For other CREATE statements, consume until semicolon or EOF
        self.consume_until_end(ctx, &mut children);
        Some(Segment::Node(NodeSegment::new(
            SegmentType::Statement,
            children,
        )))
    }

    fn parse_create_table_body(
        &self,
        ctx: &mut ParseContext,
        mut children: Vec<Segment>,
    ) -> Option<Segment> {
        let kw = ctx.eat_keyword("TABLE")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // IF NOT EXISTS
        if ctx.peek_keyword("IF") {
            let kw = ctx.advance().unwrap();
            children.push(token_segment(kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
            if let Some(kw) = ctx.eat_keyword("NOT") {
                children.push(token_segment(kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
            }
            if let Some(kw) = ctx.eat_keyword("EXISTS") {
                children.push(token_segment(kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
            }
        }

        // Table name
        if let Some(name) = self.parse_qualified_name(ctx) {
            children.push(name);
        }
        children.extend(eat_trivia_segments(ctx));

        // Column definitions in parens
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            if let Some(defs) = self.parse_paren_block(ctx) {
                children.push(defs);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::CreateTableStatement,
            children,
        )))
    }

    fn parse_drop_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("DROP")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Consume until semicolon
        self.consume_until_end(ctx, &mut children);
        Some(Segment::Node(NodeSegment::new(
            SegmentType::DropStatement,
            children,
        )))
    }

    fn parse_alter_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("ALTER")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        self.consume_until_end(ctx, &mut children);
        Some(Segment::Node(NodeSegment::new(
            SegmentType::AlterTableStatement,
            children,
        )))
    }

    // ── Expression parsing ───────────────────────────────────────

    /// Parse an expression. This uses a simple precedence climbing approach.
    fn parse_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        self.parse_or_expression(ctx)
    }

    fn parse_or_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut left = self.parse_and_expression(ctx)?;
        loop {
            let save = ctx.save();
            let trivia = eat_trivia_segments(ctx);
            if ctx.peek_keyword("OR") {
                let op = ctx.advance().unwrap();
                let mut children = vec![left];
                children.extend(trivia);
                children.push(token_segment(op, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
                if let Some(right) = self.parse_and_expression(ctx) {
                    children.push(right);
                }
                left = Segment::Node(NodeSegment::new(SegmentType::BinaryExpression, children));
            } else {
                ctx.restore(save);
                break;
            }
        }
        Some(left)
    }

    fn parse_and_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut left = self.parse_not_expression(ctx)?;
        loop {
            let save = ctx.save();
            let trivia = eat_trivia_segments(ctx);
            if ctx.peek_keyword("AND") {
                let op = ctx.advance().unwrap();
                let mut children = vec![left];
                children.extend(trivia);
                children.push(token_segment(op, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
                if let Some(right) = self.parse_not_expression(ctx) {
                    children.push(right);
                }
                left = Segment::Node(NodeSegment::new(SegmentType::BinaryExpression, children));
            } else {
                ctx.restore(save);
                break;
            }
        }
        Some(left)
    }

    fn parse_not_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        if ctx.peek_keyword("NOT") {
            let mut children = Vec::new();
            let kw = ctx.advance().unwrap();
            children.push(token_segment(kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
            if let Some(expr) = self.parse_not_expression(ctx) {
                children.push(expr);
            }
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::UnaryExpression,
                children,
            )));
        }
        self.parse_comparison_expression(ctx)
    }

    fn parse_comparison_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let left = self.parse_addition_expression(ctx)?;

        let save = ctx.save();
        let trivia = eat_trivia_segments(ctx);

        // IS [NOT] NULL
        if ctx.peek_keyword("IS") {
            let is_kw = ctx.advance().unwrap();
            let mut children = vec![left];
            children.extend(trivia);
            children.push(token_segment(is_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
            if ctx.peek_keyword("NOT") {
                let not_kw = ctx.advance().unwrap();
                children.push(token_segment(not_kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
            }
            if ctx.peek_keyword("NULL") {
                let null_kw = ctx.advance().unwrap();
                children.push(token_segment(null_kw, SegmentType::Keyword));
            }
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::IsNullExpression,
                children,
            )));
        }

        // [NOT] IN (...)
        if ctx.peek_keyword("IN") {
            let in_kw = ctx.advance().unwrap();
            let mut children = vec![left];
            children.extend(trivia);
            children.push(token_segment(in_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
            if ctx.peek_kind() == Some(TokenKind::LParen) {
                if let Some(list) = self.parse_paren_block(ctx) {
                    children.push(list);
                }
            }
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::InExpression,
                children,
            )));
        }

        // NOT IN / NOT BETWEEN / NOT LIKE
        if ctx.peek_keyword("NOT") {
            let save_not = ctx.save();
            let not_kw = ctx.advance().unwrap();
            let not_trivia = eat_trivia_segments(ctx);

            if ctx.peek_keyword("IN") {
                let in_kw = ctx.advance().unwrap();
                let mut children = vec![left];
                children.extend(trivia);
                children.push(token_segment(not_kw, SegmentType::Keyword));
                children.extend(not_trivia);
                children.push(token_segment(in_kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
                if ctx.peek_kind() == Some(TokenKind::LParen) {
                    if let Some(list) = self.parse_paren_block(ctx) {
                        children.push(list);
                    }
                }
                return Some(Segment::Node(NodeSegment::new(
                    SegmentType::InExpression,
                    children,
                )));
            }
            if ctx.peek_keyword("BETWEEN") {
                let kw = ctx.advance().unwrap();
                let mut children = vec![left];
                children.extend(trivia);
                children.push(token_segment(not_kw, SegmentType::Keyword));
                children.extend(not_trivia);
                children.push(token_segment(kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
                if let Some(lo) = self.parse_addition_expression(ctx) {
                    children.push(lo);
                }
                children.extend(eat_trivia_segments(ctx));
                if let Some(and_kw) = ctx.eat_keyword("AND") {
                    children.push(token_segment(and_kw, SegmentType::Keyword));
                }
                children.extend(eat_trivia_segments(ctx));
                if let Some(hi) = self.parse_addition_expression(ctx) {
                    children.push(hi);
                }
                return Some(Segment::Node(NodeSegment::new(
                    SegmentType::BetweenExpression,
                    children,
                )));
            }
            if ctx.peek_keyword("LIKE") || ctx.peek_keyword("ILIKE") {
                let kw = ctx.advance().unwrap();
                let mut children = vec![left];
                children.extend(trivia);
                children.push(token_segment(not_kw, SegmentType::Keyword));
                children.extend(not_trivia);
                children.push(token_segment(kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
                if let Some(pattern) = self.parse_addition_expression(ctx) {
                    children.push(pattern);
                }
                return Some(Segment::Node(NodeSegment::new(
                    SegmentType::LikeExpression,
                    children,
                )));
            }

            // NOT was consumed but wasn't NOT IN/BETWEEN/LIKE — restore
            ctx.restore(save_not);
            ctx.restore(save);
            return Some(left);
        }

        // BETWEEN ... AND ...
        if ctx.peek_keyword("BETWEEN") {
            let kw = ctx.advance().unwrap();
            let mut children = vec![left];
            children.extend(trivia);
            children.push(token_segment(kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
            if let Some(lo) = self.parse_addition_expression(ctx) {
                children.push(lo);
            }
            children.extend(eat_trivia_segments(ctx));
            if let Some(and_kw) = ctx.eat_keyword("AND") {
                children.push(token_segment(and_kw, SegmentType::Keyword));
            }
            children.extend(eat_trivia_segments(ctx));
            if let Some(hi) = self.parse_addition_expression(ctx) {
                children.push(hi);
            }
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::BetweenExpression,
                children,
            )));
        }

        // LIKE / ILIKE
        if ctx.peek_keyword("LIKE") || ctx.peek_keyword("ILIKE") {
            let kw = ctx.advance().unwrap();
            let mut children = vec![left];
            children.extend(trivia);
            children.push(token_segment(kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
            if let Some(pattern) = self.parse_addition_expression(ctx) {
                children.push(pattern);
            }
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::LikeExpression,
                children,
            )));
        }

        // Comparison operators: = <> != < > <= >=
        if let Some(kind) = ctx.peek_kind() {
            if matches!(
                kind,
                TokenKind::Eq
                    | TokenKind::Neq
                    | TokenKind::Lt
                    | TokenKind::Gt
                    | TokenKind::LtEq
                    | TokenKind::GtEq
            ) {
                let op = ctx.advance().unwrap();
                let mut children = vec![left];
                children.extend(trivia);
                children.push(token_segment(op, SegmentType::ComparisonOperator));
                children.extend(eat_trivia_segments(ctx));
                if let Some(right) = self.parse_addition_expression(ctx) {
                    children.push(right);
                }
                return Some(Segment::Node(NodeSegment::new(
                    SegmentType::BinaryExpression,
                    children,
                )));
            }
        }

        ctx.restore(save);
        Some(left)
    }

    fn parse_addition_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut left = self.parse_multiplication_expression(ctx)?;
        loop {
            let save = ctx.save();
            let trivia = eat_trivia_segments(ctx);
            if let Some(kind) = ctx.peek_kind() {
                if matches!(kind, TokenKind::Plus | TokenKind::Minus | TokenKind::Concat) {
                    let op = ctx.advance().unwrap();
                    let mut children = vec![left];
                    children.extend(trivia);
                    children.push(token_segment(op, SegmentType::ArithmeticOperator));
                    children.extend(eat_trivia_segments(ctx));
                    if let Some(right) = self.parse_multiplication_expression(ctx) {
                        children.push(right);
                    }
                    left = Segment::Node(NodeSegment::new(SegmentType::BinaryExpression, children));
                    continue;
                }
            }
            ctx.restore(save);
            break;
        }
        Some(left)
    }

    fn parse_multiplication_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut left = self.parse_unary_expression(ctx)?;
        loop {
            let save = ctx.save();
            let trivia = eat_trivia_segments(ctx);
            if let Some(kind) = ctx.peek_kind() {
                if matches!(
                    kind,
                    TokenKind::Star | TokenKind::Slash | TokenKind::Percent
                ) {
                    let op = ctx.advance().unwrap();
                    let mut children = vec![left];
                    children.extend(trivia);
                    children.push(token_segment(op, SegmentType::ArithmeticOperator));
                    children.extend(eat_trivia_segments(ctx));
                    if let Some(right) = self.parse_unary_expression(ctx) {
                        children.push(right);
                    }
                    left = Segment::Node(NodeSegment::new(SegmentType::BinaryExpression, children));
                    continue;
                }
            }
            ctx.restore(save);
            break;
        }
        Some(left)
    }

    fn parse_unary_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        if let Some(kind) = ctx.peek_kind() {
            if matches!(kind, TokenKind::Plus | TokenKind::Minus) {
                let op = ctx.advance().unwrap();
                let mut children = vec![token_segment(op, SegmentType::ArithmeticOperator)];
                children.extend(eat_trivia_segments(ctx));
                if let Some(expr) = self.parse_primary_expression(ctx) {
                    children.push(expr);
                }
                return Some(Segment::Node(NodeSegment::new(
                    SegmentType::UnaryExpression,
                    children,
                )));
            }
        }
        self.parse_primary_expression(ctx)
    }

    fn parse_primary_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        match ctx.peek_kind()? {
            // Parenthesized expression or subquery
            TokenKind::LParen => {
                // Check if it's a subquery
                let save = ctx.save();
                if let Some(subq) = self.parse_paren_subquery(ctx) {
                    return Some(subq);
                }
                ctx.restore(save);
                self.parse_paren_expression(ctx)
            }

            // Number literal
            TokenKind::NumberLiteral => {
                let token = ctx.advance().unwrap();
                Some(token_segment(token, SegmentType::NumericLiteral))
            }

            // String literal
            TokenKind::StringLiteral => {
                let token = ctx.advance().unwrap();
                Some(token_segment(token, SegmentType::StringLiteral))
            }

            // Star (e.g. SELECT *)
            TokenKind::Star => {
                let token = ctx.advance().unwrap();
                Some(token_segment(token, SegmentType::Star))
            }

            // Placeholder
            TokenKind::Placeholder => {
                let token = ctx.advance().unwrap();
                Some(token_segment(token, SegmentType::Literal))
            }

            // Quoted identifier
            TokenKind::QuotedIdentifier => {
                let token = ctx.advance().unwrap();
                Some(token_segment(token, SegmentType::QuotedIdentifier))
            }

            // Word: keyword, function call, column ref, etc.
            TokenKind::Word => {
                let text = &ctx.peek().unwrap().text;

                if text.eq_ignore_ascii_case("CASE") {
                    return self.parse_case_expression(ctx);
                }
                if text.eq_ignore_ascii_case("EXISTS") {
                    return self.parse_exists_expression(ctx);
                }
                if text.eq_ignore_ascii_case("CAST") {
                    return self.parse_cast_expression(ctx);
                }
                if text.eq_ignore_ascii_case("TRUE") || text.eq_ignore_ascii_case("FALSE") {
                    let token = ctx.advance().unwrap();
                    return Some(token_segment(token, SegmentType::BooleanLiteral));
                }
                if text.eq_ignore_ascii_case("NULL") {
                    let token = ctx.advance().unwrap();
                    return Some(token_segment(token, SegmentType::NullLiteral));
                }

                self.parse_name_or_function(ctx)
            }

            // @ variable (SQL Server)
            TokenKind::AtSign => {
                let token = ctx.advance().unwrap();
                Some(token_segment(token, SegmentType::Identifier))
            }

            _ => None,
        }
    }

    fn parse_paren_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let lp = ctx.eat_kind(TokenKind::LParen)?;
        children.push(token_segment(lp, SegmentType::LParen));
        children.extend(eat_trivia_segments(ctx));

        if let Some(expr) = self.parse_expression(ctx) {
            children.push(expr);
        }

        children.extend(eat_trivia_segments(ctx));
        if let Some(rp) = ctx.eat_kind(TokenKind::RParen) {
            children.push(token_segment(rp, SegmentType::RParen));
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::ParenExpression,
            children,
        )))
    }

    fn parse_paren_subquery(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let save = ctx.save();
        let mut children = Vec::new();
        let lp = ctx.eat_kind(TokenKind::LParen)?;
        children.push(token_segment(lp, SegmentType::LParen));
        children.extend(eat_trivia_segments(ctx));

        // Check if it's a SELECT or WITH inside
        if !ctx.peek_keyword("SELECT") && !ctx.peek_keyword("WITH") {
            ctx.restore(save);
            return None;
        }

        if let Some(sel) = self.parse_select_statement(ctx) {
            children.push(sel);
        } else {
            ctx.restore(save);
            return None;
        }

        children.extend(eat_trivia_segments(ctx));
        if let Some(rp) = ctx.eat_kind(TokenKind::RParen) {
            children.push(token_segment(rp, SegmentType::RParen));
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::Subquery,
            children,
        )))
    }

    fn parse_case_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let case_kw = ctx.eat_keyword("CASE")?;
        children.push(token_segment(case_kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Simple CASE: CASE expr WHEN ...
        // Searched CASE: CASE WHEN ...
        if !ctx.peek_keyword("WHEN") {
            if let Some(expr) = self.parse_expression(ctx) {
                children.push(expr);
                children.extend(eat_trivia_segments(ctx));
            }
        }

        // WHEN clauses
        while ctx.peek_keyword("WHEN") {
            if let Some(when) = self.parse_when_clause(ctx) {
                children.push(when);
                children.extend(eat_trivia_segments(ctx));
            }
        }

        // ELSE clause
        if ctx.peek_keyword("ELSE") {
            let mut else_children = Vec::new();
            let kw = ctx.advance().unwrap();
            else_children.push(token_segment(kw, SegmentType::Keyword));
            else_children.extend(eat_trivia_segments(ctx));
            if let Some(expr) = self.parse_expression(ctx) {
                else_children.push(expr);
            }
            children.push(Segment::Node(NodeSegment::new(
                SegmentType::ElseClause,
                else_children,
            )));
            children.extend(eat_trivia_segments(ctx));
        }

        // END
        if let Some(end_kw) = ctx.eat_keyword("END") {
            children.push(token_segment(end_kw, SegmentType::Keyword));
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::CaseExpression,
            children,
        )))
    }

    fn parse_when_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("WHEN")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        if let Some(cond) = self.parse_expression(ctx) {
            children.push(cond);
        }
        children.extend(eat_trivia_segments(ctx));

        if let Some(then_kw) = ctx.eat_keyword("THEN") {
            children.push(token_segment(then_kw, SegmentType::Keyword));
        }
        children.extend(eat_trivia_segments(ctx));

        if let Some(result) = self.parse_expression(ctx) {
            children.push(result);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::WhenClause,
            children,
        )))
    }

    fn parse_exists_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("EXISTS")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        if let Some(subq) = self.parse_paren_subquery(ctx) {
            children.push(subq);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::ExistsExpression,
            children,
        )))
    }

    fn parse_cast_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("CAST")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        let lp = ctx.eat_kind(TokenKind::LParen)?;
        children.push(token_segment(lp, SegmentType::LParen));
        children.extend(eat_trivia_segments(ctx));

        if let Some(expr) = self.parse_expression(ctx) {
            children.push(expr);
        }
        children.extend(eat_trivia_segments(ctx));

        if let Some(as_kw) = ctx.eat_keyword("AS") {
            children.push(token_segment(as_kw, SegmentType::Keyword));
        }
        children.extend(eat_trivia_segments(ctx));

        // Data type
        if let Some(dt) = self.parse_data_type(ctx) {
            children.push(dt);
        }
        children.extend(eat_trivia_segments(ctx));

        if let Some(rp) = ctx.eat_kind(TokenKind::RParen) {
            children.push(token_segment(rp, SegmentType::RParen));
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::CastExpression,
            children,
        )))
    }

    fn parse_data_type(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();

        // Type name (may be multi-word like "DOUBLE PRECISION", "CHARACTER VARYING")
        let word = ctx.eat_kind(TokenKind::Word)?;
        children.push(token_segment(word, SegmentType::Keyword));

        // Additional type words
        loop {
            let save = ctx.save();
            let trivia = eat_trivia_segments(ctx);
            if let Some(t) = ctx.peek() {
                if t.kind == TokenKind::Word && !is_clause_keyword(&t.text) {
                    children.extend(trivia);
                    let w = ctx.advance().unwrap();
                    children.push(token_segment(w, SegmentType::Keyword));
                    continue;
                }
            }
            ctx.restore(save);
            break;
        }

        // Optional (precision, scale)
        let save = ctx.save();
        let trivia = eat_trivia_segments(ctx);
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            children.extend(trivia);
            if let Some(params) = self.parse_paren_block(ctx) {
                children.push(params);
            }
        } else {
            ctx.restore(save);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::DataType,
            children,
        )))
    }

    // ── Identifiers & names ──────────────────────────────────────

    fn parse_identifier(&self, ctx: &mut ParseContext) -> Option<Segment> {
        match ctx.peek_kind()? {
            TokenKind::Word => {
                let token = ctx.advance().unwrap();
                Some(token_segment(token, SegmentType::Identifier))
            }
            TokenKind::QuotedIdentifier => {
                let token = ctx.advance().unwrap();
                Some(token_segment(token, SegmentType::QuotedIdentifier))
            }
            // @variable / @@variable (TSQL table variables, system variables)
            TokenKind::AtSign => {
                let token = ctx.advance().unwrap();
                Some(token_segment(token, SegmentType::Identifier))
            }
            _ => None,
        }
    }

    /// Parse a possibly qualified name: a, a.b, a.b.c
    fn parse_qualified_name(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let first = self.parse_identifier(ctx)?;

        let save = ctx.save();
        if ctx.peek_kind() == Some(TokenKind::Dot) {
            let mut children = vec![first];
            while ctx.peek_kind() == Some(TokenKind::Dot) {
                let dot = ctx.advance().unwrap();
                children.push(token_segment(dot, SegmentType::Dot));
                if let Some(part) = self.parse_identifier(ctx) {
                    children.push(part);
                } else {
                    // Star: schema.table.*
                    if ctx.peek_kind() == Some(TokenKind::Star) {
                        let star = ctx.advance().unwrap();
                        children.push(token_segment(star, SegmentType::Star));
                    }
                    break;
                }
            }
            Some(Segment::Node(NodeSegment::new(
                SegmentType::ColumnRef,
                children,
            )))
        } else {
            ctx.restore(save);
            Some(first)
        }
    }

    fn parse_name_or_function(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let name = self.parse_qualified_name(ctx)?;

        // Check for function call: name(...)
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            let mut children = vec![name];
            if let Some(args) = self.parse_paren_block(ctx) {
                children.push(args);
            }
            let func = Segment::Node(NodeSegment::new(SegmentType::FunctionCall, children));

            // Check for OVER clause (window function)
            let save = ctx.save();
            let trivia = eat_trivia_segments(ctx);
            if ctx.peek_keyword("OVER") {
                let mut win_children = vec![func];
                win_children.extend(trivia);
                if let Some(over) = self.parse_over_clause(ctx) {
                    win_children.push(over);
                }
                return Some(Segment::Node(NodeSegment::new(
                    SegmentType::WindowExpression,
                    win_children,
                )));
            }
            ctx.restore(save);

            return Some(func);
        }

        Some(name)
    }

    /// Parse OVER clause: `OVER (PARTITION BY ... ORDER BY ... ROWS/RANGE ...)`
    /// or `OVER window_name`
    fn parse_over_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let over_kw = ctx.eat_keyword("OVER")?;
        children.push(token_segment(over_kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // OVER window_name (named window reference, no parens)
        if ctx.peek_kind() != Some(TokenKind::LParen) {
            if let Some(name) = self.parse_identifier(ctx) {
                children.push(name);
            }
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::OverClause,
                children,
            )));
        }

        // OVER ( ... )
        let lp = ctx.eat_kind(TokenKind::LParen)?;
        children.push(token_segment(lp, SegmentType::LParen));
        children.extend(eat_trivia_segments(ctx));

        // PARTITION BY ...
        if ctx.peek_keyword("PARTITION") {
            if let Some(pb) = self.parse_partition_by_clause(ctx) {
                children.push(pb);
                children.extend(eat_trivia_segments(ctx));
            }
        }

        // ORDER BY ...
        if ctx.peek_keywords(&["ORDER", "BY"]) {
            if let Some(ob) = self.parse_window_order_by(ctx) {
                children.push(ob);
                children.extend(eat_trivia_segments(ctx));
            }
        }

        // Window frame: ROWS / RANGE / GROUPS
        if ctx.peek_keyword("ROWS") || ctx.peek_keyword("RANGE") || ctx.peek_keyword("GROUPS") {
            if let Some(frame) = self.parse_window_frame_clause(ctx) {
                children.push(frame);
                children.extend(eat_trivia_segments(ctx));
            }
        }

        if let Some(rp) = ctx.eat_kind(TokenKind::RParen) {
            children.push(token_segment(rp, SegmentType::RParen));
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::OverClause,
            children,
        )))
    }

    /// Parse PARTITION BY expr, expr, ...
    fn parse_partition_by_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let part_kw = ctx.eat_keyword("PARTITION")?;
        children.push(token_segment(part_kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        if let Some(by_kw) = ctx.eat_keyword("BY") {
            children.push(token_segment(by_kw, SegmentType::Keyword));
        }
        children.extend(eat_trivia_segments(ctx));

        // Comma-separated expressions
        parse_comma_separated(ctx, &mut children, |c| self.parse_expression(c));

        Some(Segment::Node(NodeSegment::new(
            SegmentType::PartitionByClause,
            children,
        )))
    }

    /// Parse ORDER BY inside a window spec (reuses expression parsing).
    fn parse_window_order_by(&self, ctx: &mut ParseContext) -> Option<Segment> {
        // Delegate to the existing ORDER BY parser
        self.parse_order_by_clause(ctx)
    }

    /// Parse window frame: ROWS/RANGE/GROUPS frame_spec
    fn parse_window_frame_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();

        // ROWS | RANGE | GROUPS
        let frame_kw = ctx.advance()?;
        children.push(token_segment(frame_kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // BETWEEN ... AND ... or single bound
        if ctx.peek_keyword("BETWEEN") {
            let bw_kw = ctx.advance().unwrap();
            children.push(token_segment(bw_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));

            // start bound
            eat_frame_bound(ctx, &mut children);
            children.extend(eat_trivia_segments(ctx));

            // AND
            if let Some(and_kw) = ctx.eat_keyword("AND") {
                children.push(token_segment(and_kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
            }

            // end bound
            eat_frame_bound(ctx, &mut children);
        } else {
            // Single bound (e.g. ROWS UNBOUNDED PRECEDING)
            eat_frame_bound(ctx, &mut children);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::WindowFrameClause,
            children,
        )))
    }

    // ── Simple statement ─────────────────────────────────────────

    /// Parse a simple statement (USE, TRUNCATE, etc.) by consuming until end.
    fn parse_simple_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.advance()?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));
        self.consume_until_statement_end(ctx, &mut children);
        Some(Segment::Node(NodeSegment::new(
            SegmentType::Statement,
            children,
        )))
    }

    // ── Statement boundary helpers ──────────────────────────────

    /// Check if current token looks like the start of a new statement.
    fn peek_statement_start(&self, ctx: &ParseContext) -> bool {
        if let Some(t) = ctx.peek_non_trivia() {
            if t.kind == TokenKind::Word {
                return self
                    .statement_keywords()
                    .iter()
                    .any(|kw| t.text.eq_ignore_ascii_case(kw));
            }
        }
        false
    }

    /// Consume tokens until semicolon, EOF, or start of new statement.
    /// Consume tokens until semicolon, EOF, or start of a new statement.
    /// Tracks paren depth so that keywords inside subqueries (e.g. `SELECT`
    /// within `(SELECT ...)`) do not cause premature termination.
    fn consume_until_statement_end(&self, ctx: &mut ParseContext, children: &mut Vec<Segment>) {
        let mut paren_depth = 0u32;
        while !ctx.at_eof() {
            match ctx.peek_kind() {
                Some(TokenKind::Semicolon) if paren_depth == 0 => break,
                Some(TokenKind::LParen) => {
                    paren_depth += 1;
                    let token = ctx.advance().unwrap();
                    children.push(any_token_segment(token));
                }
                Some(TokenKind::RParen) => {
                    paren_depth = paren_depth.saturating_sub(1);
                    let token = ctx.advance().unwrap();
                    children.push(any_token_segment(token));
                }
                _ => {
                    if paren_depth == 0 && self.peek_statement_start(ctx) {
                        break;
                    }
                    let token = ctx.advance().unwrap();
                    children.push(any_token_segment(token));
                }
            }
        }
    }

    // ── Utility parsing ──────────────────────────────────────────

    /// Parse parenthesized content as a simple block.
    fn parse_paren_block(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let lp = ctx.eat_kind(TokenKind::LParen)?;
        children.push(token_segment(lp, SegmentType::LParen));

        let mut depth = 1u32;
        while depth > 0 && !ctx.at_eof() {
            match ctx.peek_kind() {
                Some(TokenKind::LParen) => {
                    depth += 1;
                    let token = ctx.advance().unwrap();
                    children.push(any_token_segment(token));
                }
                Some(TokenKind::RParen) => {
                    depth -= 1;
                    let token = ctx.advance().unwrap();
                    if depth == 0 {
                        children.push(token_segment(token, SegmentType::RParen));
                    } else {
                        children.push(any_token_segment(token));
                    }
                }
                _ => {
                    let token = ctx.advance().unwrap();
                    children.push(any_token_segment(token));
                }
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::ParenExpression,
            children,
        )))
    }

    /// Consume tokens until the end of a statement.
    ///
    /// The ANSI default tracks paren and CASE/END depth, stopping at
    /// semicolons or EOF.  TSQL overrides this to additionally track
    /// BEGIN/END blocks and the GO batch separator.
    fn consume_until_end(&self, ctx: &mut ParseContext, children: &mut Vec<Segment>) {
        let mut paren_depth = 0u32;
        let mut case_depth = 0u32;
        while !ctx.at_eof() {
            match ctx.peek_kind() {
                Some(TokenKind::Semicolon) if paren_depth == 0 => break,
                Some(TokenKind::LParen) => {
                    paren_depth += 1;
                    let token = ctx.advance().unwrap();
                    children.push(any_token_segment(token));
                }
                Some(TokenKind::RParen) => {
                    paren_depth = paren_depth.saturating_sub(1);
                    let token = ctx.advance().unwrap();
                    children.push(any_token_segment(token));
                }
                _ => {
                    let t = ctx.peek().unwrap();
                    if t.kind == TokenKind::Word {
                        if t.text.eq_ignore_ascii_case("CASE") {
                            case_depth += 1;
                        } else if t.text.eq_ignore_ascii_case("END") && case_depth > 0 {
                            case_depth -= 1;
                        }
                    }
                    let token = ctx.advance().unwrap();
                    children.push(any_token_segment(token));
                }
            }
        }
    }
}

// ── Free helper functions ────────────────────────────────────────

pub fn token_segment(token: &Token, segment_type: SegmentType) -> Segment {
    Segment::Token(TokenSegment {
        token: token.clone(),
        segment_type,
    })
}

pub fn any_token_segment(token: &Token) -> Segment {
    let st = match token.kind {
        TokenKind::Whitespace => SegmentType::Whitespace,
        TokenKind::Newline => SegmentType::Newline,
        TokenKind::LineComment => SegmentType::LineComment,
        TokenKind::BlockComment => SegmentType::BlockComment,
        TokenKind::Comma => SegmentType::Comma,
        TokenKind::Dot => SegmentType::Dot,
        TokenKind::Semicolon => SegmentType::Semicolon,
        TokenKind::Star => SegmentType::Star,
        TokenKind::LParen => SegmentType::LParen,
        TokenKind::RParen => SegmentType::RParen,
        TokenKind::NumberLiteral => SegmentType::NumericLiteral,
        TokenKind::StringLiteral => SegmentType::StringLiteral,
        TokenKind::Word => SegmentType::Keyword,
        TokenKind::QuotedIdentifier => SegmentType::QuotedIdentifier,
        _ => SegmentType::Operator,
    };
    token_segment(token, st)
}

pub fn unparsable_token(token: &Token) -> Segment {
    Segment::Token(TokenSegment {
        token: token.clone(),
        segment_type: SegmentType::Unparsable,
    })
}

pub fn eat_trivia_segments(ctx: &mut ParseContext) -> Vec<Segment> {
    ctx.eat_trivia()
        .into_iter()
        .map(any_token_segment)
        .collect()
}

/// Parse statements until `is_end` returns true, consuming unparsable tokens as fallback.
pub fn parse_statement_list(
    grammar: &dyn Grammar,
    ctx: &mut ParseContext,
    children: &mut Vec<Segment>,
    is_end: impl Fn(&ParseContext) -> bool,
) {
    loop {
        children.extend(eat_trivia_segments(ctx));
        if ctx.at_eof() || is_end(ctx) {
            break;
        }
        if let Some(stmt) = grammar.parse_statement(ctx) {
            children.push(stmt);
        } else {
            children.extend(eat_trivia_segments(ctx));
            if !ctx.at_eof() && !is_end(ctx) {
                if let Some(token) = ctx.advance() {
                    children.push(unparsable_token(token));
                }
            }
        }
    }
}

/// Parse a comma-separated list of items, appending them to `children`.
///
/// Parses the first item, then loops: save → eat trivia → comma? → parse next.
/// If no comma is found, restores to before the trivia and returns.
pub fn parse_comma_separated(
    ctx: &mut ParseContext,
    children: &mut Vec<Segment>,
    mut parse_one: impl FnMut(&mut ParseContext) -> Option<Segment>,
) {
    if let Some(item) = parse_one(ctx) {
        children.push(item);
    }
    loop {
        let save = ctx.save();
        let trivia = eat_trivia_segments(ctx);
        if let Some(comma) = ctx.eat_kind(TokenKind::Comma) {
            children.extend(trivia);
            children.push(token_segment(comma, SegmentType::Comma));
            children.extend(eat_trivia_segments(ctx));
            if let Some(item) = parse_one(ctx) {
                children.push(item);
            }
        } else {
            ctx.restore(save);
            break;
        }
    }
}

fn peek_join_keyword(ctx: &ParseContext) -> bool {
    if let Some(t) = ctx.peek_non_trivia() {
        if t.kind == TokenKind::Word {
            return is_join_keyword(&t.text);
        }
    }
    false
}

/// Consume a frame bound: UNBOUNDED PRECEDING/FOLLOWING, CURRENT ROW, N PRECEDING/FOLLOWING
fn eat_frame_bound(ctx: &mut ParseContext, children: &mut Vec<Segment>) {
    // CURRENT ROW
    if ctx.peek_keyword("CURRENT") {
        let kw = ctx.advance().unwrap();
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("ROW") {
            let row_kw = ctx.advance().unwrap();
            children.push(token_segment(row_kw, SegmentType::Keyword));
        }
        return;
    }

    // UNBOUNDED PRECEDING/FOLLOWING
    if ctx.peek_keyword("UNBOUNDED") {
        let kw = ctx.advance().unwrap();
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("PRECEDING") || ctx.peek_keyword("FOLLOWING") {
            let dir = ctx.advance().unwrap();
            children.push(token_segment(dir, SegmentType::Keyword));
        }
        return;
    }

    // N PRECEDING/FOLLOWING
    if ctx.peek_kind() == Some(TokenKind::NumberLiteral) {
        let num = ctx.advance().unwrap();
        children.push(token_segment(num, SegmentType::NumericLiteral));
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("PRECEDING") || ctx.peek_keyword("FOLLOWING") {
            let dir = ctx.advance().unwrap();
            children.push(token_segment(dir, SegmentType::Keyword));
        }
    }
}

/// Sorted list of keywords that must NOT be consumed as implicit aliases.
const CLAUSE_KEYWORDS: &[&str] = &[
    "ALTER",
    "AND",
    "AS",
    "BEGIN",
    "BETWEEN",
    "BREAK",
    "CASE",
    "CATCH",
    "CLOSE",
    "COMMIT",
    "CONTINUE",
    "CREATE",
    "CROSS",
    "CURSOR",
    "DEALLOCATE",
    "DECLARE",
    "DELETE",
    "DROP",
    "ELSE",
    "END",
    "EXCEPT",
    "EXEC",
    "EXECUTE",
    "EXISTS",
    "FETCH",
    "FOR",
    "FROM",
    "FULL",
    "GO",
    "GOTO",
    "GROUP",
    "HAVING",
    "IF",
    "IN",
    "INNER",
    "INSERT",
    "INTERSECT",
    "INTO",
    "IS",
    "JOIN",
    "LEFT",
    "LIKE",
    "LIMIT",
    "MERGE",
    "NEXT",
    "NOT",
    "OFFSET",
    "ON",
    "OPEN",
    "OR",
    "ORDER",
    "OUTPUT",
    "OVER",
    "PARTITION",
    "PRINT",
    "RAISERROR",
    "RETURN",
    "RETURNING",
    "RIGHT",
    "ROLLBACK",
    "SELECT",
    "SET",
    "TABLE",
    "THEN",
    "THROW",
    "TRUNCATE",
    "TRY",
    "UNION",
    "UPDATE",
    "USING",
    "VALUES",
    "WHEN",
    "WHERE",
    "WHILE",
    "WITH",
];

/// Case-insensitive binary search on a sorted uppercase keyword list.
/// Zero allocations — compares byte-by-byte.
fn binary_search_keyword(list: &[&str], word: &str) -> bool {
    list.binary_search_by(|kw| {
        kw.as_bytes()
            .iter()
            .copied()
            .cmp(word.as_bytes().iter().map(|b| b.to_ascii_uppercase()))
    })
    .is_ok()
}

pub fn is_clause_keyword(word: &str) -> bool {
    binary_search_keyword(CLAUSE_KEYWORDS, word)
}

const JOIN_KEYWORDS: &[&str] = &["CROSS", "FULL", "INNER", "JOIN", "LEFT", "RIGHT"];

pub fn is_join_keyword(word: &str) -> bool {
    binary_search_keyword(JOIN_KEYWORDS, word) || word.eq_ignore_ascii_case("CROSS")
}
