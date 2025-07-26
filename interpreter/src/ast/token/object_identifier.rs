use pest::iterators::Pairs;

use crate::{
    ast::{
        node::{Node, NodeBuilder},
        token::identifier::Identifier,
    },
    error::{AlthreadResult, Pos},
    parser::Rule,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectIdentifier {
    pub parts: Vec<Node<Identifier>>,
}

impl NodeBuilder for ObjectIdentifier {
    fn build(pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let parts = pairs
            .map(|pair| {
                // `pair` is an atomic IDENT rule. We cannot call Node::build on it.
                // We must construct the Node<Identifier> manually.
                let span = pair.as_span();
                Ok(Node {
                    pos: Pos {
                        line: span.start_pos().line_col().0,
                        col: span.start_pos().line_col().1,
                        start: span.start(),
                        end: span.end(),
                        file_path: filepath.to_string(),
                    },
                    value: Identifier {
                        value: pair.as_str().to_string(),
                    },
                })
            })
            .collect::<AlthreadResult<Vec<_>>>()?;
        Ok(Self { parts })
    }
}
