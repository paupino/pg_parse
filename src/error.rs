use std::fmt::{Display, Formatter};

/// Error structure representing the basic error scenarios for `pg_parse`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Error {
    ParseError(String),
    InvalidAst(String),
    InvalidJson(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ParseError(value) => write!(f, "Parse Error: {}", value),
            Error::InvalidAst(value) => write!(f, "Invalid AST: {}", value),
            Error::InvalidJson(value) => write!(f, "Invalid JSON: {}", value),
        }
    }
}

impl std::error::Error for Error {}

/// Convenient Result alias for returning `pg_parse::Error`.
pub type Result<T> = core::result::Result<T, Error>;
