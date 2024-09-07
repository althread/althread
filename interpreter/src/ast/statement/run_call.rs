use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::literal::Literal,
    }, compiler::CompilerState, error::AlthreadResult, parser::Rule, vm::instruction::{Instruction, InstructionType, RunCallControl}
};

#[derive(Debug)]
pub struct RunCall {
    pub identifier: Node<String>,
}

impl NodeBuilder for RunCall {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        let identifier = Node {
            line: pair.line_col().0,
            column: pair.line_col().1,
            value: pair.as_str().to_string(),
        };

        Ok(Self { identifier })
    }
}

impl InstructionBuilder for Node<RunCall> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {
        Ok(vec![Instruction {
            control: InstructionType::RunCall(RunCallControl{
                name: self.value.identifier.value.clone()
            }), 
            line: self.line, 
            column: self.column 
        }])
    }
}

impl AstDisplay for RunCall {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}run: {}", self.identifier)?;

        Ok(())
    }
}