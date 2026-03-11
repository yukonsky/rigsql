use rigsql_core::{Segment, SegmentType};

/// Check if an AliasExpression's children contain an explicit AS keyword.
pub fn has_as_keyword(children: &[Segment]) -> bool {
    children.iter().any(|child| {
        if let Segment::Token(t) = child {
            t.segment_type == SegmentType::Keyword && t.token.text.eq_ignore_ascii_case("AS")
        } else {
            false
        }
    })
}

/// Return the first non-trivia child segment.
pub fn first_non_trivia(children: &[Segment]) -> Option<&Segment> {
    children.iter().find(|c| !c.segment_type().is_trivia())
}

/// Return the last non-trivia child segment.
pub fn last_non_trivia(children: &[Segment]) -> Option<&Segment> {
    children
        .iter()
        .rev()
        .find(|c| !c.segment_type().is_trivia())
}

/// Keywords that should NOT be treated as alias names.
/// Sorted alphabetically for binary_search.
const NOT_ALIAS_KEYWORDS: &[&str] = &[
    "ALTER",
    "AND",
    "BEGIN",
    "BREAK",
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
    "FETCH",
    "FOR",
    "FROM",
    "FULL",
    "GO",
    "GOTO",
    "GROUP",
    "HAVING",
    "IF",
    "INNER",
    "INSERT",
    "INTERSECT",
    "INTO",
    "JOIN",
    "LEFT",
    "LIMIT",
    "MERGE",
    "NATURAL",
    "NEXT",
    "OFFSET",
    "ON",
    "OPEN",
    "OR",
    "ORDER",
    "OUTPUT",
    "OVER",
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
    "VALUES",
    "WHEN",
    "WHERE",
    "WHILE",
    "WITH",
];

/// Check if the "alias name" in an AliasExpression is actually a misidentified
/// SQL keyword (e.g. OVER in window functions). Returns true if the alias
/// looks like a false positive.
pub fn is_false_alias(children: &[Segment]) -> bool {
    // The alias name is the last non-trivia child
    if let Some(Segment::Token(t)) = last_non_trivia(children) {
        let upper = t.token.text.to_ascii_uppercase();
        return NOT_ALIAS_KEYWORDS.binary_search(&upper.as_str()).is_ok();
    }
    false
}

/// Extract the alias name from an AliasExpression.
/// The alias name is the last Identifier or QuotedIdentifier before any
/// non-trivia, non-keyword segment (scanning from the end).
pub fn extract_alias_name(children: &[Segment]) -> Option<String> {
    for child in children.iter().rev() {
        let st = child.segment_type();
        if st == SegmentType::Identifier || st == SegmentType::QuotedIdentifier {
            if let Segment::Token(t) = child {
                return Some(t.token.text.to_string());
            }
        }
        if st.is_trivia() {
            continue;
        }
        if st != SegmentType::Keyword {
            break;
        }
    }
    None
}

/// Find a keyword by case-insensitive name in children. Returns (index, segment).
pub fn find_keyword_in_children<'a>(
    children: &'a [Segment],
    name: &str,
) -> Option<(usize, &'a Segment)> {
    children.iter().enumerate().find(|(_, c)| {
        if let Segment::Token(t) = c {
            t.segment_type == SegmentType::Keyword && t.token.text.eq_ignore_ascii_case(name)
        } else {
            false
        }
    })
}
