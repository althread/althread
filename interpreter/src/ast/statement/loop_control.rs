use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix}, node::{InstructionBuilder, Node, NodeBuilder}, 
    }, compiler::CompilerState, error::AlthreadResult, parser::Rule, vm::instruction::{Instruction, InstructionType, JumpControl}
};

use super::Statement;

#[derive(Debug, Clone)]
pub struct LoopControl {
    pub statement: Box<Node<Statement>>,
}

impl NodeBuilder for LoopControl {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let statement = Box::new(Node::build(pairs.next().unwrap())?);

        Ok(Self {
            statement
        })
    }
}


impl InstructionBuilder for Node<LoopControl> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {

        let mut instructions = self.value.statement.as_ref().compile(state)?;
 
        instructions.push(Instruction {
            pos: Some(self.value.statement.as_ref().pos),
            control: InstructionType::Jump(JumpControl { 
                jump: -(instructions.len() as i64),
            }),
        });

        Ok(instructions)
    }
}



impl AstDisplay for LoopControl {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}loop_control")?;

        let prefix = prefix.switch();
        writeln!(f, "{prefix}do")?;
        {
            let prefix = prefix.add_leaf();
            self.statement.as_ref().ast_fmt(f, &prefix)?;
        }

        Ok(())
    }
}
