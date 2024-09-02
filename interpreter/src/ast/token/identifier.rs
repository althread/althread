use pest::iterators::Pairs;

use crate::{
    ast::node::{Node, NodeBuilder},
    
    error::{AlthreadError, AlthreadResult, ErrorType},
    no_rule,
    parser::Rule,
};

use super::literal::Literal;

pub type Identifier = Node<String>;

impl NodeBuilder for Identifier {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        match pair.as_rule() {
            Rule::IDENT => Ok(Node {
                line: pair.line_col().0,
                column: pair.line_col().1,
                value: pair.as_str().to_string(),
            }),
            _ => Err(no_rule!(pair)),
        }
    }
}

