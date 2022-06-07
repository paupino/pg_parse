use pg_parse::ast::{ConstrType, InsertStmt, List, Node, ParamRef, SelectStmt, Value};

#[test]
fn it_can_generate_a_create_index_ast() {
    let result =
        pg_parse::parse("CREATE INDEX ix_test ON contacts.person (id, ssn) WHERE ssn IS NOT NULL;");
    assert!(result.is_ok());
    let result = result.unwrap();
    let el: &Node = &result[0];
    match *el {
        Node::IndexStmt(ref stmt) => {
            assert_eq!(stmt.idxname, Some("ix_test".to_string()), "idxname");
            let relation = stmt.relation.as_ref().expect("relation exists");
            assert_eq!(
                relation.schemaname,
                Some("contacts".to_string()),
                "schemaname"
            );
            assert_eq!(relation.relname, Some("person".to_string()), "relname");
            let params = stmt.index_params.as_ref().expect("index params");
            assert_eq!(2, params.len(), "Params length");
        }
        _ => panic!("Unexpected type"),
    }
}

#[test]
fn it_can_generate_a_create_table_ast() {
    let result =
        pg_parse::parse("CREATE TABLE contacts.person(id serial primary key, name text not null, balance numeric(5, 12));");
    assert!(result.is_ok());
    let result = result.unwrap();
    let el: &Node = &result[0];
    match *el {
        Node::CreateStmt(ref stmt) => {
            let relation = stmt.relation.as_ref().expect("relation exists");
            assert_eq!(
                relation.schemaname,
                Some("contacts".to_string()),
                "schemaname"
            );
            assert_eq!(relation.relname, Some("person".to_string()), "relname");
            let columns = stmt.table_elts.as_ref().expect("columns");
            assert_eq!(3, columns.len(), "Columns length");
            let balance = &columns[2];
            let column = match balance {
                Node::ColumnDef(def) => def,
                _ => panic!("Unexpected column type"),
            };
            assert_eq!(column.colname, Some("balance".into()));
            let ty = match &column.type_name {
                Some(t) => t,
                None => panic!("Missing type for column balance"),
            };

            // Check the name of the type, and the modifiers
            let names = match &ty.names {
                Some(n) => n,
                None => panic!("No type names found"),
            };
            assert_eq!(names.len(), 2);
            match &names[0] {
                Node::String { value } => assert_eq!(value, &Some("pg_catalog".into())),
                unexpected => panic!("Unexpected type for name[0] {:?}", unexpected),
            }
            match &names[1] {
                Node::String { value } => assert_eq!(value, &Some("numeric".into())),
                unexpected => panic!("Unexpected type for name[1] {:?}", unexpected),
            }

            // Do the mods
            let mods = match &ty.typmods {
                Some(m) => m,
                None => panic!("No type mods found"),
            };
            assert_eq!(mods.len(), 2);
            match &mods[0] {
                Node::A_Const(pg_parse::ast::A_Const { val, .. }) => {
                    let constant = match **val {
                        pg_parse::ast::Value(Node::Integer { value }) => value,
                        _ => panic!("Expected value"),
                    };
                    assert_eq!(constant, 5);
                }
                unexpected => panic!("Unexpected type for mods[0] {:?}", unexpected),
            }
            match &mods[1] {
                Node::A_Const(pg_parse::ast::A_Const { val, .. }) => {
                    let constant = match **val {
                        pg_parse::ast::Value(Node::Integer { value }) => value,
                        _ => panic!("Expected value"),
                    };
                    assert_eq!(constant, 12);
                }
                unexpected => panic!("Unexpected type for mods[0] {:?}", unexpected),
            }
        }
        _ => panic!("Unexpected type"),
    }
}

#[test]
fn it_will_error_on_invalid_input() {
    let result = pg_parse::parse("CREATE RANDOM ix_test ON contacts.person;");
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap(),
        pg_parse::Error::ParseError("syntax error at or near \"RANDOM\"".into())
    );
}

#[test]
fn it_can_parse_lists_of_values() {
    let result = pg_parse::parse("INSERT INTO contacts.person(name, ssn) VALUES ($1, $2)");
    assert!(result.is_ok());
    let result = result.unwrap();
    let el: &Node = &result[0];

    match el {
        Node::InsertStmt(InsertStmt {
            select_stmt: Some(select_stmt),
            ..
        }) => match select_stmt.as_ref() {
            Node::SelectStmt(SelectStmt {
                values_lists: Some(values_lists),
                ..
            }) => {
                let values = &values_lists[0];

                match values {
                    Node::List(List { items }) => {
                        assert_eq!(2, items.len(), "Items length");

                        for (index, item) in items.iter().enumerate() {
                            match item {
                                Node::ParamRef(ParamRef { number, .. }) => {
                                    // postgres params indices start at 1
                                    let expected = index + 1;

                                    assert_eq!(expected, *number as usize, "Param number");
                                }
                                node => panic!("Unexpected type {:#?}", &node),
                            }
                        }
                    }
                    node => panic!("Unexpected type {:#?}", &node),
                }
            }
            node => panic!("Unexpected type {:#?}", &node),
        },
        node => panic!("Unexpected type {:#?}", &node),
    }
}

#[test]
fn it_can_parse_a_table_of_defaults() {
    let result = pg_parse::parse(
        "CREATE TABLE default_values
(
    id       serial        NOT NULL PRIMARY KEY,
    ival     int           NOT NULL DEFAULT(1),
    bval     boolean       NOT NULL DEFAULT(TRUE),
    sval     text          NOT NULL DEFAULT('hello'),
    mval     numeric(10,2) NOT NULL DEFAULT(5.12),
    nval     int           NULL DEFAULT(NULL)
);",
    );
    assert!(result.is_ok());
    let result = result.unwrap();
    let el: &Node = &result[0];
    match *el {
        Node::CreateStmt(ref stmt) => {
            let relation = stmt.relation.as_ref().expect("relation exists");
            assert_eq!(relation.schemaname, None, "schemaname");
            assert_eq!(
                relation.relname,
                Some("default_values".to_string()),
                "relname"
            );
            let columns = stmt.table_elts.as_ref().expect("columns");
            assert_eq!(6, columns.len(), "Columns length");
            let nval = &columns[5];
            let column = match nval {
                Node::ColumnDef(def) => def,
                _ => panic!("Unexpected column type"),
            };
            assert_eq!(column.colname, Some("nval".into()));
            assert!(column.constraints.is_some());
            let constraints = column.constraints.as_ref().unwrap();
            assert_eq!(2, constraints.len(), "constraint #");
            let c1 = match &constraints[0] {
                Node::Constraint(c) => c,
                _ => panic!("Unexpected constraint type"),
            };
            let c2 = match &constraints[1] {
                Node::Constraint(c) => c,
                _ => panic!("Unexpected constraint type"),
            };
            assert_eq!(*c1.contype, ConstrType::CONSTR_NULL);
            assert_eq!(*c2.contype, ConstrType::CONSTR_DEFAULT);
            assert!(c2.raw_expr.is_some());
            let raw_expr = c2.raw_expr.as_ref().unwrap();
            let a_const = match **raw_expr {
                Node::A_Const(ref a) => a,
                _ => panic!("Expected constant"),
            };
            assert!(
                matches!(*a_const.val, Value(Node::Null {})),
                "{:?}",
                a_const
            )
        }
        _ => panic!("Unexpected type"),
    }
}

#[test]
fn it_can_parse_tests() {
    // This is a set of tests inspired by libpg_query that test various situations. The scenario that
    // inspired this was actually SELECT DISTINCT, since it libpg_query it'll return [{}] which doesn't
    // have enough information to be parsed by pg_parse. We no ignore empty array components like this.
    const TESTS: [(&str, &str); 26] = [
        ("SELECT 1",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(A_Const(A_Const { val: Value(Integer { value: 1 }), location: 7 })), location: 7 })]), from_clause: None, where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("SELECT 1; SELECT 2",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(A_Const(A_Const { val: Value(Integer { value: 1 }), location: 7 })), location: 7 })]), from_clause: None, where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None }), SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(A_Const(A_Const { val: Value(Integer { value: 2 }), location: 17 })), location: 17 })]), from_clause: None, where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("select sum(unique1) FILTER (WHERE unique1 IN (SELECT unique1 FROM onek where unique1 < 100)) FROM tenk1",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(FuncCall(FuncCall { funcname: Some([String { value: Some(\"sum\") }]), args: Some([ColumnRef(ColumnRef { fields: Some([String { value: Some(\"unique1\") }]), location: 11 })]), agg_order: None, agg_filter: Some(SubLink(SubLink { sub_link_type: ANY_SUBLINK, sub_link_id: 0, testexpr: Some(ColumnRef(ColumnRef { fields: Some([String { value: Some(\"unique1\") }]), location: 34 })), oper_name: None, subselect: Some(SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(ColumnRef(ColumnRef { fields: Some([String { value: Some(\"unique1\") }]), location: 53 })), location: 53 })]), from_clause: Some([RangeVar(RangeVar { catalogname: None, schemaname: None, relname: Some(\"onek\"), inh: true, relpersistence: 'p', alias: None, location: 66 })]), where_clause: Some(A_Expr(A_Expr { kind: AEXPR_OP, name: Some([String { value: Some(\"<\") }]), lexpr: Some(ColumnRef(ColumnRef { fields: Some([String { value: Some(\"unique1\") }]), location: 77 })), rexpr: Some(A_Const(A_Const { val: Value(Integer { value: 100 }), location: 87 })), location: 85 })), group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })), location: 42 })), agg_within_group: false, agg_star: false, agg_distinct: false, func_variadic: false, over: None, location: 7 })), location: 7 })]), from_clause: Some([RangeVar(RangeVar { catalogname: None, schemaname: None, relname: Some(\"tenk1\"), inh: true, relpersistence: 'p', alias: None, location: 98 })]), where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("select sum(unique1) FILTER (WHERE unique1 = ANY (SELECT unique1 FROM onek where unique1 < 100)) FROM tenk1",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(FuncCall(FuncCall { funcname: Some([String { value: Some(\"sum\") }]), args: Some([ColumnRef(ColumnRef { fields: Some([String { value: Some(\"unique1\") }]), location: 11 })]), agg_order: None, agg_filter: Some(SubLink(SubLink { sub_link_type: ANY_SUBLINK, sub_link_id: 0, testexpr: Some(ColumnRef(ColumnRef { fields: Some([String { value: Some(\"unique1\") }]), location: 34 })), oper_name: Some([String { value: Some(\"=\") }]), subselect: Some(SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(ColumnRef(ColumnRef { fields: Some([String { value: Some(\"unique1\") }]), location: 56 })), location: 56 })]), from_clause: Some([RangeVar(RangeVar { catalogname: None, schemaname: None, relname: Some(\"onek\"), inh: true, relpersistence: 'p', alias: None, location: 69 })]), where_clause: Some(A_Expr(A_Expr { kind: AEXPR_OP, name: Some([String { value: Some(\"<\") }]), lexpr: Some(ColumnRef(ColumnRef { fields: Some([String { value: Some(\"unique1\") }]), location: 80 })), rexpr: Some(A_Const(A_Const { val: Value(Integer { value: 100 }), location: 90 })), location: 88 })), group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })), location: 42 })), agg_within_group: false, agg_star: false, agg_distinct: false, func_variadic: false, over: None, location: 7 })), location: 7 })]), from_clause: Some([RangeVar(RangeVar { catalogname: None, schemaname: None, relname: Some(\"tenk1\"), inh: true, relpersistence: 'p', alias: None, location: 101 })]), where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("CREATE FOREIGN TABLE films (code char(5) NOT NULL, title varchar(40) NOT NULL, did integer NOT NULL, date_prod date, kind varchar(10), len interval hour to minute) SERVER film_server;",
        "[CreateForeignTableStmt(CreateForeignTableStmt { base: CreateStmt { relation: Some(RangeVar { catalogname: None, schemaname: None, relname: Some(\"films\"), inh: true, relpersistence: 'p', alias: None, location: 21 }), table_elts: Some([ColumnDef(ColumnDef { colname: Some(\"code\"), type_name: Some(TypeName { names: Some([String { value: Some(\"pg_catalog\") }, String { value: Some(\"bpchar\") }]), type_oid: 0, setof: false, pct_type: false, typmods: Some([A_Const(A_Const { val: Value(Integer { value: 5 }), location: 38 })]), typemod: -1, array_bounds: None, location: 33 }), inhcount: 0, is_local: true, is_not_null: false, is_from_type: false, storage: '\\0', raw_default: None, cooked_default: None, identity: '\\0', identity_sequence: None, generated: '\\0', coll_clause: None, coll_oid: 0, constraints: Some([Constraint(Constraint { contype: CONSTR_NOTNULL, conname: None, deferrable: false, initdeferred: false, location: 41, is_no_inherit: false, raw_expr: None, cooked_expr: None, generated_when: '\\0', keys: None, including: None, exclusions: None, options: None, indexname: None, indexspace: None, reset_default_tblspc: false, access_method: None, where_clause: None, pktable: None, fk_attrs: None, pk_attrs: None, fk_matchtype: '\\0', fk_upd_action: '\\0', fk_del_action: '\\0', old_conpfeqop: None, old_pktable_oid: 0, skip_validation: false, initially_valid: false })]), fdwoptions: None, location: 28 }), ColumnDef(ColumnDef { colname: Some(\"title\"), type_name: Some(TypeName { names: Some([String { value: Some(\"pg_catalog\") }, String { value: Some(\"varchar\") }]), type_oid: 0, setof: false, pct_type: false, typmods: Some([A_Const(A_Const { val: Value(Integer { value: 40 }), location: 65 })]), typemod: -1, array_bounds: None, location: 57 }), inhcount: 0, is_local: true, is_not_null: false, is_from_type: false, storage: '\\0', raw_default: None, cooked_default: None, identity: '\\0', identity_sequence: None, generated: '\\0', coll_clause: None, coll_oid: 0, constraints: Some([Constraint(Constraint { contype: CONSTR_NOTNULL, conname: None, deferrable: false, initdeferred: false, location: 69, is_no_inherit: false, raw_expr: None, cooked_expr: None, generated_when: '\\0', keys: None, including: None, exclusions: None, options: None, indexname: None, indexspace: None, reset_default_tblspc: false, access_method: None, where_clause: None, pktable: None, fk_attrs: None, pk_attrs: None, fk_matchtype: '\\0', fk_upd_action: '\\0', fk_del_action: '\\0', old_conpfeqop: None, old_pktable_oid: 0, skip_validation: false, initially_valid: false })]), fdwoptions: None, location: 51 }), ColumnDef(ColumnDef { colname: Some(\"did\"), type_name: Some(TypeName { names: Some([String { value: Some(\"pg_catalog\") }, String { value: Some(\"int4\") }]), type_oid: 0, setof: false, pct_type: false, typmods: None, typemod: -1, array_bounds: None, location: 83 }), inhcount: 0, is_local: true, is_not_null: false, is_from_type: false, storage: '\\0', raw_default: None, cooked_default: None, identity: '\\0', identity_sequence: None, generated: '\\0', coll_clause: None, coll_oid: 0, constraints: Some([Constraint(Constraint { contype: CONSTR_NOTNULL, conname: None, deferrable: false, initdeferred: false, location: 91, is_no_inherit: false, raw_expr: None, cooked_expr: None, generated_when: '\\0', keys: None, including: None, exclusions: None, options: None, indexname: None, indexspace: None, reset_default_tblspc: false, access_method: None, where_clause: None, pktable: None, fk_attrs: None, pk_attrs: None, fk_matchtype: '\\0', fk_upd_action: '\\0', fk_del_action: '\\0', old_conpfeqop: None, old_pktable_oid: 0, skip_validation: false, initially_valid: false })]), fdwoptions: None, location: 79 }), ColumnDef(ColumnDef { colname: Some(\"date_prod\"), type_name: Some(TypeName { names: Some([String { value: Some(\"date\") }]), type_oid: 0, setof: false, pct_type: false, typmods: None, typemod: -1, array_bounds: None, location: 111 }), inhcount: 0, is_local: true, is_not_null: false, is_from_type: false, storage: '\\0', raw_default: None, cooked_default: None, identity: '\\0', identity_sequence: None, generated: '\\0', coll_clause: None, coll_oid: 0, constraints: None, fdwoptions: None, location: 101 }), ColumnDef(ColumnDef { colname: Some(\"kind\"), type_name: Some(TypeName { names: Some([String { value: Some(\"pg_catalog\") }, String { value: Some(\"varchar\") }]), type_oid: 0, setof: false, pct_type: false, typmods: Some([A_Const(A_Const { val: Value(Integer { value: 10 }), location: 130 })]), typemod: -1, array_bounds: None, location: 122 }), inhcount: 0, is_local: true, is_not_null: false, is_from_type: false, storage: '\\0', raw_default: None, cooked_default: None, identity: '\\0', identity_sequence: None, generated: '\\0', coll_clause: None, coll_oid: 0, constraints: None, fdwoptions: None, location: 117 }), ColumnDef(ColumnDef { colname: Some(\"len\"), type_name: Some(TypeName { names: Some([String { value: Some(\"pg_catalog\") }, String { value: Some(\"interval\") }]), type_oid: 0, setof: false, pct_type: false, typmods: Some([A_Const(A_Const { val: Value(Integer { value: 3072 }), location: 148 })]), typemod: -1, array_bounds: None, location: 139 }), inhcount: 0, is_local: true, is_not_null: false, is_from_type: false, storage: '\\0', raw_default: None, cooked_default: None, identity: '\\0', identity_sequence: None, generated: '\\0', coll_clause: None, coll_oid: 0, constraints: None, fdwoptions: None, location: 135 })]), inh_relations: None, partbound: None, partspec: None, of_typename: None, constraints: None, options: None, oncommit: ONCOMMIT_NOOP, tablespacename: None, access_method: None, if_not_exists: false }, servername: Some(\"film_server\"), options: None })]"),
        ("CREATE FOREIGN TABLE ft1 () SERVER no_server",
        "[CreateForeignTableStmt(CreateForeignTableStmt { base: CreateStmt { relation: Some(RangeVar { catalogname: None, schemaname: None, relname: Some(\"ft1\"), inh: true, relpersistence: 'p', alias: None, location: 21 }), table_elts: None, inh_relations: None, partbound: None, partspec: None, of_typename: None, constraints: None, options: None, oncommit: ONCOMMIT_NOOP, tablespacename: None, access_method: None, if_not_exists: false }, servername: Some(\"no_server\"), options: None })]"),
        // ("SELECT parse_ident(E'\"c\".X XXXX\002XXXXXX')", ""),
        ("ALTER ROLE postgres LOGIN SUPERUSER PASSWORD 'xyz'",
        "[AlterRoleStmt(AlterRoleStmt { role: Some(RoleSpec { roletype: ROLESPEC_CSTRING, rolename: Some(\"postgres\"), location: 11 }), options: Some([DefElem(DefElem { defnamespace: None, defname: Some(\"canlogin\"), arg: Some(Integer { value: 1 }), defaction: DEFELEM_UNSPEC, location: 20 }), DefElem(DefElem { defnamespace: None, defname: Some(\"superuser\"), arg: Some(Integer { value: 1 }), defaction: DEFELEM_UNSPEC, location: 26 }), DefElem(DefElem { defnamespace: None, defname: Some(\"password\"), arg: Some(String { value: Some(\"xyz\") }), defaction: DEFELEM_UNSPEC, location: 36 })]), action: 1 })]"),
        ("SELECT extract($1 FROM $2)",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(FuncCall(FuncCall { funcname: Some([String { value: Some(\"pg_catalog\") }, String { value: Some(\"date_part\") }]), args: Some([ParamRef(ParamRef { number: 1, location: 15 }), ParamRef(ParamRef { number: 2, location: 23 })]), agg_order: None, agg_filter: None, agg_within_group: false, agg_star: false, agg_distinct: false, func_variadic: false, over: None, location: 7 })), location: 7 })]), from_clause: None, where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("WITH w AS NOT MATERIALIZED (SELECT * FROM big_table) SELECT * FROM w LIMIT 1",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(ColumnRef(ColumnRef { fields: Some([A_Star(A_Star)]), location: 60 })), location: 60 })]), from_clause: Some([RangeVar(RangeVar { catalogname: None, schemaname: None, relname: Some(\"w\"), inh: true, relpersistence: 'p', alias: None, location: 67 })]), where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: Some(A_Const(A_Const { val: Value(Integer { value: 1 }), location: 75 })), limit_option: LIMIT_OPTION_COUNT, locking_clause: None, with_clause: Some(WithClause { ctes: Some([CommonTableExpr(CommonTableExpr { ctename: Some(\"w\"), aliascolnames: None, ctematerialized: CTEMaterializeNever, ctequery: Some(SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(ColumnRef(ColumnRef { fields: Some([A_Star(A_Star)]), location: 35 })), location: 35 })]), from_clause: Some([RangeVar(RangeVar { catalogname: None, schemaname: None, relname: Some(\"big_table\"), inh: true, relpersistence: 'p', alias: None, location: 42 })]), where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })), location: 5, cterecursive: false, cterefcount: 0, ctecolnames: None, ctecoltypes: None, ctecoltypmods: None, ctecolcollations: None })]), recursive: false, location: 0 }), op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("CREATE USER test PASSWORD $1",
        "[CreateRoleStmt(CreateRoleStmt { stmt_type: ROLESTMT_USER, role: Some(\"test\"), options: Some([DefElem(DefElem { defnamespace: None, defname: Some(\"password\"), arg: Some(ParamRef(ParamRef { number: 1, location: 26 })), defaction: DEFELEM_UNSPEC, location: 17 })]) })]"),
        ("ALTER USER test ENCRYPTED PASSWORD $2",
        "[AlterRoleStmt(AlterRoleStmt { role: Some(RoleSpec { roletype: ROLESPEC_CSTRING, rolename: Some(\"test\"), location: 11 }), options: Some([DefElem(DefElem { defnamespace: None, defname: Some(\"password\"), arg: Some(ParamRef(ParamRef { number: 2, location: 35 })), defaction: DEFELEM_UNSPEC, location: 16 })]), action: 1 })]"),
        ("SET SCHEMA $3",
        "[VariableSetStmt(VariableSetStmt { kind: VAR_SET_VALUE, name: Some(\"search_path\"), args: Some([ParamRef(ParamRef { number: 3, location: 11 })]), is_local: false })]"),
        ("SET ROLE $4",
        "[VariableSetStmt(VariableSetStmt { kind: VAR_SET_VALUE, name: Some(\"role\"), args: Some([ParamRef(ParamRef { number: 4, location: 9 })]), is_local: false })]"),
        ("SET SESSION AUTHORIZATION $5",
        "[VariableSetStmt(VariableSetStmt { kind: VAR_SET_VALUE, name: Some(\"session_authorization\"), args: Some([ParamRef(ParamRef { number: 5, location: 26 })]), is_local: false })]"),
        ("SELECT EXTRACT($1 FROM TIMESTAMP $2)",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(FuncCall(FuncCall { funcname: Some([String { value: Some(\"pg_catalog\") }, String { value: Some(\"date_part\") }]), args: Some([ParamRef(ParamRef { number: 1, location: 15 }), TypeCast(TypeCast { arg: Some(ParamRef(ParamRef { number: 2, location: 33 })), type_name: Some(TypeName { names: Some([String { value: Some(\"pg_catalog\") }, String { value: Some(\"timestamp\") }]), type_oid: 0, setof: false, pct_type: false, typmods: None, typemod: -1, array_bounds: None, location: 23 }), location: -1 })]), agg_order: None, agg_filter: None, agg_within_group: false, agg_star: false, agg_distinct: false, func_variadic: false, over: None, location: 7 })), location: 7 })]), from_clause: None, where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("SELECT DATE $1",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(TypeCast(TypeCast { arg: Some(ParamRef(ParamRef { number: 1, location: 12 })), type_name: Some(TypeName { names: Some([String { value: Some(\"date\") }]), type_oid: 0, setof: false, pct_type: false, typmods: None, typemod: -1, array_bounds: None, location: 7 }), location: -1 })), location: 7 })]), from_clause: None, where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("SELECT INTERVAL $1",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(TypeCast(TypeCast { arg: Some(ParamRef(ParamRef { number: 1, location: 16 })), type_name: Some(TypeName { names: Some([String { value: Some(\"pg_catalog\") }, String { value: Some(\"interval\") }]), type_oid: 0, setof: false, pct_type: false, typmods: None, typemod: -1, array_bounds: None, location: 7 }), location: -1 })), location: 7 })]), from_clause: None, where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("SELECT INTERVAL $1 YEAR",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(TypeCast(TypeCast { arg: Some(ParamRef(ParamRef { number: 1, location: 16 })), type_name: Some(TypeName { names: Some([String { value: Some(\"pg_catalog\") }, String { value: Some(\"interval\") }]), type_oid: 0, setof: false, pct_type: false, typmods: Some([A_Const(A_Const { val: Value(Integer { value: 4 }), location: 19 })]), typemod: -1, array_bounds: None, location: 7 }), location: -1 })), location: 7 })]), from_clause: None, where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("SELECT INTERVAL (6) $1",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(TypeCast(TypeCast { arg: Some(ParamRef(ParamRef { number: 1, location: 20 })), type_name: Some(TypeName { names: Some([String { value: Some(\"pg_catalog\") }, String { value: Some(\"interval\") }]), type_oid: 0, setof: false, pct_type: false, typmods: Some([A_Const(A_Const { val: Value(Integer { value: 32767 }), location: -1 }), A_Const(A_Const { val: Value(Integer { value: 6 }), location: 17 })]), typemod: -1, array_bounds: None, location: 7 }), location: -1 })), location: 7 })]), from_clause: None, where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("SET search_path = $1",
        "[VariableSetStmt(VariableSetStmt { kind: VAR_SET_VALUE, name: Some(\"search_path\"), args: Some([ParamRef(ParamRef { number: 1, location: 18 })]), is_local: false })]"),
        ("ALTER ROLE postgres LOGIN SUPERUSER PASSWORD ?",
        "[AlterRoleStmt(AlterRoleStmt { role: Some(RoleSpec { roletype: ROLESPEC_CSTRING, rolename: Some(\"postgres\"), location: 11 }), options: Some([DefElem(DefElem { defnamespace: None, defname: Some(\"canlogin\"), arg: Some(Integer { value: 1 }), defaction: DEFELEM_UNSPEC, location: 20 }), DefElem(DefElem { defnamespace: None, defname: Some(\"superuser\"), arg: Some(Integer { value: 1 }), defaction: DEFELEM_UNSPEC, location: 26 }), DefElem(DefElem { defnamespace: None, defname: Some(\"password\"), arg: Some(ParamRef(ParamRef { number: 0, location: 45 })), defaction: DEFELEM_UNSPEC, location: 36 })]), action: 1 })]"),
        ("WITH a AS (SELECT * FROM x WHERE x.y = ? AND x.z = 1) SELECT * FROM a WHERE b = 5",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(ColumnRef(ColumnRef { fields: Some([A_Star(A_Star)]), location: 61 })), location: 61 })]), from_clause: Some([RangeVar(RangeVar { catalogname: None, schemaname: None, relname: Some(\"a\"), inh: true, relpersistence: 'p', alias: None, location: 68 })]), where_clause: Some(A_Expr(A_Expr { kind: AEXPR_OP, name: Some([String { value: Some(\"=\") }]), lexpr: Some(ColumnRef(ColumnRef { fields: Some([String { value: Some(\"b\") }]), location: 76 })), rexpr: Some(A_Const(A_Const { val: Value(Integer { value: 5 }), location: 80 })), location: 78 })), group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: Some(WithClause { ctes: Some([CommonTableExpr(CommonTableExpr { ctename: Some(\"a\"), aliascolnames: None, ctematerialized: CTEMaterializeDefault, ctequery: Some(SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(ColumnRef(ColumnRef { fields: Some([A_Star(A_Star)]), location: 18 })), location: 18 })]), from_clause: Some([RangeVar(RangeVar { catalogname: None, schemaname: None, relname: Some(\"x\"), inh: true, relpersistence: 'p', alias: None, location: 25 })]), where_clause: Some(BoolExpr(BoolExpr { boolop: AND_EXPR, args: Some([A_Expr(A_Expr { kind: AEXPR_OP, name: Some([String { value: Some(\"=\") }]), lexpr: Some(ColumnRef(ColumnRef { fields: Some([String { value: Some(\"x\") }, String { value: Some(\"y\") }]), location: 33 })), rexpr: Some(ParamRef(ParamRef { number: 0, location: 39 })), location: 37 }), A_Expr(A_Expr { kind: AEXPR_OP, name: Some([String { value: Some(\"=\") }]), lexpr: Some(ColumnRef(ColumnRef { fields: Some([String { value: Some(\"x\") }, String { value: Some(\"z\") }]), location: 45 })), rexpr: Some(A_Const(A_Const { val: Value(Integer { value: 1 }), location: 51 })), location: 49 })]), location: 41 })), group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })), location: 5, cterecursive: false, cterefcount: 0, ctecolnames: None, ctecoltypes: None, ctecoltypmods: None, ctecolcollations: None })]), recursive: false, location: 0 }), op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("SELECT count(*) from testjsonb  WHERE j->'array' ? 'bar'",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(FuncCall(FuncCall { funcname: Some([String { value: Some(\"count\") }]), args: None, agg_order: None, agg_filter: None, agg_within_group: false, agg_star: true, agg_distinct: false, func_variadic: false, over: None, location: 7 })), location: 7 })]), from_clause: Some([RangeVar(RangeVar { catalogname: None, schemaname: None, relname: Some(\"testjsonb\"), inh: true, relpersistence: 'p', alias: None, location: 21 })]), where_clause: Some(A_Expr(A_Expr { kind: AEXPR_OP, name: Some([String { value: Some(\"?\") }]), lexpr: Some(A_Expr(A_Expr { kind: AEXPR_OP, name: Some([String { value: Some(\"->\") }]), lexpr: Some(ColumnRef(ColumnRef { fields: Some([String { value: Some(\"j\") }]), location: 38 })), rexpr: Some(A_Const(A_Const { val: Value(String { value: Some(\"array\") }), location: 41 })), location: 39 })), rexpr: Some(A_Const(A_Const { val: Value(String { value: Some(\"bar\") }), location: 51 })), location: 49 })), group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("SELECT DISTINCT a FROM b",
        "[SelectStmt(SelectStmt { distinct_clause: Some([]), into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(ColumnRef(ColumnRef { fields: Some([String { value: Some(\"a\") }]), location: 16 })), location: 16 })]), from_clause: Some([RangeVar(RangeVar { catalogname: None, schemaname: None, relname: Some(\"b\"), inh: true, relpersistence: 'p', alias: None, location: 23 })]), where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("SELECT * FROM generate_series(1, 2)",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(ColumnRef(ColumnRef { fields: Some([A_Star(A_Star)]), location: 7 })), location: 7 })]), from_clause: Some([RangeFunction(RangeFunction { lateral: false, ordinality: false, is_rowsfrom: false, functions: Some([List(List { items: [FuncCall(FuncCall { funcname: Some([String { value: Some(\"generate_series\") }]), args: Some([A_Const(A_Const { val: Value(Integer { value: 1 }), location: 30 }), A_Const(A_Const { val: Value(Integer { value: 2 }), location: 33 })]), agg_order: None, agg_filter: None, agg_within_group: false, agg_star: false, agg_distinct: false, func_variadic: false, over: None, location: 14 })] })]), alias: None, coldeflist: None })]), where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]"),
        ("SELECT 1 + 1",
        "[SelectStmt(SelectStmt { distinct_clause: None, into_clause: None, target_list: Some([ResTarget(ResTarget { name: None, indirection: None, val: Some(A_Expr(A_Expr { kind: AEXPR_OP, name: Some([String { value: Some(\"+\") }]), lexpr: Some(A_Const(A_Const { val: Value(Integer { value: 1 }), location: 7 })), rexpr: Some(A_Const(A_Const { val: Value(Integer { value: 1 }), location: 11 })), location: 9 })), location: 7 })]), from_clause: None, where_clause: None, group_clause: None, having_clause: None, window_clause: None, values_lists: None, sort_clause: None, limit_offset: None, limit_count: None, limit_option: LIMIT_OPTION_DEFAULT, locking_clause: None, with_clause: None, op: SETOP_NONE, all: false, larg: None, rarg: None })]")
    ];

    for (expr, tree) in TESTS {
        println!("{}", expr);
        let parsed = pg_parse::parse(expr);
        assert!(parsed.is_ok(), "Failed to parse: {}", expr);
        assert_eq!(format!("{:?}", parsed.unwrap()), tree, "Expr: {}", expr);
    }
}
