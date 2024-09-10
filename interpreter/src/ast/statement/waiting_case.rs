use std::fmt;

use pest::iterators::Pairs;

use crate::error::AlthreadResult;
use crate::parser::Rule;

use super::expression::Expression;
use super::super::{
    display::{AstDisplay, Prefix},
    node::{Node, NodeBuilder},
    statement::Statement,
};



#[derive(Debug, Clone)]
pub struct WaitingBlockCase {
    pub expression: Node<Expression>,
    pub statement: Option<Node<Statement>>,
}

impl NodeBuilder for WaitingBlockCase {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {

        let expression = Node::build(pairs.next().unwrap())?;
        
        let statement = match pairs.next() {
            Some(p) => Some(Node::build(p)?),
            None => None,
        };

        Ok(Self {
            expression,
            statement,
        })
    }
}


impl AstDisplay for WaitingBlockCase {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}wait case")?;

        {
            let prefix = prefix.add_branch();
            writeln!(f, "{prefix}condition")?;
            self.expression.ast_fmt(f, &prefix.add_leaf())?;
        }
        if let Some(statement) = &self.statement {
            let prefix = prefix.add_leaf();
            writeln!(f, "{prefix}statement")?;
            statement.ast_fmt(f, &prefix.add_leaf())?;
        }


        Ok(())
    }
}
