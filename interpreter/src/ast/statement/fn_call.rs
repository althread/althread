use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{Node, NodeBuilder, NodeExecutor},
        token::{identifier::Identifier, literal::Literal},
    },
    env::process_env::ProcessEnv,
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

impl NodeExecutor for FnCall {
    fn eval(&self, env: &mut ProcessEnv) -> AlthreadResult<Option<Literal>> {
        
        if self.fn_name.value.value != "print" {
            return Err(AlthreadError::new(
                ErrorType::RuntimeError,
                self.fn_name.line,
                self.fn_name.column,
                format!("Function {} not found", self.fn_name.value.value),
            ));
        }

        if let Some(value) = self.value.eval(env)? {
            println!("{}", value);
        }

        Ok(Some(Literal::Null))
    }
}

impl AstDisplay for FnCall {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}print")?;
        self.value.ast_fmt(f, &prefix.add_leaf())?;

        Ok(())
    }
}
