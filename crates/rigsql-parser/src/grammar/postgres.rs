use rigsql_core::{NodeSegment, Segment, SegmentType, TokenKind};

use crate::context::ParseContext;

use super::ansi::ANSI_STATEMENT_KEYWORDS;
use super::{eat_trivia_segments, parse_comma_separated, token_segment, Grammar};

/// PostgreSQL grammar — extends ANSI with PostgreSQL-specific syntax.
pub struct PostgresGrammar;

impl Grammar for PostgresGrammar {
    fn statement_keywords(&self) -> &[&str] {
        ANSI_STATEMENT_KEYWORDS
    }

    fn dispatch_statement(&self, ctx: &mut ParseContext) -> Option<Segment> {
        self.dispatch_ansi_statement(ctx)
    }

    // ── Override: SELECT clause to support DISTINCT ON ────────────

    fn parse_select_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();

        let kw = ctx.eat_keyword("SELECT")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // DISTINCT ON (expr, ...) or DISTINCT or ALL
        if ctx.peek_keyword("DISTINCT") {
            let distinct_kw = ctx.advance().unwrap();
            children.push(token_segment(distinct_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));

            // PostgreSQL: DISTINCT ON (col1, col2, ...)
            if ctx.peek_keyword("ON") {
                let on_kw = ctx.advance().unwrap();
                children.push(token_segment(on_kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));

                if ctx.peek_kind() == Some(TokenKind::LParen) {
                    if let Some(cols) = self.parse_paren_block(ctx) {
                        children.push(cols);
                    }
                }
                children.extend(eat_trivia_segments(ctx));
            }
        } else if ctx.peek_keyword("ALL") {
            let all_kw = ctx.advance().unwrap();
            children.push(token_segment(all_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
        }

        // Select targets (comma-separated expressions)
        parse_comma_separated(ctx, &mut children, |c| self.parse_select_target(c));

        Some(Segment::Node(NodeSegment::new(
            SegmentType::SelectClause,
            children,
        )))
    }

    // ── Override: INSERT to support ON CONFLICT and RETURNING ──────

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

        // ON CONFLICT clause (PostgreSQL upsert)
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("ON") {
            if let Some(oc) = self.parse_on_conflict_clause(ctx) {
                children.push(oc);
            }
        }

        // RETURNING clause
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("RETURNING") {
            if let Some(ret) = self.parse_returning_clause(ctx) {
                children.push(ret);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::InsertStatement,
            children,
        )))
    }

    // ── Override: UPDATE to support RETURNING ──────────────────────

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

        // RETURNING clause
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("RETURNING") {
            if let Some(ret) = self.parse_returning_clause(ctx) {
                children.push(ret);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::UpdateStatement,
            children,
        )))
    }

    // ── Override: DELETE to support RETURNING ──────────────────────

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

        // RETURNING clause
        children.extend(eat_trivia_segments(ctx));
        if ctx.peek_keyword("RETURNING") {
            if let Some(ret) = self.parse_returning_clause(ctx) {
                children.push(ret);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::DeleteStatement,
            children,
        )))
    }

    // ── Override: unary expression to add :: postfix cast ──────────

    fn parse_unary_expression(&self, ctx: &mut ParseContext) -> Option<Segment> {
        // Handle unary +/- prefix
        if let Some(kind) = ctx.peek_kind() {
            if matches!(kind, TokenKind::Plus | TokenKind::Minus) {
                let op = ctx.advance().unwrap();
                let mut children = vec![token_segment(op, SegmentType::ArithmeticOperator)];
                children.extend(eat_trivia_segments(ctx));
                if let Some(expr) = self.parse_primary_expression(ctx) {
                    children.push(expr);
                }
                let base = Segment::Node(NodeSegment::new(SegmentType::UnaryExpression, children));
                return Some(self.parse_postfix(ctx, base));
            }
        }
        let base = self.parse_primary_expression(ctx)?;
        Some(self.parse_postfix(ctx, base))
    }
}

// ── PostgreSQL-specific parsing methods ────────────────────────────

impl PostgresGrammar {
    /// Parse postfix operators: `::type` cast and `[idx]` array subscript.
    /// Loops to handle chaining: `arr[1]::text`, `col::int[]`.
    fn parse_postfix(&self, ctx: &mut ParseContext, mut expr: Segment) -> Segment {
        loop {
            // Peek ahead past trivia without consuming, to avoid Vec allocation
            // on the common path where no postfix operator follows.
            let save = ctx.save();
            eat_trivia_segments(ctx);
            let next = ctx.peek_kind();
            ctx.restore(save);

            if next != Some(TokenKind::ColonColon) && next != Some(TokenKind::LBracket) {
                break;
            }

            let trivia = eat_trivia_segments(ctx);

            // :: type cast
            if ctx.peek_kind() == Some(TokenKind::ColonColon) {
                let cc = ctx.advance().unwrap();
                let mut children = vec![expr];
                children.extend(trivia);
                children.push(token_segment(cc, SegmentType::Operator));
                children.extend(eat_trivia_segments(ctx));
                if let Some(dt) = self.parse_data_type(ctx) {
                    children.push(dt);
                }
                // Handle array type suffix: ::int[]
                let save2 = ctx.save();
                if ctx.peek_kind() == Some(TokenKind::LBracket) {
                    let lb = ctx.advance().unwrap();
                    if ctx.peek_kind() == Some(TokenKind::RBracket) {
                        let rb = ctx.advance().unwrap();
                        children.push(token_segment(lb, SegmentType::Operator));
                        children.push(token_segment(rb, SegmentType::Operator));
                    } else {
                        ctx.restore(save2);
                    }
                }
                expr = Segment::Node(NodeSegment::new(SegmentType::TypeCastExpression, children));
                continue;
            }

            // [idx] array subscript
            if ctx.peek_kind() == Some(TokenKind::LBracket) {
                let lb = ctx.advance().unwrap();
                let mut children = vec![expr];
                children.extend(trivia);
                children.push(token_segment(lb, SegmentType::Operator));
                children.extend(eat_trivia_segments(ctx));
                if let Some(idx) = self.parse_expression(ctx) {
                    children.push(idx);
                }
                children.extend(eat_trivia_segments(ctx));
                if let Some(rb) = ctx.eat_kind(TokenKind::RBracket) {
                    children.push(token_segment(rb, SegmentType::Operator));
                }
                expr = Segment::Node(NodeSegment::new(
                    SegmentType::ArrayAccessExpression,
                    children,
                ));
                continue;
            }

            // Unreachable: we only enter the loop body when next is :: or [
            unreachable!();
        }
        expr
    }

    /// Parse RETURNING clause: `RETURNING expr, expr, ...`
    fn parse_returning_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("RETURNING")?;
        children.push(token_segment(kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        parse_comma_separated(ctx, &mut children, |c| self.parse_select_target(c));

        Some(Segment::Node(NodeSegment::new(
            SegmentType::ReturningClause,
            children,
        )))
    }

    /// Parse ON CONFLICT clause:
    /// `ON CONFLICT (col, ...) DO NOTHING`
    /// `ON CONFLICT (col, ...) DO UPDATE SET col = expr, ...`
    fn parse_on_conflict_clause(&self, ctx: &mut ParseContext) -> Option<Segment> {
        let save = ctx.save();
        let mut children = Vec::new();

        let on_kw = ctx.eat_keyword("ON")?;
        let trivia = eat_trivia_segments(ctx);
        if !ctx.peek_keyword("CONFLICT") {
            ctx.restore(save);
            return None;
        }
        children.push(token_segment(on_kw, SegmentType::Keyword));
        children.extend(trivia);

        let conflict_kw = ctx.advance().unwrap();
        children.push(token_segment(conflict_kw, SegmentType::Keyword));
        children.extend(eat_trivia_segments(ctx));

        // Optional conflict target: (column, ...) or ON CONSTRAINT name
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            if let Some(cols) = self.parse_paren_block(ctx) {
                children.push(cols);
            }
            children.extend(eat_trivia_segments(ctx));
        } else if ctx.peek_keyword("ON") {
            // ON CONSTRAINT constraint_name
            let on2 = ctx.advance().unwrap();
            children.push(token_segment(on2, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));
            if ctx.peek_keyword("CONSTRAINT") {
                let cons_kw = ctx.advance().unwrap();
                children.push(token_segment(cons_kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));
                if let Some(name) = self.parse_identifier(ctx) {
                    children.push(name);
                }
                children.extend(eat_trivia_segments(ctx));
            }
        }

        // WHERE clause for conflict target (partial index)
        if ctx.peek_keyword("WHERE") {
            if let Some(wh) = self.parse_where_clause(ctx) {
                children.push(wh);
                children.extend(eat_trivia_segments(ctx));
            }
        }

        // DO NOTHING or DO UPDATE SET ...
        if ctx.peek_keyword("DO") {
            let do_kw = ctx.advance().unwrap();
            children.push(token_segment(do_kw, SegmentType::Keyword));
            children.extend(eat_trivia_segments(ctx));

            if ctx.peek_keyword("NOTHING") {
                let nothing_kw = ctx.advance().unwrap();
                children.push(token_segment(nothing_kw, SegmentType::Keyword));
            } else if ctx.peek_keyword("UPDATE") {
                let update_kw = ctx.advance().unwrap();
                children.push(token_segment(update_kw, SegmentType::Keyword));
                children.extend(eat_trivia_segments(ctx));

                // SET clause
                if ctx.peek_keyword("SET") {
                    if let Some(set) = self.parse_set_clause(ctx) {
                        children.push(set);
                    }
                }

                // WHERE clause (for DO UPDATE)
                children.extend(eat_trivia_segments(ctx));
                if ctx.peek_keyword("WHERE") {
                    if let Some(wh) = self.parse_where_clause(ctx) {
                        children.push(wh);
                    }
                }
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::OnConflictClause,
            children,
        )))
    }
}
