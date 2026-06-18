use std::fmt::{Display, Formatter};

/// Error structure representing the basic error scenarios for `pg_parse`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Error {
    /// A syntax/parse error reported by the underlying PostgreSQL parser.
    ParseError {
        /// The human readable error message (e.g. `syntax error at or near "RANDOM"`).
        message: String,
        /// The location within the input query at which the error occurred, as reported by
        /// libpg_query's `cursorpos` field.
        ///
        /// # Semantics
        ///
        /// `cursorpos` is a **1-based**, **character (Unicode code point)** offset into the
        /// query string — *not* a byte offset. It originates from PostgreSQL's internal error
        /// cursor, which counts characters from 1. For pure-ASCII input characters and bytes
        /// coincide, but for input containing multi-byte UTF-8 characters they do not, so do
        /// **not** treat `cursorpos` as a byte index.
        ///
        /// For a query such as `CREATE TABL public.foo (id int)`, `cursorpos` points at the
        /// first character of the offending token (`TABL`), i.e. `cursorpos == 8` (1-based).
        ///
        /// To convert it into a 0-based **character** index, subtract 1:
        ///
        /// ```text
        /// let char_index = (cursorpos - 1) as usize;
        /// ```
        ///
        /// To map it to a 0-based **byte** offset suitable for slicing the original `&str`
        /// (e.g. to render a caret/source-span diagnostic), walk the character boundaries
        /// rather than subtracting:
        ///
        /// ```text
        /// let char_index = (cursorpos - 1) as usize;
        /// let byte_offset = input
        ///     .char_indices()
        ///     .nth(char_index)
        ///     .map(|(byte, _)| byte)
        ///     .unwrap_or(input.len()); // char_index == char count => end of input
        /// ```
        ///
        /// ## End-of-input errors
        ///
        /// When the parser hits the end of the input unexpectedly (e.g. `SELECT * FROM` or
        /// `SELECT * FROM foo WHERE`, which produce "syntax error at end of input"), `cursorpos`
        /// points **one past the last character**, i.e. it equals the input's character length
        /// plus 1. It is *not* `0` in this case. Using the `char_indices()` mapping above, the
        /// resulting `byte_offset` lands exactly at `input.len()` (the end of the string).
        ///
        /// ## The value `0`
        ///
        /// A `cursorpos` of `0` means libpg_query reported no position for the error. The parse
        /// entry points exercised by this crate produce positioned syntax errors, so `0` is not
        /// expected in practice, but callers rendering carets should still treat any value `<= 0`
        /// (or one exceeding the input length) as "no specific location" rather than indexing
        /// with it.
        cursorpos: i32,
    },
    InvalidAst(String),
    InvalidAstWithDebug(String, String),
    InvalidJson(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            // Note: the position (`cursorpos`) is intentionally omitted so that the textual
            // output remains identical to previous versions for downstream consumers that match
            // on the `Display` representation.
            Error::ParseError { message, .. } => write!(f, "Parse Error: {}", message),
            Error::InvalidAst(value) => write!(f, "Invalid AST: {}", value),
            Error::InvalidAstWithDebug(value, debug) => {
                write!(f, "Invalid AST: {}. Debug: {}", value, debug)
            }
            Error::InvalidJson(value) => write!(f, "Invalid JSON: {}", value),
        }
    }
}

impl std::error::Error for Error {}

/// Convenient Result alias for returning `pg_parse::Error`.
pub type Result<T> = core::result::Result<T, Error>;
