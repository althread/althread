use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
    },
    compiler::CompilerState,
    error::{AlthreadError, AlthreadResult, ErrorType},
    parser::Rule,
    vm::instruction::{Instruction, InstructionType, JumpControl},
};

use super::Statement;

#[derive(Debug, Clone)]
pub struct Atomic {
    pub statement: Box<Node<Statement>>,
}

impl NodeBuilder for Atomic {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let statement = Box::new(Node::build(pairs.next().unwrap())?);

        Ok(Self { statement })
    }
}

impl InstructionBuilder for Node<Atomic> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {
        let mut instructions= Vec::new();
        
        instructions.push(Instruction {
            pos: Some(self.value.statement.as_ref().pos),
            control: InstructionType::AtomicStart,
        });
        if state.is_atomic {
            return Err(AlthreadError::new(
                ErrorType::InstructionNotAllowed,
                Some(self.value.statement.as_ref().pos),
                "Atomic blocks cannot be nested".to_string(),
            ));
        }
        state.is_atomic = true;

        instructions.extend(self.value.statement.as_ref().compile(state)?);

        state.is_atomic = false;
        instructions.push(Instruction {
            pos: Some(self.value.statement.as_ref().pos),
            control: InstructionType::AtomicEnd,
        });

        Ok(instructions)
    }
}

impl AstDisplay for Atomic {
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
