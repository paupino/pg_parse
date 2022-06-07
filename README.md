pg_parse &emsp; [![Build Status]][actions] [![Latest Version]][crates.io] [![Docs Badge]][docs]
===========

[Build Status]: https://img.shields.io/endpoint.svg?url=https%3A%2F%2Factions-badge.atrox.dev%2Fpaupino%2Fpg_parse%2Fbadge&label=build&logo=none
[actions]: https://actions-badge.atrox.dev/paupino/pg_parse/goto
[Latest Version]: https://img.shields.io/crates/v/pg_parse.svg
[crates.io]: https://crates.io/crates/pg_parse
[Docs Badge]: https://docs.rs/pg_parse/badge.svg
[docs]: https://docs.rs/pg_parse

PostgreSQL parser for Rust that uses the [actual PostgreSQL server source]((https://github.com/pganalyze/libpg_query)) to parse 
SQL queries and return the internal PostgreSQL parse tree.

## Getting started

Add the following to your `Cargo.toml`

```toml
[dependencies]
pg_parse = "0.9"
```

## Example: Parsing a query

```rust
use pg_parse::ast::Node;

let result = pg_parse::parse("SELECT * FROM contacts");
assert!(result.is_ok());
let result = result.unwrap();
assert!(matches!(*&result[0], Node::SelectStmt(_)));

// We can also convert back to a string
assert_eq!(result[0].to_string(), "SELECT * FROM contacts");
```

## What's the difference between pg_parse and pg_query.rs?

The [`pganalyze`](https://github.com/pganalyze/) organization will maintain the official implementation called [`pg_query.rs`](https://github.com/pganalyze/pg_query.rs). This
closely resembles the name of the C library also published by the team (`libpg_query`). This implementation will use the protobuf 
interface introduced with version 13 of `libpg_query`.

This library similarly consumes `libpg_query` however utilizes the older JSON interface to manage parsing. The intention of this library
is to maintain a dependency "light" implementation with `serde` being the only required runtime dependency. While this was originally called
`pg_query.rs` it makes sense to decouple itself from the official naming convention and go on it's own. Hence `pg_parse`.

So which one should you use? You probably want to use the official `pg_query.rs` library as that will continue to be 
kept closely up to date with `libpg_query` updates. This library will continue to be maintained however may not be as up
to date as the official implementation.

## Credits

A huge thank you to [Lukas Fittl](https://github.com/lfittl) for all of his amazing work creating [libpg_query](https://github.com/pganalyze/libpg_query).
