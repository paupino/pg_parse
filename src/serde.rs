use std::fmt;

use serde::de::{Deserializer, Error, SeqAccess, Visitor};

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

#[cfg(test)]
mod tests {
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
        let json = "{ \"values\": [{ \"Null\": {} }, {}] }";
        let nodes: Nodes = serde_json::from_str(json).unwrap();
        assert_eq!(1, nodes.values.len());
        assert!(matches!(nodes.values[0], Node::Null {}))
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
        let json = "{ \"values\": [{ \"Null\": {} }, {}] }";
        let nodes: OptionalNodes = serde_json::from_str(json).unwrap();
        assert!(nodes.values.is_some());
        let values = nodes.values.unwrap();
        assert_eq!(1, values.len());
        assert!(matches!(values[0], Node::Null {}))
    }

    #[test]
    fn it_can_deserialize_an_optional_node_array_with_some_empty_array() {
        let json = "{\"values\":[{}]}";
        let nodes: OptionalNodes = serde_json::from_str(json).unwrap();
        assert!(nodes.values.is_some());
        let values = nodes.values.unwrap();
        assert_eq!(0, values.len());
    }
}
