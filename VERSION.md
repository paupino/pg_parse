# Version 0.14

**This release upgrades `libpg_query` so contains breaking changes.**

Modified:

* Updated `libpg_query` to [18.0.0](https://github.com/pganalyze/libpg_query/releases/tag/18.0.0). This bumps the embedded
  Postgres version to 18. Several AST changes are introduced:
  * `Node::SinglePartitionSpec` was removed.
  * `Node::ATAlterConstraint`, `Node::ReturningClause`, `Node::ReturningOption`, and `Node::ReturningExpr` were added.
  * `AlterTableType::AT_CheckNotNull` was removed.
  * `ConstrType::CONSTR_ATTR_ENFORCED` and `ConstrType::CONSTR_ATTR_NOT_ENFORCED` were added.
  * `JoinType::JOIN_RIGHT_SEMI` was added.
  * `InsertStmt`, `UpdateStmt`, and `DeleteStmt` now expose `returning_clause: Option<ReturningClause>` instead of a flat
    `returning_list: Option<Vec<Node>>`. The list of returned expressions now lives at `returning_clause.exprs`.
  * `A_Expr` gained `rexpr_list_start` and `rexpr_list_end` fields.
  * `Constraint` gained `is_enforced`, `generated_kind`, `without_overlaps`, `fk_with_period`, and `pk_with_period`;
    the `inhcount` field was removed.
  * `CreateStmt` gained an `nnconstraints` field.
  * `VariableSetStmt` gained `jumble_args` and `location` fields.
  * Fingerprints have changed for some queries to align with Postgres 18.
* `build.rs` now generates the `CompareType` enum from the new `access/cmptype` section.
* The `str` feature now round-trips the new PG18 syntax:
  * `CHECK (...) [NOT] ENFORCED` column constraints.
  * Virtual generated columns (`GENERATED ALWAYS AS (...) VIRTUAL`).
  * `RETURNING WITH (OLD AS ..., NEW AS ...)` on `INSERT`/`UPDATE`/`DELETE`.
  * `ALTER TABLE ... ALTER CONSTRAINT c [NOT] ENFORCED | [NOT] DEFERRABLE [INITIALLY DEFERRED] | [NO] INHERIT`
    via the new `ATAlterConstraint` node.

# Version 0.13

Modified:

* Updated `libpg_query` to [17-6.2.2](https://github.com/pganalyze/libpg_query/tree/17-6.2.2). This bumps the embedded
  Postgres version from 17.4 to 17.7. No AST changes are introduced.
* `build.rs` updated so that cross-compiling to alternative targets works.

# Version 0.12

**This release upgrades `libpg_query` so contains breaking changes.**

Modified:

* Updated `libpg_query` to [17-6.1.0](https://github.com/pganalyze/libpg_query/tree/17-6.1.0).
* Made string generation opt-in instead of enabled by default.
* Updated dependencies.

Other:

* Bumped the project version to `2024` and updated syntax accordingly.

# Version 0.11

**This release upgrades `libpg_query` so contains breaking changes.**

New:

* Introduced a `parse_debug` function to allow for consuming functions to inspect the raw JSON output from
  `libpg_query`.
  This is likely only used internally by library authors, but a useful feature nonetheless.

Modified:

* Updated `libpg_query` to [15-4.2.2](https://github.com/pganalyze/libpg_query/tree/15-4.2.2). This required a lot of
  refactoring to support the modified
  AST being generated.
* String generation is now feature gated under `str`. This feature is not feature complete
  so should be used with caution. Please note, this is currently enabled by default.

Other:

* Bumped the project version to `2021` and updated syntax accordingly.

Please note that some syntax support has been dropped between Postgres version releases. For example,
the `?` placeholder is no longer supported. For a full list, please see the `libpg_query` changelog.

# Version 0.10

Modified:

* Updated `libpg_query` to [13-2.2.0](https://github.com/pganalyze/libpg_query/releases/tag/13-2.2.0).
* Build optimization to prevent rebuilding when no changes [#16](https://github.com/paupino/pg_parse/pull/16).

Thank you [@haileys](https://github.com/haileys) for your contribution!

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

An example of this behaviour is `SELECT DISTINCT a, b FROM c`. The `distinct_clause` generates `[{}]` from
`libpg_query`.
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