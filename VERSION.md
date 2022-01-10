# Version 0.6.0

Fixes issue when parsing some statements that would contain trailing null objects in an array. Deserialization of these
is now performed correctly. Note that this may cause some differing behavior from other `libpg_query` implementations
whereby a "null object" is intended to indicate the end of an array.

An example of this behaviour is `SELECT DISTINCT a, b FROM c`. The `distinct_clause` generates `[{}]` from `libpg_query`.
`pg_query.rs` now parses this as `vec![]`.

# Version 0.5.0

* Enums can now be compared directly.
* `Null` is generated with empty parameters to support JSON mapping.

# Version 0.4.0

Updates `libpg_query` dependency to [`13-2.1.0`](https://github.com/pganalyze/libpg_query/tree/13-2.1.0).

# Version 0.3.0

* Fixes `Value` parsing in some situations such as for `typemod`

# Version 0.2.0

* Adds in the `List` node type by generating `nodes/pg_list` in `structdef`.
* Implement `std::error::Error` for `Error` type