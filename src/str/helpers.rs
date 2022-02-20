use crate::ast::Node;
use crate::str::SqlError;

macro_rules! must {
    ($expr:expr) => {
        $expr
            .as_ref()
            .ok_or_else(|| SqlError::Missing(stringify!($expr).into()))?
    };
}

macro_rules! node {
    ($expr:expr, $ty:path) => {
        match &$expr {
            $ty(elem) => elem,
            unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
        }
    };
}

macro_rules! iter_only {
    ($expr:expr, $ty:path) => {
        $expr.iter().filter_map(|n| match n {
            $ty(elem) => Some(elem),
            _ => None,
        })
    };
}

macro_rules! int_value {
    ($expr:expr) => {
        match &$expr {
            Node::Integer { value } => *value,
            unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
        }
    };
}

macro_rules! string_value {
    ($expr:expr) => {
        match &$expr {
            Node::String { value: Some(value) } => value,
            unexpected => return Err(SqlError::UnexpectedNodeType(unexpected.name())),
        }
    };
}

macro_rules! unsupported {
    ($expr:expr) => {
        return Err(SqlError::Unsupported(format!("{:?}", $expr)))
    };
}

pub(in crate::str) fn join_strings(
    buffer: &mut String,
    nodes: &[Node],
    delim: &str,
) -> core::result::Result<(), SqlError> {
    for (index, node) in nodes.iter().enumerate() {
        if index > 0 {
            buffer.push_str(delim);
        }
        if let Node::String { value: Some(value) } = node {
            buffer.push_str(&quote_identifier(value));
        } else {
            return Err(SqlError::UnexpectedNodeType(node.name()));
        }
    }
    Ok(())
}

pub(in crate::str) fn quote_identifier(ident: &str) -> String {
    if ident.is_empty() {
        return String::new();
    }

    // future: Use the direct PostgreSQL function
    // For now, we partially reproduce it
    // We don't need to quote if the identifier starts with a lowercase letter or underscore
    // and contains only lowercase letters, digits and underscores, AND is not a reserved keyword.
    let chars = ident.chars().collect::<Vec<_>>();
    let safe = chars
        .iter()
        .all(|c| c.is_lowercase() || c.is_digit(10) || *c == '_')
        && !chars[0].is_digit(10);
    if safe && !is_keyword(ident) {
        ident.to_string()
    } else {
        format!("\"{}\"", ident)
    }
}

pub(in crate::str) fn persistence_from_code(code: char) -> Option<&'static str> {
    match code {
        // Regular table
        'p' => None,
        'u' => Some("UNLOGGED"),
        't' => Some("TEMPORARY"),
        // Just ignore rather than error
        _ => None,
    }
}

pub(in crate::str) fn node_vec_to_string_vec(nodes: &[Node]) -> Vec<&String> {
    nodes
        .iter()
        .filter_map(|n| match n {
            Node::String { value: Some(value) } => Some(value),
            _ => None,
        })
        .collect::<Vec<_>>()
}

// Returns true if the operator contains ONLY operator characters
pub(in crate::str) fn is_operator(op: &str) -> bool {
    for char in op.chars() {
        match char {
            '~' | '!' | '@' | '#' | '^' | '&' | '|' | '`' | '?' | '+' | '-' | '*' | '/' | '%'
            | '<' | '>' | '=' => {}
            _ => return false,
        }
    }
    true
}

pub(in crate::str) fn is_keyword(ident: &str) -> bool {
    matches!(
        &ident.to_ascii_lowercase()[..],
        "all"
            | "analyse"
            | "analyze"
            | "and"
            | "any"
            | "array"
            | "as"
            | "asc"
            | "asymmetric"
            | "authorization"
            | "binary"
            | "both"
            | "case"
            | "cast"
            | "check"
            | "collate"
            | "collation"
            | "column"
            | "concurrently"
            | "constraint"
            | "create"
            | "cross"
            | "current_catalog"
            | "current_date"
            | "current_role"
            | "current_schema"
            | "current_time"
            | "current_timestamp"
            | "current_user"
            | "default"
            | "deferrable"
            | "desc"
            | "distinct"
            | "do"
            | "else"
            | "end"
            | "except"
            | "false"
            | "fetch"
            | "for"
            | "foreign"
            | "freeze"
            | "from"
            | "full"
            | "grant"
            | "group"
            | "having"
            | "ilike"
            | "in"
            | "initially"
            | "inner"
            | "intersect"
            | "into"
            | "is"
            | "isnull"
            | "join"
            | "lateral"
            | "leading"
            | "left"
            | "like"
            | "limit"
            | "localtime"
            | "localtimestamp"
            | "natural"
            | "not"
            | "notnull"
            | "null"
            | "offset"
            | "on"
            | "only"
            | "or"
            | "order"
            | "outer"
            | "overlaps"
            | "placing"
            | "primary"
            | "references"
            | "returning"
            | "right"
            | "select"
            | "session_user"
            | "similar"
            | "some"
            | "symmetric"
            | "table"
            | "tablesample"
            | "then"
            | "to"
            | "trailing"
            | "true"
            | "union"
            | "unique"
            | "user"
            | "using"
            | "variadic"
            | "verbose"
            | "when"
            | "where"
            | "window"
            | "with"
    )
}

pub(in crate::str) fn non_reserved_word_or_sconst(
    buffer: &mut String,
    val: &str,
) -> core::result::Result<(), SqlError> {
    if val.is_empty() {
        buffer.push_str("''");
    } else if val.len() >= 64 {
        // NAMEDATALEN constant in pg
        use crate::str::SqlBuilder;
        crate::str::ext::StringLiteral(val).build(buffer)?;
    } else {
        buffer.push_str(&quote_identifier(val));
    }
    Ok(())
}
