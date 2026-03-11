use strum::{Display, EnumString};

use crate::{Span, Token};

/// A node in the Concrete Syntax Tree.
///
/// Leaf nodes wrap individual tokens. Branch nodes group children
/// under a named production (e.g. `SelectStatement`, `WhereClause`).
#[derive(Debug, Clone)]
pub enum Segment {
    Token(TokenSegment),
    Node(NodeSegment),
}

impl Segment {
    pub fn span(&self) -> Span {
        match self {
            Segment::Token(t) => t.token.span,
            Segment::Node(n) => n.span,
        }
    }

    pub fn segment_type(&self) -> SegmentType {
        match self {
            Segment::Token(t) => t.segment_type,
            Segment::Node(n) => n.segment_type,
        }
    }

    /// Recursively collect all leaf tokens in order.
    pub fn tokens(&self) -> Vec<&Token> {
        match self {
            Segment::Token(t) => vec![&t.token],
            Segment::Node(n) => n.children.iter().flat_map(|c| c.tokens()).collect(),
        }
    }

    /// Iterator over direct children (empty for token segments).
    pub fn children(&self) -> &[Segment] {
        match self {
            Segment::Token(_) => &[],
            Segment::Node(n) => &n.children,
        }
    }

    /// Recursively visit all segments depth-first.
    pub fn walk(&self, visitor: &mut dyn FnMut(&Segment)) {
        visitor(self);
        if let Segment::Node(n) = self {
            for child in &n.children {
                child.walk(visitor);
            }
        }
    }

    /// Reconstruct source text from leaf tokens.
    pub fn raw(&self) -> String {
        self.tokens().iter().map(|t| t.text.as_str()).collect()
    }
}

/// A leaf segment wrapping a single token.
#[derive(Debug, Clone)]
pub struct TokenSegment {
    pub token: Token,
    pub segment_type: SegmentType,
}

/// A branch segment grouping children under a named production.
#[derive(Debug, Clone)]
pub struct NodeSegment {
    pub segment_type: SegmentType,
    pub children: Vec<Segment>,
    pub span: Span,
}

impl NodeSegment {
    /// Create a new node from children, computing span automatically.
    pub fn new(segment_type: SegmentType, children: Vec<Segment>) -> Self {
        let span = if children.is_empty() {
            Span::new(0, 0)
        } else {
            let first = children.first().unwrap().span();
            let last = children.last().unwrap().span();
            first.merge(last)
        };
        Self {
            segment_type,
            children,
            span,
        }
    }
}

/// Type tag for CST segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString)]
pub enum SegmentType {
    // Top-level
    File,
    Statement,

    // DML Statements
    SelectStatement,
    InsertStatement,
    UpdateStatement,
    DeleteStatement,

    // DDL Statements
    CreateTableStatement,
    AlterTableStatement,
    DropStatement,

    // PostgreSQL
    TypeCastExpression,
    OnConflictClause,
    ArrayAccessExpression,

    // TSQL
    TableHint,

    // TSQL Statements
    DeclareStatement,
    SetVariableStatement,
    IfStatement,
    BeginEndBlock,
    WhileStatement,
    TryCatchBlock,
    ExecStatement,
    ReturnStatement,
    PrintStatement,
    ThrowStatement,
    RaiserrorStatement,
    GoStatement,

    // Clauses
    SelectClause,
    FromClause,
    WhereClause,
    GroupByClause,
    HavingClause,
    OrderByClause,
    LimitClause,
    OffsetClause,
    JoinClause,
    OnClause,
    UsingClause,
    SetClause,
    ValuesClause,
    ReturningClause,
    WithClause,
    CteDefinition,
    InsertColumnsClause,

    // Expressions
    ColumnRef,
    TableRef,
    FunctionCall,
    FunctionArgs,
    Expression,
    BinaryExpression,
    UnaryExpression,
    ParenExpression,
    CaseExpression,
    WhenClause,
    ElseClause,
    Subquery,
    ExistsExpression,
    InExpression,
    BetweenExpression,
    CastExpression,
    IsNullExpression,
    LikeExpression,

    // Window functions
    WindowExpression,
    OverClause,
    PartitionByClause,
    WindowFrameClause,

    // Alias
    AliasExpression,

    // Column / Table definition
    ColumnDefinition,
    DataType,
    ColumnConstraint,
    TableConstraint,

    // Order
    OrderByExpression,
    SortOrder,

    // Atoms (leaf-level semantic types)
    Keyword,
    Identifier,
    QualifiedIdentifier,
    QuotedIdentifier,
    Literal,
    NumericLiteral,
    StringLiteral,
    BooleanLiteral,
    NullLiteral,
    Operator,
    ComparisonOperator,
    ArithmeticOperator,
    Comma,
    Dot,
    Semicolon,
    Star,
    LParen,
    RParen,

    // Trivia
    Whitespace,
    Newline,
    LineComment,
    BlockComment,

    // Fallback
    Unparsable,
}

impl SegmentType {
    /// Returns true if this is a trivia type (whitespace/comment).
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            SegmentType::Whitespace
                | SegmentType::Newline
                | SegmentType::LineComment
                | SegmentType::BlockComment
        )
    }
}
