use pest::iterators::Pairs;

use crate::{
    ast::node::{Node, NodeBuilder},
    error::{AlthreadResult, Pos},
    no_rule,
    parser::Rule,
};

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

impl NodeBuilder for ArgsList {
    fn build(pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let mut identifiers = Vec::new();
        let mut datatypes = Vec::new();
        for pair in pairs {
            match pair.as_rule() {
                Rule::datatype => {
                    datatypes.push(Node::build(pair, filepath)?);
                }
                Rule::identifier => {
                    identifiers.push(Node::build(pair, filepath)?);
                }
                _ => return Err(no_rule!(pair, "ArgsList", filepath)),
            }
        }
        Ok(Self {
            identifiers,
            datatypes,
        })
    }
}
