use pg_query::ast::{Node, InsertStmt, List, ParamRef, SelectStmt};

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
        pg_query::parse("CREATE TABLE contacts.person(id serial primary key, name text not null);");
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
            assert_eq!(2, columns.len(), "Columns length");
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
