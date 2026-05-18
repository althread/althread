use crate::ast::{node::Node, token::identifier::Identifier};

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectIdentifier {
    pub parts: Vec<Node<Identifier>>,
}

impl std::fmt::Display for ObjectIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = self
            .parts
            .iter()
            .map(|p| p.value.value.as_str())
            .collect::<Vec<_>>()
            .join(".");
        write!(f, "{}", s)
    }
}
