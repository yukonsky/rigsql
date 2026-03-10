use rigsql_core::{
    NodeSegment, Segment, SegmentType, Token, TokenKind, TokenSegment,
};

use crate::context::ParseContext;

/// Grammar provides methods to parse SQL constructs into CST segments.
///
/// Each parse method returns `Option<Segment>` — `None` means the
/// construct was not found at the current position, and the cursor
/// is left unchanged (backtracking).
pub struct Grammar;

impl Grammar {
    // ── Top-level ────────────────────────────────────────────────

    /// Parse a complete SQL file: zero or more statements.
    pub fn parse_file<'a>(ctx: &mut ParseContext<'a>) -> Segment {
        let mut children = Vec::new();
        while !ctx.at_eof() {
            children.extend(Self::eat_trivia_segments(ctx));
            if ctx.at_eof() {
                break;
            }
            if let Some(stmt) = Self::parse_statement(ctx) {
                children.push(stmt);
            } else {
                // Consume unparsable token to avoid infinite loop
                children.extend(Self::eat_trivia_segments(ctx));
                if !ctx.at_eof() {
                    if let Some(token) = ctx.advance() {
                        children.push(Self::unparsable_token(token));
                    }
                }
            }
        }
        Segment::Node(NodeSegment::new(SegmentType::File, children))
    }

    /// Parse a single statement (terminated by `;` or EOF).
    pub fn parse_statement<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let save = ctx.save();
        let mut children = Vec::new();
        children.extend(Self::eat_trivia_segments(ctx));

        let inner = if ctx.peek_keyword("SELECT") || ctx.peek_keyword("WITH") {
            Self::parse_select_statement(ctx)
        } else if ctx.peek_keyword("INSERT") {
            Self::parse_insert_statement(ctx)
        } else if ctx.peek_keyword("UPDATE") {
            Self::parse_update_statement(ctx)
        } else if ctx.peek_keyword("DELETE") {
            Self::parse_delete_statement(ctx)
        } else if ctx.peek_keyword("CREATE") {
            Self::parse_create_statement(ctx)
        } else if ctx.peek_keyword("DROP") {
            Self::parse_drop_statement(ctx)
        } else if ctx.peek_keyword("ALTER") {
            Self::parse_alter_statement(ctx)
        } else {
            None
        };

        match inner {
            Some(stmt_seg) => {
                children.push(stmt_seg);
                // Optional trailing semicolon
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(semi) = ctx.eat_kind(TokenKind::Semicolon) {
                    children.push(Self::token_segment(semi, SegmentType::Semicolon));
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

    fn parse_select_statement<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();

        // WITH clause (optional)
        if ctx.peek_keyword("WITH") {
            if let Some(with) = Self::parse_with_clause(ctx) {
                children.push(with);
                children.extend(Self::eat_trivia_segments(ctx));
            }
        }

        // SELECT clause (required)
        let select = Self::parse_select_clause(ctx)?;
        children.push(select);

        // FROM clause (optional)
        children.extend(Self::eat_trivia_segments(ctx));
        if ctx.peek_keyword("FROM") {
            if let Some(from) = Self::parse_from_clause(ctx) {
                children.push(from);
            }
        }

        // WHERE clause (optional)
        children.extend(Self::eat_trivia_segments(ctx));
        if ctx.peek_keyword("WHERE") {
            if let Some(wh) = Self::parse_where_clause(ctx) {
                children.push(wh);
            }
        }

        // GROUP BY clause (optional)
        children.extend(Self::eat_trivia_segments(ctx));
        if ctx.peek_keywords(&["GROUP", "BY"]) {
            if let Some(gb) = Self::parse_group_by_clause(ctx) {
                children.push(gb);
            }
        }

        // HAVING clause (optional)
        children.extend(Self::eat_trivia_segments(ctx));
        if ctx.peek_keyword("HAVING") {
            if let Some(hav) = Self::parse_having_clause(ctx) {
                children.push(hav);
            }
        }

        // ORDER BY clause (optional)
        children.extend(Self::eat_trivia_segments(ctx));
        if ctx.peek_keywords(&["ORDER", "BY"]) {
            if let Some(ob) = Self::parse_order_by_clause(ctx) {
                children.push(ob);
            }
        }

        // LIMIT clause (optional)
        children.extend(Self::eat_trivia_segments(ctx));
        if ctx.peek_keyword("LIMIT") {
            if let Some(lim) = Self::parse_limit_clause(ctx) {
                children.push(lim);
            }
        }

        // OFFSET clause (optional)
        children.extend(Self::eat_trivia_segments(ctx));
        if ctx.peek_keyword("OFFSET") {
            if let Some(off) = Self::parse_offset_clause(ctx) {
                children.push(off);
            }
        }

        // UNION / INTERSECT / EXCEPT (optional, recursive)
        children.extend(Self::eat_trivia_segments(ctx));
        if ctx.peek_keyword("UNION")
            || ctx.peek_keyword("INTERSECT")
            || ctx.peek_keyword("EXCEPT")
        {
            if let Some(set_op) = Self::parse_set_operation(ctx) {
                children.push(set_op);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::SelectStatement,
            children,
        )))
    }

    fn parse_select_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();

        let kw = ctx.eat_keyword("SELECT")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));

        children.extend(Self::eat_trivia_segments(ctx));

        // DISTINCT / ALL (optional)
        if ctx.peek_keyword("DISTINCT") || ctx.peek_keyword("ALL") {
            if let Some(token) = ctx.advance() {
                children.push(Self::token_segment(token, SegmentType::Keyword));
                children.extend(Self::eat_trivia_segments(ctx));
            }
        }

        // TOP (N) / TOP N (TSQL)
        if ctx.peek_keyword("TOP") {
            if let Some(top_kw) = ctx.advance() {
                children.push(Self::token_segment(top_kw, SegmentType::Keyword));
                children.extend(Self::eat_trivia_segments(ctx));
                // TOP (expr) or TOP N
                if let Some(lparen) = ctx.eat_kind(TokenKind::LParen) {
                    children.push(Self::token_segment(lparen, SegmentType::LParen));
                    children.extend(Self::eat_trivia_segments(ctx));
                    if let Some(expr) = Self::parse_expression(ctx) {
                        children.push(expr);
                    }
                    children.extend(Self::eat_trivia_segments(ctx));
                    if let Some(rparen) = ctx.eat_kind(TokenKind::RParen) {
                        children.push(Self::token_segment(rparen, SegmentType::RParen));
                    }
                } else if let Some(num) = ctx.eat_kind(TokenKind::NumberLiteral) {
                    children.push(Self::token_segment(num, SegmentType::Literal));
                }
                children.extend(Self::eat_trivia_segments(ctx));
            }
        }

        // Select targets (comma-separated expressions)
        if let Some(expr) = Self::parse_select_target(ctx) {
            children.push(expr);
        }
        loop {
            children.extend(Self::eat_trivia_segments(ctx));
            if let Some(comma) = ctx.eat_kind(TokenKind::Comma) {
                children.push(Self::token_segment(comma, SegmentType::Comma));
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(expr) = Self::parse_select_target(ctx) {
                    children.push(expr);
                }
            } else {
                break;
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::SelectClause,
            children,
        )))
    }

    fn parse_select_target<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        // Parse an expression, optionally followed by alias (AS alias_name or just alias_name)
        let expr = Self::parse_expression(ctx)?;

        let save = ctx.save();
        let trivia = Self::eat_trivia_segments(ctx);

        // Check for alias: AS name, or just a word that's not a keyword
        if ctx.peek_keyword("AS") {
            let mut children = vec![expr];
            children.extend(trivia);
            let as_kw = ctx.advance().unwrap();
            children.push(Self::token_segment(as_kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
            if let Some(alias) = Self::parse_identifier(ctx) {
                children.push(alias);
            }
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::AliasExpression,
                children,
            )));
        }

        // Implicit alias: a bare word that's not a clause keyword
        if let Some(t) = ctx.peek_non_trivia() {
            if t.kind == TokenKind::Word && !Self::is_clause_keyword(&t.text) {
                let mut children = vec![expr];
                children.extend(trivia);
                if let Some(alias) = Self::parse_identifier(ctx) {
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

    fn parse_from_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("FROM")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // Table references (comma-separated)
        if let Some(tref) = Self::parse_table_reference(ctx) {
            children.push(tref);
        }
        loop {
            let save = ctx.save();
            let trivia = Self::eat_trivia_segments(ctx);
            if let Some(comma) = ctx.eat_kind(TokenKind::Comma) {
                children.extend(trivia);
                children.push(Self::token_segment(comma, SegmentType::Comma));
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(tref) = Self::parse_table_reference(ctx) {
                    children.push(tref);
                }
            } else {
                ctx.restore(save);
                break;
            }
        }

        // JOIN clauses
        loop {
            children.extend(Self::eat_trivia_segments(ctx));
            if Self::peek_join_keyword(ctx) {
                if let Some(join) = Self::parse_join_clause(ctx) {
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

    fn parse_table_reference<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let save = ctx.save();

        // Subquery in parens
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            if let Some(subq) = Self::parse_paren_subquery(ctx) {
                // Optional alias
                let save2 = ctx.save();
                let trivia = Self::eat_trivia_segments(ctx);
                if ctx.peek_keyword("AS")
                    || ctx
                        .peek_non_trivia()
                        .is_some_and(|t| t.kind == TokenKind::Word && !Self::is_clause_keyword(&t.text))
                {
                    let mut children = vec![subq];
                    children.extend(trivia);
                    if ctx.peek_keyword("AS") {
                        let kw = ctx.advance().unwrap();
                        children.push(Self::token_segment(kw, SegmentType::Keyword));
                        children.extend(Self::eat_trivia_segments(ctx));
                    }
                    if let Some(alias) = Self::parse_identifier(ctx) {
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
        let name = Self::parse_qualified_name(ctx);
        if name.is_none() {
            ctx.restore(save);
            return None;
        }
        let name = name.unwrap();

        // Optional alias
        let save2 = ctx.save();
        let trivia = Self::eat_trivia_segments(ctx);
        if ctx.peek_keyword("AS") {
            let mut children = vec![name];
            children.extend(trivia);
            let kw = ctx.advance().unwrap();
            children.push(Self::token_segment(kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
            if let Some(alias) = Self::parse_identifier(ctx) {
                children.push(alias);
            }
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::AliasExpression,
                children,
            )));
        }
        if let Some(t) = ctx.peek_non_trivia() {
            if t.kind == TokenKind::Word
                && !Self::is_clause_keyword(&t.text)
                && !Self::is_join_keyword(&t.text)
            {
                let mut children = vec![name];
                children.extend(trivia);
                if let Some(alias) = Self::parse_identifier(ctx) {
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

    fn parse_where_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("WHERE")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));
        if let Some(expr) = Self::parse_expression(ctx) {
            children.push(expr);
        }
        Some(Segment::Node(NodeSegment::new(
            SegmentType::WhereClause,
            children,
        )))
    }

    fn parse_group_by_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let group_kw = ctx.eat_keyword("GROUP")?;
        children.push(Self::token_segment(group_kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));
        let by_kw = ctx.eat_keyword("BY")?;
        children.push(Self::token_segment(by_kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // Comma-separated expressions
        if let Some(expr) = Self::parse_expression(ctx) {
            children.push(expr);
        }
        loop {
            let save = ctx.save();
            let trivia = Self::eat_trivia_segments(ctx);
            if let Some(comma) = ctx.eat_kind(TokenKind::Comma) {
                children.extend(trivia);
                children.push(Self::token_segment(comma, SegmentType::Comma));
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(expr) = Self::parse_expression(ctx) {
                    children.push(expr);
                }
            } else {
                ctx.restore(save);
                break;
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::GroupByClause,
            children,
        )))
    }

    fn parse_having_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("HAVING")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));
        if let Some(expr) = Self::parse_expression(ctx) {
            children.push(expr);
        }
        Some(Segment::Node(NodeSegment::new(
            SegmentType::HavingClause,
            children,
        )))
    }

    fn parse_order_by_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let order_kw = ctx.eat_keyword("ORDER")?;
        children.push(Self::token_segment(order_kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));
        let by_kw = ctx.eat_keyword("BY")?;
        children.push(Self::token_segment(by_kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // Comma-separated order expressions
        if let Some(expr) = Self::parse_order_expression(ctx) {
            children.push(expr);
        }
        loop {
            let save = ctx.save();
            let trivia = Self::eat_trivia_segments(ctx);
            if let Some(comma) = ctx.eat_kind(TokenKind::Comma) {
                children.extend(trivia);
                children.push(Self::token_segment(comma, SegmentType::Comma));
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(expr) = Self::parse_order_expression(ctx) {
                    children.push(expr);
                }
            } else {
                ctx.restore(save);
                break;
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::OrderByClause,
            children,
        )))
    }

    fn parse_order_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let expr = Self::parse_expression(ctx)?;
        children.push(expr);

        let save = ctx.save();
        let trivia = Self::eat_trivia_segments(ctx);
        if ctx.peek_keyword("ASC") || ctx.peek_keyword("DESC") {
            children.extend(trivia);
            let kw = ctx.advance().unwrap();
            children.push(Self::token_segment(kw, SegmentType::Keyword));
        } else {
            ctx.restore(save);
        }

        // NULLS FIRST / NULLS LAST
        let save = ctx.save();
        let trivia = Self::eat_trivia_segments(ctx);
        if ctx.peek_keyword("NULLS") {
            children.extend(trivia);
            let kw = ctx.advance().unwrap();
            children.push(Self::token_segment(kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
            if ctx.peek_keyword("FIRST") || ctx.peek_keyword("LAST") {
                let kw = ctx.advance().unwrap();
                children.push(Self::token_segment(kw, SegmentType::Keyword));
            }
        } else {
            ctx.restore(save);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::OrderByExpression,
            children,
        )))
    }

    fn parse_limit_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("LIMIT")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));
        if let Some(expr) = Self::parse_expression(ctx) {
            children.push(expr);
        }
        Some(Segment::Node(NodeSegment::new(
            SegmentType::LimitClause,
            children,
        )))
    }

    fn parse_offset_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("OFFSET")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));
        if let Some(expr) = Self::parse_expression(ctx) {
            children.push(expr);
        }
        Some(Segment::Node(NodeSegment::new(
            SegmentType::OffsetClause,
            children,
        )))
    }

    fn parse_with_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("WITH")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // RECURSIVE (optional)
        if ctx.peek_keyword("RECURSIVE") {
            let kw = ctx.advance().unwrap();
            children.push(Self::token_segment(kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
        }

        // CTE definitions (comma-separated)
        if let Some(cte) = Self::parse_cte_definition(ctx) {
            children.push(cte);
        }
        loop {
            let save = ctx.save();
            let trivia = Self::eat_trivia_segments(ctx);
            if let Some(comma) = ctx.eat_kind(TokenKind::Comma) {
                children.extend(trivia);
                children.push(Self::token_segment(comma, SegmentType::Comma));
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(cte) = Self::parse_cte_definition(ctx) {
                    children.push(cte);
                }
            } else {
                ctx.restore(save);
                break;
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::WithClause,
            children,
        )))
    }

    fn parse_cte_definition<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let name = Self::parse_identifier(ctx)?;
        children.push(name);
        children.extend(Self::eat_trivia_segments(ctx));

        // Optional column list
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            if let Some(cols) = Self::parse_paren_list(ctx) {
                children.push(cols);
                children.extend(Self::eat_trivia_segments(ctx));
            }
        }

        let as_kw = ctx.eat_keyword("AS")?;
        children.push(Self::token_segment(as_kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // ( subquery )
        if let Some(subq) = Self::parse_paren_subquery(ctx) {
            children.push(subq);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::CteDefinition,
            children,
        )))
    }

    fn parse_set_operation<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();

        // UNION / INTERSECT / EXCEPT
        let kw = ctx.advance()?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // ALL / DISTINCT (optional)
        if ctx.peek_keyword("ALL") || ctx.peek_keyword("DISTINCT") {
            let kw = ctx.advance().unwrap();
            children.push(Self::token_segment(kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
        }

        // Following select
        if let Some(sel) = Self::parse_select_statement(ctx) {
            children.push(sel);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::SelectStatement,
            children,
        )))
    }

    // ── JOIN ─────────────────────────────────────────────────────

    fn peek_join_keyword(ctx: &ParseContext) -> bool {
        if let Some(t) = ctx.peek_non_trivia() {
            if t.kind == TokenKind::Word {
                return Self::is_join_keyword(&t.text);
            }
        }
        false
    }

    fn parse_join_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();

        // Optional: INNER / LEFT / RIGHT / FULL / CROSS
        if ctx.peek_keyword("INNER")
            || ctx.peek_keyword("LEFT")
            || ctx.peek_keyword("RIGHT")
            || ctx.peek_keyword("FULL")
            || ctx.peek_keyword("CROSS")
        {
            let kw = ctx.advance().unwrap();
            children.push(Self::token_segment(kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));

            // Optional: OUTER
            if ctx.peek_keyword("OUTER") {
                let kw = ctx.advance().unwrap();
                children.push(Self::token_segment(kw, SegmentType::Keyword));
                children.extend(Self::eat_trivia_segments(ctx));
            }
        }

        let join_kw = ctx.eat_keyword("JOIN")?;
        children.push(Self::token_segment(join_kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // Table reference
        if let Some(tref) = Self::parse_table_reference(ctx) {
            children.push(tref);
        }

        // ON or USING
        children.extend(Self::eat_trivia_segments(ctx));
        if ctx.peek_keyword("ON") {
            let kw = ctx.advance().unwrap();
            let mut on_children = vec![Self::token_segment(kw, SegmentType::Keyword)];
            on_children.extend(Self::eat_trivia_segments(ctx));
            if let Some(expr) = Self::parse_expression(ctx) {
                on_children.push(expr);
            }
            children.push(Segment::Node(NodeSegment::new(
                SegmentType::OnClause,
                on_children,
            )));
        } else if ctx.peek_keyword("USING") {
            let kw = ctx.advance().unwrap();
            let mut using_children = vec![Self::token_segment(kw, SegmentType::Keyword)];
            using_children.extend(Self::eat_trivia_segments(ctx));
            if let Some(paren) = Self::parse_paren_list(ctx) {
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

    fn parse_insert_statement<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("INSERT")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        let into_kw = ctx.eat_keyword("INTO")?;
        children.push(Self::token_segment(into_kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // Table name
        if let Some(name) = Self::parse_qualified_name(ctx) {
            children.push(name);
        }
        children.extend(Self::eat_trivia_segments(ctx));

        // Optional column list
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            if let Some(cols) = Self::parse_paren_list(ctx) {
                children.push(cols);
                children.extend(Self::eat_trivia_segments(ctx));
            }
        }

        // VALUES or SELECT
        if ctx.peek_keyword("VALUES") {
            if let Some(vals) = Self::parse_values_clause(ctx) {
                children.push(vals);
            }
        } else if ctx.peek_keyword("SELECT") || ctx.peek_keyword("WITH") {
            if let Some(sel) = Self::parse_select_statement(ctx) {
                children.push(sel);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::InsertStatement,
            children,
        )))
    }

    fn parse_values_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("VALUES")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // Comma-separated (expr, expr, ...)
        if let Some(row) = Self::parse_paren_list(ctx) {
            children.push(row);
        }
        loop {
            let save = ctx.save();
            let trivia = Self::eat_trivia_segments(ctx);
            if let Some(comma) = ctx.eat_kind(TokenKind::Comma) {
                children.extend(trivia);
                children.push(Self::token_segment(comma, SegmentType::Comma));
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(row) = Self::parse_paren_list(ctx) {
                    children.push(row);
                }
            } else {
                ctx.restore(save);
                break;
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::ValuesClause,
            children,
        )))
    }

    // ── UPDATE ───────────────────────────────────────────────────

    fn parse_update_statement<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("UPDATE")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // Table name
        if let Some(name) = Self::parse_table_reference(ctx) {
            children.push(name);
        }
        children.extend(Self::eat_trivia_segments(ctx));

        // SET clause
        if ctx.peek_keyword("SET") {
            if let Some(set) = Self::parse_set_clause(ctx) {
                children.push(set);
            }
        }

        // WHERE clause
        children.extend(Self::eat_trivia_segments(ctx));
        if ctx.peek_keyword("WHERE") {
            if let Some(wh) = Self::parse_where_clause(ctx) {
                children.push(wh);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::UpdateStatement,
            children,
        )))
    }

    fn parse_set_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("SET")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // col = expr, ...
        if let Some(assign) = Self::parse_expression(ctx) {
            children.push(assign);
        }
        loop {
            let save = ctx.save();
            let trivia = Self::eat_trivia_segments(ctx);
            if let Some(comma) = ctx.eat_kind(TokenKind::Comma) {
                children.extend(trivia);
                children.push(Self::token_segment(comma, SegmentType::Comma));
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(assign) = Self::parse_expression(ctx) {
                    children.push(assign);
                }
            } else {
                ctx.restore(save);
                break;
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::SetClause,
            children,
        )))
    }

    // ── DELETE ────────────────────────────────────────────────────

    fn parse_delete_statement<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("DELETE")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // FROM
        if ctx.peek_keyword("FROM") {
            let from_kw = ctx.advance().unwrap();
            children.push(Self::token_segment(from_kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
        }

        // Table name
        if let Some(name) = Self::parse_qualified_name(ctx) {
            children.push(name);
        }

        // WHERE clause
        children.extend(Self::eat_trivia_segments(ctx));
        if ctx.peek_keyword("WHERE") {
            if let Some(wh) = Self::parse_where_clause(ctx) {
                children.push(wh);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::DeleteStatement,
            children,
        )))
    }

    // ── DDL ──────────────────────────────────────────────────────

    fn parse_create_statement<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("CREATE")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        if ctx.peek_keyword("TABLE") {
            return Self::parse_create_table_body(ctx, children);
        }

        // For other CREATE statements, consume until semicolon or EOF
        Self::consume_until_end(ctx, &mut children);
        Some(Segment::Node(NodeSegment::new(
            SegmentType::Statement,
            children,
        )))
    }

    fn parse_create_table_body<'a>(
        ctx: &mut ParseContext<'a>,
        mut children: Vec<Segment>,
    ) -> Option<Segment> {
        let kw = ctx.eat_keyword("TABLE")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // IF NOT EXISTS
        if ctx.peek_keyword("IF") {
            let kw = ctx.advance().unwrap();
            children.push(Self::token_segment(kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
            if let Some(kw) = ctx.eat_keyword("NOT") {
                children.push(Self::token_segment(kw, SegmentType::Keyword));
                children.extend(Self::eat_trivia_segments(ctx));
            }
            if let Some(kw) = ctx.eat_keyword("EXISTS") {
                children.push(Self::token_segment(kw, SegmentType::Keyword));
                children.extend(Self::eat_trivia_segments(ctx));
            }
        }

        // Table name
        if let Some(name) = Self::parse_qualified_name(ctx) {
            children.push(name);
        }
        children.extend(Self::eat_trivia_segments(ctx));

        // Column definitions in parens
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            if let Some(defs) = Self::parse_paren_block(ctx) {
                children.push(defs);
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::CreateTableStatement,
            children,
        )))
    }

    fn parse_drop_statement<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("DROP")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // Consume until semicolon
        Self::consume_until_end(ctx, &mut children);
        Some(Segment::Node(NodeSegment::new(
            SegmentType::DropStatement,
            children,
        )))
    }

    fn parse_alter_statement<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("ALTER")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        Self::consume_until_end(ctx, &mut children);
        Some(Segment::Node(NodeSegment::new(
            SegmentType::AlterTableStatement,
            children,
        )))
    }

    // ── Expression parsing ───────────────────────────────────────

    /// Parse an expression. This uses a simple precedence climbing approach.
    pub fn parse_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        Self::parse_or_expression(ctx)
    }

    fn parse_or_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut left = Self::parse_and_expression(ctx)?;
        loop {
            let save = ctx.save();
            let trivia = Self::eat_trivia_segments(ctx);
            if ctx.peek_keyword("OR") {
                let op = ctx.advance().unwrap();
                let mut children = vec![left];
                children.extend(trivia);
                children.push(Self::token_segment(op, SegmentType::Keyword));
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(right) = Self::parse_and_expression(ctx) {
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

    fn parse_and_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut left = Self::parse_not_expression(ctx)?;
        loop {
            let save = ctx.save();
            let trivia = Self::eat_trivia_segments(ctx);
            if ctx.peek_keyword("AND") {
                let op = ctx.advance().unwrap();
                let mut children = vec![left];
                children.extend(trivia);
                children.push(Self::token_segment(op, SegmentType::Keyword));
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(right) = Self::parse_not_expression(ctx) {
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

    fn parse_not_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        if ctx.peek_keyword("NOT") {
            let mut children = Vec::new();
            let kw = ctx.advance().unwrap();
            children.push(Self::token_segment(kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
            if let Some(expr) = Self::parse_not_expression(ctx) {
                children.push(expr);
            }
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::UnaryExpression,
                children,
            )));
        }
        Self::parse_comparison_expression(ctx)
    }

    fn parse_comparison_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let left = Self::parse_addition_expression(ctx)?;

        let save = ctx.save();
        let trivia = Self::eat_trivia_segments(ctx);

        // IS [NOT] NULL
        if ctx.peek_keyword("IS") {
            let is_kw = ctx.advance().unwrap();
            let mut children = vec![left];
            children.extend(trivia);
            children.push(Self::token_segment(is_kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
            if ctx.peek_keyword("NOT") {
                let not_kw = ctx.advance().unwrap();
                children.push(Self::token_segment(not_kw, SegmentType::Keyword));
                children.extend(Self::eat_trivia_segments(ctx));
            }
            if ctx.peek_keyword("NULL") {
                let null_kw = ctx.advance().unwrap();
                children.push(Self::token_segment(null_kw, SegmentType::Keyword));
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
            children.push(Self::token_segment(in_kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
            if ctx.peek_kind() == Some(TokenKind::LParen) {
                if let Some(list) = Self::parse_paren_block(ctx) {
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
            let not_trivia = Self::eat_trivia_segments(ctx);

            if ctx.peek_keyword("IN") {
                let in_kw = ctx.advance().unwrap();
                let mut children = vec![left];
                children.extend(trivia);
                children.push(Self::token_segment(not_kw, SegmentType::Keyword));
                children.extend(not_trivia);
                children.push(Self::token_segment(in_kw, SegmentType::Keyword));
                children.extend(Self::eat_trivia_segments(ctx));
                if ctx.peek_kind() == Some(TokenKind::LParen) {
                    if let Some(list) = Self::parse_paren_block(ctx) {
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
                children.push(Self::token_segment(not_kw, SegmentType::Keyword));
                children.extend(not_trivia);
                children.push(Self::token_segment(kw, SegmentType::Keyword));
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(lo) = Self::parse_addition_expression(ctx) {
                    children.push(lo);
                }
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(and_kw) = ctx.eat_keyword("AND") {
                    children.push(Self::token_segment(and_kw, SegmentType::Keyword));
                }
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(hi) = Self::parse_addition_expression(ctx) {
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
                children.push(Self::token_segment(not_kw, SegmentType::Keyword));
                children.extend(not_trivia);
                children.push(Self::token_segment(kw, SegmentType::Keyword));
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(pattern) = Self::parse_addition_expression(ctx) {
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
            children.push(Self::token_segment(kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
            if let Some(lo) = Self::parse_addition_expression(ctx) {
                children.push(lo);
            }
            children.extend(Self::eat_trivia_segments(ctx));
            if let Some(and_kw) = ctx.eat_keyword("AND") {
                children.push(Self::token_segment(and_kw, SegmentType::Keyword));
            }
            children.extend(Self::eat_trivia_segments(ctx));
            if let Some(hi) = Self::parse_addition_expression(ctx) {
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
            children.push(Self::token_segment(kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
            if let Some(pattern) = Self::parse_addition_expression(ctx) {
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
                children.push(Self::token_segment(op, SegmentType::ComparisonOperator));
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(right) = Self::parse_addition_expression(ctx) {
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

    fn parse_addition_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut left = Self::parse_multiplication_expression(ctx)?;
        loop {
            let save = ctx.save();
            let trivia = Self::eat_trivia_segments(ctx);
            if let Some(kind) = ctx.peek_kind() {
                if matches!(kind, TokenKind::Plus | TokenKind::Minus | TokenKind::Concat) {
                    let op = ctx.advance().unwrap();
                    let mut children = vec![left];
                    children.extend(trivia);
                    children.push(Self::token_segment(op, SegmentType::ArithmeticOperator));
                    children.extend(Self::eat_trivia_segments(ctx));
                    if let Some(right) = Self::parse_multiplication_expression(ctx) {
                        children.push(right);
                    }
                    left = Segment::Node(NodeSegment::new(
                        SegmentType::BinaryExpression,
                        children,
                    ));
                    continue;
                }
            }
            ctx.restore(save);
            break;
        }
        Some(left)
    }

    fn parse_multiplication_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut left = Self::parse_unary_expression(ctx)?;
        loop {
            let save = ctx.save();
            let trivia = Self::eat_trivia_segments(ctx);
            if let Some(kind) = ctx.peek_kind() {
                if matches!(kind, TokenKind::Star | TokenKind::Slash | TokenKind::Percent) {
                    let op = ctx.advance().unwrap();
                    let mut children = vec![left];
                    children.extend(trivia);
                    children.push(Self::token_segment(op, SegmentType::ArithmeticOperator));
                    children.extend(Self::eat_trivia_segments(ctx));
                    if let Some(right) = Self::parse_unary_expression(ctx) {
                        children.push(right);
                    }
                    left = Segment::Node(NodeSegment::new(
                        SegmentType::BinaryExpression,
                        children,
                    ));
                    continue;
                }
            }
            ctx.restore(save);
            break;
        }
        Some(left)
    }

    fn parse_unary_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        if let Some(kind) = ctx.peek_kind() {
            if matches!(kind, TokenKind::Plus | TokenKind::Minus) {
                let op = ctx.advance().unwrap();
                let mut children =
                    vec![Self::token_segment(op, SegmentType::ArithmeticOperator)];
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(expr) = Self::parse_primary_expression(ctx) {
                    children.push(expr);
                }
                return Some(Segment::Node(NodeSegment::new(
                    SegmentType::UnaryExpression,
                    children,
                )));
            }
        }
        Self::parse_primary_expression(ctx)
    }

    fn parse_primary_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        match ctx.peek_kind()? {
            // Parenthesized expression or subquery
            TokenKind::LParen => {
                // Check if it's a subquery
                let save = ctx.save();
                if let Some(subq) = Self::parse_paren_subquery(ctx) {
                    return Some(subq);
                }
                ctx.restore(save);
                Self::parse_paren_expression(ctx)
            }

            // Number literal
            TokenKind::NumberLiteral => {
                let token = ctx.advance().unwrap();
                Some(Self::token_segment(token, SegmentType::NumericLiteral))
            }

            // String literal
            TokenKind::StringLiteral => {
                let token = ctx.advance().unwrap();
                Some(Self::token_segment(token, SegmentType::StringLiteral))
            }

            // Star (e.g. SELECT *)
            TokenKind::Star => {
                let token = ctx.advance().unwrap();
                Some(Self::token_segment(token, SegmentType::Star))
            }

            // Placeholder
            TokenKind::Placeholder => {
                let token = ctx.advance().unwrap();
                Some(Self::token_segment(token, SegmentType::Literal))
            }

            // Quoted identifier
            TokenKind::QuotedIdentifier => {
                let token = ctx.advance().unwrap();
                Some(Self::token_segment(token, SegmentType::QuotedIdentifier))
            }

            // Word: keyword, function call, column ref, etc.
            TokenKind::Word => {
                let text = &ctx.peek().unwrap().text;

                if text.eq_ignore_ascii_case("CASE") {
                    return Self::parse_case_expression(ctx);
                }
                if text.eq_ignore_ascii_case("EXISTS") {
                    return Self::parse_exists_expression(ctx);
                }
                if text.eq_ignore_ascii_case("CAST") {
                    return Self::parse_cast_expression(ctx);
                }
                if text.eq_ignore_ascii_case("TRUE") || text.eq_ignore_ascii_case("FALSE") {
                    let token = ctx.advance().unwrap();
                    return Some(Self::token_segment(token, SegmentType::BooleanLiteral));
                }
                if text.eq_ignore_ascii_case("NULL") {
                    let token = ctx.advance().unwrap();
                    return Some(Self::token_segment(token, SegmentType::NullLiteral));
                }

                Self::parse_name_or_function(ctx)
            }

            // @ variable (SQL Server)
            TokenKind::AtSign => {
                let token = ctx.advance().unwrap();
                Some(Self::token_segment(token, SegmentType::Identifier))
            }

            _ => None,
        }
    }

    fn parse_paren_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let lp = ctx.eat_kind(TokenKind::LParen)?;
        children.push(Self::token_segment(lp, SegmentType::LParen));
        children.extend(Self::eat_trivia_segments(ctx));

        if let Some(expr) = Self::parse_expression(ctx) {
            children.push(expr);
        }

        children.extend(Self::eat_trivia_segments(ctx));
        if let Some(rp) = ctx.eat_kind(TokenKind::RParen) {
            children.push(Self::token_segment(rp, SegmentType::RParen));
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::ParenExpression,
            children,
        )))
    }

    fn parse_paren_subquery<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let save = ctx.save();
        let mut children = Vec::new();
        let lp = ctx.eat_kind(TokenKind::LParen)?;
        children.push(Self::token_segment(lp, SegmentType::LParen));
        children.extend(Self::eat_trivia_segments(ctx));

        // Check if it's a SELECT or WITH inside
        if !ctx.peek_keyword("SELECT") && !ctx.peek_keyword("WITH") {
            ctx.restore(save);
            return None;
        }

        if let Some(sel) = Self::parse_select_statement(ctx) {
            children.push(sel);
        } else {
            ctx.restore(save);
            return None;
        }

        children.extend(Self::eat_trivia_segments(ctx));
        if let Some(rp) = ctx.eat_kind(TokenKind::RParen) {
            children.push(Self::token_segment(rp, SegmentType::RParen));
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::Subquery,
            children,
        )))
    }

    fn parse_case_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let case_kw = ctx.eat_keyword("CASE")?;
        children.push(Self::token_segment(case_kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // Simple CASE: CASE expr WHEN ...
        // Searched CASE: CASE WHEN ...
        if !ctx.peek_keyword("WHEN") {
            if let Some(expr) = Self::parse_expression(ctx) {
                children.push(expr);
                children.extend(Self::eat_trivia_segments(ctx));
            }
        }

        // WHEN clauses
        while ctx.peek_keyword("WHEN") {
            if let Some(when) = Self::parse_when_clause(ctx) {
                children.push(when);
                children.extend(Self::eat_trivia_segments(ctx));
            }
        }

        // ELSE clause
        if ctx.peek_keyword("ELSE") {
            let mut else_children = Vec::new();
            let kw = ctx.advance().unwrap();
            else_children.push(Self::token_segment(kw, SegmentType::Keyword));
            else_children.extend(Self::eat_trivia_segments(ctx));
            if let Some(expr) = Self::parse_expression(ctx) {
                else_children.push(expr);
            }
            children.push(Segment::Node(NodeSegment::new(
                SegmentType::ElseClause,
                else_children,
            )));
            children.extend(Self::eat_trivia_segments(ctx));
        }

        // END
        if let Some(end_kw) = ctx.eat_keyword("END") {
            children.push(Self::token_segment(end_kw, SegmentType::Keyword));
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::CaseExpression,
            children,
        )))
    }

    fn parse_when_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("WHEN")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        if let Some(cond) = Self::parse_expression(ctx) {
            children.push(cond);
        }
        children.extend(Self::eat_trivia_segments(ctx));

        if let Some(then_kw) = ctx.eat_keyword("THEN") {
            children.push(Self::token_segment(then_kw, SegmentType::Keyword));
        }
        children.extend(Self::eat_trivia_segments(ctx));

        if let Some(result) = Self::parse_expression(ctx) {
            children.push(result);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::WhenClause,
            children,
        )))
    }

    fn parse_exists_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("EXISTS")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        if let Some(subq) = Self::parse_paren_subquery(ctx) {
            children.push(subq);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::ExistsExpression,
            children,
        )))
    }

    fn parse_cast_expression<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let kw = ctx.eat_keyword("CAST")?;
        children.push(Self::token_segment(kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        let lp = ctx.eat_kind(TokenKind::LParen)?;
        children.push(Self::token_segment(lp, SegmentType::LParen));
        children.extend(Self::eat_trivia_segments(ctx));

        if let Some(expr) = Self::parse_expression(ctx) {
            children.push(expr);
        }
        children.extend(Self::eat_trivia_segments(ctx));

        if let Some(as_kw) = ctx.eat_keyword("AS") {
            children.push(Self::token_segment(as_kw, SegmentType::Keyword));
        }
        children.extend(Self::eat_trivia_segments(ctx));

        // Data type
        if let Some(dt) = Self::parse_data_type(ctx) {
            children.push(dt);
        }
        children.extend(Self::eat_trivia_segments(ctx));

        if let Some(rp) = ctx.eat_kind(TokenKind::RParen) {
            children.push(Self::token_segment(rp, SegmentType::RParen));
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::CastExpression,
            children,
        )))
    }

    fn parse_data_type<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();

        // Type name (may be multi-word like "DOUBLE PRECISION", "CHARACTER VARYING")
        let word = ctx.eat_kind(TokenKind::Word)?;
        children.push(Self::token_segment(word, SegmentType::Keyword));

        // Additional type words
        loop {
            let save = ctx.save();
            let trivia = Self::eat_trivia_segments(ctx);
            if let Some(t) = ctx.peek() {
                if t.kind == TokenKind::Word && !Self::is_clause_keyword(&t.text) {
                    children.extend(trivia);
                    let w = ctx.advance().unwrap();
                    children.push(Self::token_segment(w, SegmentType::Keyword));
                    continue;
                }
            }
            ctx.restore(save);
            break;
        }

        // Optional (precision, scale)
        let save = ctx.save();
        let trivia = Self::eat_trivia_segments(ctx);
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            children.extend(trivia);
            if let Some(params) = Self::parse_paren_block(ctx) {
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

    fn parse_identifier<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        match ctx.peek_kind()? {
            TokenKind::Word => {
                let token = ctx.advance().unwrap();
                Some(Self::token_segment(token, SegmentType::Identifier))
            }
            TokenKind::QuotedIdentifier => {
                let token = ctx.advance().unwrap();
                Some(Self::token_segment(token, SegmentType::QuotedIdentifier))
            }
            _ => None,
        }
    }

    /// Parse a possibly qualified name: a, a.b, a.b.c
    fn parse_qualified_name<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let first = Self::parse_identifier(ctx)?;

        let save = ctx.save();
        if ctx.peek_kind() == Some(TokenKind::Dot) {
            let mut children = vec![first];
            while ctx.peek_kind() == Some(TokenKind::Dot) {
                let dot = ctx.advance().unwrap();
                children.push(Self::token_segment(dot, SegmentType::Dot));
                if let Some(part) = Self::parse_identifier(ctx) {
                    children.push(part);
                } else {
                    // Star: schema.table.*
                    if ctx.peek_kind() == Some(TokenKind::Star) {
                        let star = ctx.advance().unwrap();
                        children.push(Self::token_segment(star, SegmentType::Star));
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

    fn parse_name_or_function<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let name = Self::parse_qualified_name(ctx)?;

        // Check for function call: name(...)
        if ctx.peek_kind() == Some(TokenKind::LParen) {
            let mut children = vec![name];
            if let Some(args) = Self::parse_paren_block(ctx) {
                children.push(args);
            }
            let func = Segment::Node(NodeSegment::new(
                SegmentType::FunctionCall,
                children,
            ));

            // Check for OVER clause (window function)
            let save = ctx.save();
            let trivia = Self::eat_trivia_segments(ctx);
            if ctx.peek_keyword("OVER") {
                let mut win_children = vec![func];
                win_children.extend(trivia);
                if let Some(over) = Self::parse_over_clause(ctx) {
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
    fn parse_over_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let over_kw = ctx.eat_keyword("OVER")?;
        children.push(Self::token_segment(over_kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // OVER window_name (named window reference, no parens)
        if ctx.peek_kind() != Some(TokenKind::LParen) {
            if let Some(name) = Self::parse_identifier(ctx) {
                children.push(name);
            }
            return Some(Segment::Node(NodeSegment::new(
                SegmentType::OverClause,
                children,
            )));
        }

        // OVER ( ... )
        let lp = ctx.eat_kind(TokenKind::LParen)?;
        children.push(Self::token_segment(lp, SegmentType::LParen));
        children.extend(Self::eat_trivia_segments(ctx));

        // PARTITION BY ...
        if ctx.peek_keyword("PARTITION") {
            if let Some(pb) = Self::parse_partition_by_clause(ctx) {
                children.push(pb);
                children.extend(Self::eat_trivia_segments(ctx));
            }
        }

        // ORDER BY ...
        if ctx.peek_keywords(&["ORDER", "BY"]) {
            if let Some(ob) = Self::parse_window_order_by(ctx) {
                children.push(ob);
                children.extend(Self::eat_trivia_segments(ctx));
            }
        }

        // Window frame: ROWS / RANGE / GROUPS
        if ctx.peek_keyword("ROWS") || ctx.peek_keyword("RANGE") || ctx.peek_keyword("GROUPS") {
            if let Some(frame) = Self::parse_window_frame_clause(ctx) {
                children.push(frame);
                children.extend(Self::eat_trivia_segments(ctx));
            }
        }

        if let Some(rp) = ctx.eat_kind(TokenKind::RParen) {
            children.push(Self::token_segment(rp, SegmentType::RParen));
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::OverClause,
            children,
        )))
    }

    /// Parse PARTITION BY expr, expr, ...
    fn parse_partition_by_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let part_kw = ctx.eat_keyword("PARTITION")?;
        children.push(Self::token_segment(part_kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        if let Some(by_kw) = ctx.eat_keyword("BY") {
            children.push(Self::token_segment(by_kw, SegmentType::Keyword));
        }
        children.extend(Self::eat_trivia_segments(ctx));

        // Comma-separated expressions
        if let Some(expr) = Self::parse_expression(ctx) {
            children.push(expr);
        }
        loop {
            let save = ctx.save();
            let trivia = Self::eat_trivia_segments(ctx);
            if let Some(comma) = ctx.eat_kind(TokenKind::Comma) {
                children.extend(trivia);
                children.push(Self::token_segment(comma, SegmentType::Comma));
                children.extend(Self::eat_trivia_segments(ctx));
                if let Some(expr) = Self::parse_expression(ctx) {
                    children.push(expr);
                }
            } else {
                ctx.restore(save);
                break;
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::PartitionByClause,
            children,
        )))
    }

    /// Parse ORDER BY inside a window spec (reuses expression parsing).
    fn parse_window_order_by<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        // Delegate to the existing ORDER BY parser
        Self::parse_order_by_clause(ctx)
    }

    /// Parse window frame: ROWS/RANGE/GROUPS frame_spec
    fn parse_window_frame_clause<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();

        // ROWS | RANGE | GROUPS
        let frame_kw = ctx.advance()?;
        children.push(Self::token_segment(frame_kw, SegmentType::Keyword));
        children.extend(Self::eat_trivia_segments(ctx));

        // BETWEEN ... AND ... or single bound
        if ctx.peek_keyword("BETWEEN") {
            let bw_kw = ctx.advance().unwrap();
            children.push(Self::token_segment(bw_kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));

            // start bound
            Self::eat_frame_bound(ctx, &mut children);
            children.extend(Self::eat_trivia_segments(ctx));

            // AND
            if let Some(and_kw) = ctx.eat_keyword("AND") {
                children.push(Self::token_segment(and_kw, SegmentType::Keyword));
                children.extend(Self::eat_trivia_segments(ctx));
            }

            // end bound
            Self::eat_frame_bound(ctx, &mut children);
        } else {
            // Single bound (e.g. ROWS UNBOUNDED PRECEDING)
            Self::eat_frame_bound(ctx, &mut children);
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::WindowFrameClause,
            children,
        )))
    }

    /// Consume a frame bound: UNBOUNDED PRECEDING/FOLLOWING, CURRENT ROW, N PRECEDING/FOLLOWING
    fn eat_frame_bound(ctx: &mut ParseContext, children: &mut Vec<Segment>) {
        // CURRENT ROW
        if ctx.peek_keyword("CURRENT") {
            let kw = ctx.advance().unwrap();
            children.push(Self::token_segment(kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
            if ctx.peek_keyword("ROW") {
                let row_kw = ctx.advance().unwrap();
                children.push(Self::token_segment(row_kw, SegmentType::Keyword));
            }
            return;
        }

        // UNBOUNDED PRECEDING/FOLLOWING
        if ctx.peek_keyword("UNBOUNDED") {
            let kw = ctx.advance().unwrap();
            children.push(Self::token_segment(kw, SegmentType::Keyword));
            children.extend(Self::eat_trivia_segments(ctx));
            if ctx.peek_keyword("PRECEDING") || ctx.peek_keyword("FOLLOWING") {
                let dir = ctx.advance().unwrap();
                children.push(Self::token_segment(dir, SegmentType::Keyword));
            }
            return;
        }

        // N PRECEDING/FOLLOWING
        if ctx.peek_kind() == Some(TokenKind::NumberLiteral) {
            let num = ctx.advance().unwrap();
            children.push(Self::token_segment(num, SegmentType::NumericLiteral));
            children.extend(Self::eat_trivia_segments(ctx));
            if ctx.peek_keyword("PRECEDING") || ctx.peek_keyword("FOLLOWING") {
                let dir = ctx.advance().unwrap();
                children.push(Self::token_segment(dir, SegmentType::Keyword));
            }
        }
    }

    // ── Utility parsing ──────────────────────────────────────────

    /// Parse parenthesized content as a simple block.
    fn parse_paren_block<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        let mut children = Vec::new();
        let lp = ctx.eat_kind(TokenKind::LParen)?;
        children.push(Self::token_segment(lp, SegmentType::LParen));

        let mut depth = 1u32;
        while depth > 0 && !ctx.at_eof() {
            match ctx.peek_kind() {
                Some(TokenKind::LParen) => {
                    depth += 1;
                    let token = ctx.advance().unwrap();
                    children.push(Self::any_token_segment(token));
                }
                Some(TokenKind::RParen) => {
                    depth -= 1;
                    let token = ctx.advance().unwrap();
                    if depth == 0 {
                        children.push(Self::token_segment(token, SegmentType::RParen));
                    } else {
                        children.push(Self::any_token_segment(token));
                    }
                }
                _ => {
                    let token = ctx.advance().unwrap();
                    children.push(Self::any_token_segment(token));
                }
            }
        }

        Some(Segment::Node(NodeSegment::new(
            SegmentType::ParenExpression,
            children,
        )))
    }

    /// Parse parenthesized comma-separated list of identifiers/expressions.
    fn parse_paren_list<'a>(ctx: &mut ParseContext<'a>) -> Option<Segment> {
        Self::parse_paren_block(ctx)
    }

    fn consume_until_end(ctx: &mut ParseContext, children: &mut Vec<Segment>) {
        while !ctx.at_eof() {
            if ctx.peek_kind() == Some(TokenKind::Semicolon) {
                break;
            }
            let token = ctx.advance().unwrap();
            children.push(Self::any_token_segment(token));
        }
    }

    // ── Segment constructors ─────────────────────────────────────

    fn token_segment(token: &Token, segment_type: SegmentType) -> Segment {
        Segment::Token(TokenSegment {
            token: token.clone(),
            segment_type,
        })
    }

    fn any_token_segment(token: &Token) -> Segment {
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
        Self::token_segment(token, st)
    }

    fn unparsable_token(token: &Token) -> Segment {
        Segment::Token(TokenSegment {
            token: token.clone(),
            segment_type: SegmentType::Unparsable,
        })
    }

    fn eat_trivia_segments(ctx: &mut ParseContext) -> Vec<Segment> {
        ctx.eat_trivia()
            .into_iter()
            .map(|t| Self::any_token_segment(t))
            .collect()
    }

    /// Sorted list of keywords that must NOT be consumed as implicit aliases.
    const CLAUSE_KEYWORDS: &[&str] = &[
        "ALTER", "AND", "AS", "BEGIN", "BETWEEN", "BREAK", "CASE", "CATCH",
        "CLOSE", "COMMIT", "CONTINUE", "CREATE", "CROSS", "CURSOR",
        "DEALLOCATE", "DECLARE", "DELETE", "DROP", "ELSE", "END", "EXCEPT",
        "EXEC", "EXECUTE", "EXISTS", "FETCH", "FOR", "FROM", "FULL", "GO",
        "GOTO", "GROUP", "HAVING", "IF", "IN", "INNER", "INSERT",
        "INTERSECT", "INTO", "IS", "JOIN", "LEFT", "LIKE", "LIMIT",
        "MERGE", "NEXT", "NOT", "OFFSET", "ON", "OPEN", "OR", "ORDER",
        "OUTPUT", "OVER", "PARTITION", "PRINT", "RAISERROR", "RETURN",
        "RETURNING", "RIGHT", "ROLLBACK", "SELECT", "SET", "TABLE",
        "THEN", "THROW", "TRUNCATE", "TRY", "UNION", "UPDATE", "USING",
        "VALUES", "WHEN", "WHERE", "WHILE", "WITH",
    ];

    fn is_clause_keyword(word: &str) -> bool {
        let upper = word.to_ascii_uppercase();
        Self::CLAUSE_KEYWORDS.binary_search(&upper.as_str()).is_ok()
    }

    fn is_join_keyword(word: &str) -> bool {
        word.eq_ignore_ascii_case("JOIN")
            || word.eq_ignore_ascii_case("INNER")
            || word.eq_ignore_ascii_case("LEFT")
            || word.eq_ignore_ascii_case("RIGHT")
            || word.eq_ignore_ascii_case("FULL")
            || word.eq_ignore_ascii_case("CROSS")
    }
}
