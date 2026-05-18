use crate::{ast::node::Node, error::Pos};

use super::{datatype::DataType, identifier::Identifier};

#[derive(Debug, Clone)]
pub struct ArgsList {
    pub identifiers: Vec<Node<Identifier>>,
    pub datatypes: Vec<Node<DataType>>,
}

impl Node<ArgsList> {
    pub fn new() -> Self {
        Self {
            pos: Pos::default(),
            value: ArgsList {
                identifiers: Vec::new(),
                datatypes: Vec::new(),
            },
        }
    }
}
