# Version 0.9.1

Modified:
* Updated `regex` library to remove potential security vulnerability.

# Version 0.9.0

Modified:
* Updated to latest `libpg_query` version which fixes some memory leaks. 
* Removed `clippy` build dependency which was subject to a potential security vulnerability.

# Version 0.8.0

New:
* `to_string` functionality for AST allowing you to turn the parsed tree back into SQL.

# Version 0.7.0

Renamed project from `pg_query.rs` to `pg_parse`. Going forward the `pganalyze` team will maintain the official fork
leveraging protobuf whereas this library will continue to use the JSON subsystem.

* Remove `Expr` from generated output since it is a generic superclass.

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