use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{Node, NodeBuilder},
        token::{identifier::Identifier, literal::Literal},
    },
    error::{AlthreadError, AlthreadResult, ErrorType},
    parser::Rule,
};

use super::expression::Expression;

#[derive(Debug)]
pub struct FnCall {
    pub fn_name: Node<Identifier>,
    pub value: Node<Expression>,
}

impl NodeBuilder for FnCall {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        
        let fn_name = Node::build(pairs.next().unwrap())?;
        let value = Node::build(pairs.next().unwrap())?;

        Ok(Self { fn_name, value })
    }
}


impl AstDisplay for FnCall {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}print")?;
        self.value.ast_fmt(f, &prefix.add_leaf())?;

        Ok(())
    }
}
