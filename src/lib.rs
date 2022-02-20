//! pg_parse
//! ============
//!
//! PostgreSQL parser that uses the [actual PostgreSQL server source]((https://github.com/pganalyze/libpg_query)) to parse
//! SQL queries and return the internal PostgreSQL parse tree.
//!
//! Warning! This library is in early stages of development so any APIs exposed are subject to change.
//!
//! ## Getting started
//!
//! Add the following to your `Cargo.toml`
//!
//! ```toml
//! [dependencies]
//! pg_parse = "0.8"
//! ```
//!
//! # Example: Parsing a query
//!
//! ```rust
//! use pg_parse::ast::Node;
//!
//! let result = pg_parse::parse("SELECT * FROM contacts");
//! assert!(result.is_ok());
//! let result = result.unwrap();
//! assert!(matches!(*&result[0], Node::SelectStmt(_)));
//!
//! // We can also convert back to a string
//! assert_eq!(result[0].to_string(), "SELECT * FROM contacts");
//! ```
//!

/// Generated structures representing the PostgreSQL AST.
pub mod ast;
mod bindings;
mod error;
mod query;
mod serde;
mod str;

pub use error::*;
pub use query::*;
