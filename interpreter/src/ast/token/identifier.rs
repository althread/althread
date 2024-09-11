use pest::iterators::Pairs;

use crate::{
    ast::node::{Node, NodeBuilder},
    error::{AlthreadResult, Pos},
    no_rule,
    parser::Rule,
};


pub type Identifier = Node<String>;

impl NodeBuilder for Identifier {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        match pair.as_rule() {
            Rule::IDENT => Ok(Node {
                pos: Pos {
                    line: pair.line_col().0,
                    col: pair.line_col().1,
                    start: pair.as_span().start(),
                    end: pair.as_span().end(),
                },
                value: pair.as_str().to_string(),
            }),
            _ => Err(no_rule!(pair, "Identifier")),
        }
    }
}

