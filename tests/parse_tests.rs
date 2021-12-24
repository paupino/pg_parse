use pg_query::ast::{InsertStmt, List, Node, ParamRef, SelectStmt};

#[test]
fn it_can_generate_a_create_index_ast() {
    let result =
        pg_query::parse("CREATE INDEX ix_test ON contacts.person (id, ssn) WHERE ssn IS NOT NULL;");
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
        pg_query::parse("CREATE TABLE contacts.person(id serial primary key, name text not null, balance numeric(5, 12));");
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
                Node::A_Const(pg_query::ast::A_Const { val, .. }) => {
                    let constant = match **val {
                        pg_query::ast::Value(Node::Integer { value }) => value,
                        _ => panic!("Expected value"),
                    };
                    assert_eq!(constant, 5);
                }
                unexpected => panic!("Unexpected type for mods[0] {:?}", unexpected),
            }
            match &mods[1] {
                Node::A_Const(pg_query::ast::A_Const { val, .. }) => {
                    let constant = match **val {
                        pg_query::ast::Value(Node::Integer { value }) => value,
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
    let result = pg_query::parse("CREATE RANDOM ix_test ON contacts.person;");
    assert!(result.is_err());
    assert_eq!(
        result.err().unwrap(),
        pg_query::Error::ParseError("syntax error at or near \"RANDOM\"".into())
    );
}

#[test]
fn it_can_parse_lists_of_values() {
    let result = pg_query::parse("INSERT INTO contacts.person(name, ssn) VALUES ($1, $2)");
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
