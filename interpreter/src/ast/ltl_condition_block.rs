use std::fmt;

use pest::iterators::Pairs;

use crate::error::AlthreadResult;
use crate::parser::Rule;

use super::statement::expression::LtlExpression;
use super::{
    display::{AstDisplay, Prefix},
    node::{Node, NodeBuilder},
};

#[derive(Debug)]
pub struct LtlConditionBlock {
    pub children: Vec<Node<LtlExpression>>,
}

impl NodeBuilder for LtlConditionBlock {
    fn build(pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let mut block = Self::new();

        for pair in pairs {
            let node = Node::build(pair, filepath)?;
            block.children.push(node);
        }

        Ok(block)
    }
}

impl LtlConditionBlock {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }
}

impl AstDisplay for LtlConditionBlock {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        let mut node_count = self.children.len();
        for node in &self.children {
            node_count -= 1;
            if node_count == 0 {
                node.ast_fmt(f, &prefix.switch())?;
            } else {
                node.ast_fmt(f, &prefix)?;
            }
        }

        Ok(())
    }
}
