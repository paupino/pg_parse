#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
#![allow(clippy::all)]

// Type aliases
pub type bits32 = u32;

// Generated types
include!(concat!(env!("OUT_DIR"), "/ast.rs"));

#[derive(Debug, serde::Deserialize)]
pub struct Value(pub Node);

impl Value {
    pub fn inner(&self) -> &Node {
        &self.0
    }
}
