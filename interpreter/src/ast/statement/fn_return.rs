use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::{datatype::DataType, identifier::Identifier},
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType},
    parser::Rule,
    vm::instruction::{Instruction, InstructionType},
};

use super::expression::Expression;

#[derive(Debug, Clone)]
pub struct FnReturn {
    pub value: Node<Expression>,
}

impl NodeBuilder for FnReturn {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        let value = Expression::build_top_level(pair)?;
        Ok(Self { value })
    }
}

impl InstructionBuilder for FnReturn {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        if !state.in_function {
            return Err(AlthreadError::new(
                ErrorType::ReturnOutsideFunction,
                Some(self.value.pos),
                "Return statement outside function".to_string(),
            ));
        }
        
        todo!("Not implemented yet");
    }
}

impl AstDisplay for FnReturn {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}return")?;
        let prefix = prefix.add_branch();

        let prefix = prefix.switch();
        writeln!(f, "{}value:", &prefix)?;
        self.value.ast_fmt(f, &prefix.add_leaf())?;

        Ok(())
    }
}