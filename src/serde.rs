use std::fmt;

use crate::ast::ConstValue;
use serde::de::{Deserializer, Error, SeqAccess, Visitor};
use serde::Deserialize;

pub(crate) fn deserialize_node_array<'de, D>(
    deserializer: D,
) -> Result<Vec<crate::ast::Node>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum NodeOrError {
        Node(Box<crate::ast::Node>),
        // This consumes one "item" when `T` errors while deserializing.
        // This is necessary to make this work, when instead of having a direct value
        // like integer or string, the deserializer sees a list or map.
        Error(serde::de::IgnoredAny),
    }

    struct NodeArray;
    impl<'de> Visitor<'de> for NodeArray {
        type Value = Vec<crate::ast::Node>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("Vec<Node>")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut values = Vec::with_capacity(seq.size_hint().unwrap_or_default());

            while let Some(value) = seq.next_element()? {
                if let NodeOrError::Node(value) = value {
                    values.push(*value);
                }
            }
            Ok(values)
        }
    }

    deserializer.deserialize_seq(NodeArray)
}

pub(crate) fn deserialize_node_array_opt<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<crate::ast::Node>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct NodeArrayOpt;
    impl<'de> Visitor<'de> for NodeArrayOpt {
        type Value = Option<Vec<crate::ast::Node>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("Option<Vec<Node>>")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            let value = deserialize_node_array(deserializer)?;
            Ok(Some(value))
        }
    }

    deserializer.deserialize_option(NodeArrayOpt)
}

impl<'de> serde::Deserialize<'de> for ConstValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ConstValueVisitor;

        impl<'de> Visitor<'de> for ConstValueVisitor {
            type Value = ConstValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("ConnectorTopics")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                #[derive(Deserialize)]
                struct BoolValue {
                    boolval: bool,
                }
                #[derive(Deserialize)]
                struct IntValue {
                    ival: i64,
                }

                #[derive(Deserialize)]
                struct FloatValue {
                    fval: String,
                }

                #[derive(Deserialize)]
                struct StringValue {
                    sval: String,
                }

                #[derive(Deserialize)]
                struct BitStringValue {
                    bsval: String,
                }

                fn maybe_location<'de, V>(mut inner: V) -> Result<(), V::Error>
                where
                    V: serde::de::MapAccess<'de>,
                {
                    // We may have a location after this which we need to consume
                    if let Some(_location) = inner.next_key::<String>()? {
                        let _pos = inner.next_value::<i32>()?;
                    }
                    Ok(())
                }

                if let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "boolval" => {
                            let value = map.next_value::<BoolValue>()?;
                            maybe_location(map)?;
                            Ok(ConstValue::Bool(value.boolval))
                        }
                        "ival" => {
                            let value = map.next_value::<IntValue>()?;
                            maybe_location(map)?;
                            Ok(ConstValue::Integer(value.ival))
                        }
                        "fval" => {
                            let value = map.next_value::<FloatValue>()?;
                            maybe_location(map)?;
                            Ok(ConstValue::Float(value.fval))
                        }
                        "sval" => {
                            let value = map.next_value::<StringValue>()?;
                            maybe_location(map)?;
                            Ok(ConstValue::String(value.sval))
                        }
                        "bsval" => {
                            let value = map.next_value::<BitStringValue>()?;
                            maybe_location(map)?;
                            Ok(ConstValue::BitString(value.bsval))
                        }
                        "location" => {
                            let _location = map.next_value::<i32>()?;
                            Ok(ConstValue::Null)
                        }
                        unknown => Err(Error::unknown_field(
                            unknown,
                            &["boolval", "ival", "fval", "sval", "bsval"],
                        )),
                    }
                } else {
                    Err(Error::custom("expected value"))
                }
            }
        }

        deserializer.deserialize_map(ConstValueVisitor {})
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::ConstValue;
    use crate::ast::Node;
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct Nodes {
        #[serde(deserialize_with = "crate::serde::deserialize_node_array")]
        values: Vec<Node>,
    }

    #[derive(Deserialize)]
    pub struct OptionalNodes {
        #[serde(deserialize_with = "crate::serde::deserialize_node_array_opt", default)]
        values: Option<Vec<Node>>,
    }

    #[test]
    fn it_can_deserialize_a_node_array() {
        let json = "{ \"values\": [{ \"A_Const\": { \"ival\": { \"ival\": 10 }, \"location\": 253 } }, {}] }";
        let nodes: Nodes = serde_json::from_str(json).unwrap();
        assert_eq!(1, nodes.values.len());
        assert!(matches!(
            nodes.values[0],
            Node::A_Const {
                isnull: None,
                val: ConstValue::Integer(10)
            }
        ))
    }

    #[test]
    fn it_can_deserialize_an_optional_node_array_with_missing_property() {
        let json = "{ }";
        let nodes: OptionalNodes = serde_json::from_str(json).unwrap();
        assert!(nodes.values.is_none());
    }

    #[test]
    fn it_can_deserialize_an_optional_node_array_with_null() {
        let json = "{ \"values\": null }";
        let nodes: OptionalNodes = serde_json::from_str(json).unwrap();
        assert!(nodes.values.is_none());
    }

    #[test]
    fn it_can_deserialize_an_optional_node_array_with_some() {
        let json = "{ \"values\": [{ \"Boolean\": { \"boolval\": false } }, {}] }";
        let nodes: OptionalNodes = serde_json::from_str(json).unwrap();
        assert!(nodes.values.is_some());
        let values = nodes.values.unwrap();
        assert_eq!(1, values.len());
        assert!(matches!(values[0], Node::Boolean { boolval: false }))
    }

    #[test]
    fn it_can_deserialize_an_optional_node_array_with_some_empty_array() {
        let json = "{\"values\":[{}]}";
        let nodes: OptionalNodes = serde_json::from_str(json).unwrap();
        assert!(nodes.values.is_some());
        let values = nodes.values.unwrap();
        assert_eq!(0, values.len());
    }

    #[test]
    fn it_can_deserialize_const_with_location() {
        let tests = [
            (
                "{ \"ival\": { \"ival\": 10 }, \"location\": 253 }",
                ConstValue::Integer(10),
            ),
            (
                "{ \"boolval\": { \"boolval\": true }, \"location\": 253 }",
                ConstValue::Bool(true),
            ),
            (
                "{ \"fval\": { \"fval\": \"1.23\" }, \"location\": 253 }",
                ConstValue::Float("1.23".to_string()),
            ),
            (
                "{ \"sval\": { \"sval\": \"hello\" }, \"location\": 253 }",
                ConstValue::String("hello".to_string()),
            ),
            (
                "{ \"bsval\": { \"bsval\": \"b123\" }, \"location\": 253 }",
                ConstValue::BitString("b123".to_string()),
            ),
        ];
        for (json, test) in &tests {
            let deserialized: ConstValue =
                serde_json::from_str(json).expect("Failed to deserialize");
            assert_eq!(deserialized, *test, "Failed to deserialize: {}", json);
        }
    }

    #[test]
    fn it_can_deserialize_const_without_location() {
        let tests = [
            ("{ \"ival\": { \"ival\": 10 } }", ConstValue::Integer(10)),
            (
                "{ \"boolval\": { \"boolval\": true } }",
                ConstValue::Bool(true),
            ),
            (
                "{ \"fval\": { \"fval\": \"1.23\" } }",
                ConstValue::Float("1.23".to_string()),
            ),
            (
                "{ \"sval\": { \"sval\": \"hello\" } }",
                ConstValue::String("hello".to_string()),
            ),
            (
                "{ \"bsval\": { \"bsval\": \"b123\" } }",
                ConstValue::BitString("b123".to_string()),
            ),
        ];
        for (json, test) in &tests {
            let deserialized: ConstValue =
                serde_json::from_str(json).expect("Failed to deserialize");
            assert_eq!(deserialized, *test, "Failed to deserialize: {}", json);
        }
    }

    #[test]
    fn it_can_deserialize_a_const() {
        // The structure of a_const changed dramatically which broke the deserialization.
        // Effectively, the definition of this is:
        //     "A_Const": {
        //       "fields": [
        //         {
        //           "name": "isnull",
        //           "c_type": "bool"
        //         },
        //         {
        //           "name": "val",
        //           "c_type": "Node"
        //         }
        //       ]
        //     }
        // The reality, however, is that nodes get sent in like:
        //   // Works ok
        //   "A_Const":
        //   {
        //     "isnull": true,
        //     "location": 323
        //   }
        //
        //   // Does not work ok
        //   "A_Const":
        //   {
        //        "ival":
        //        {
        //          "ival": 1
        //        },
        //        "location": 123
        //   }
        // Consequently, this test covers these cases
        let null_json = "{ \"A_Const\": { \"isnull\": true, \"location\": 323 } }";
        let null_const: Node = serde_json::from_str(null_json).expect("Failed to deserialize");
        let Node::A_Const { isnull, val } = null_const else {
            panic!("Expected A_Const node: {:#?}", null_const);
        };
        assert!(isnull.is_some(), "Expected isnull to be Some");
        assert!(isnull.unwrap(), "Expected isnull to be true");
        // assert_eq!(val, None, "Expected val to be None");

        let ival_json = "{ \"A_Const\": { \"ival\": { \"ival\": 1 }, \"location\": 123 } }";
        let ival_const: Node = serde_json::from_str(ival_json).expect("Failed to deserialize");
        let Node::A_Const { isnull, val } = ival_const else {
            panic!("Expected A_Const node: {:#?}", ival_const);
        };
        assert!(isnull.is_none(), "Expected isnull to be None");
        assert_eq!(val, ConstValue::Integer(1), "Expected val to be an integer");
    }

    #[test]
    fn it_can_parse_empty_nodes() {
        let json = "{\"Integer\":{}}";
        let node: Node = serde_json::from_str(json).unwrap();
        // This defaults to 0 - this is because it exits libpg_query like this, even for zero's.
        // We should keep an eye on this as 0 could be different than absence of data in the future.
        assert!(
            matches!(node, Node::Integer { ival: 0 }),
            "Expected integer node to default to 0"
        );
    }
}
