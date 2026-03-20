use std::fmt;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node},
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::{AlthreadError, AlthreadResult, ErrorType},
    vm::instruction::{Instruction, InstructionType},
};

use super::Statement;

#[derive(Debug, Clone)]
pub struct Atomic {
    pub statement: Box<Node<Statement>>,
    pub delegated: bool,
}

impl InstructionBuilder for Node<Atomic> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        if state.is_atomic {
            return Err(AlthreadError::new(
                ErrorType::InstructionNotAllowed,
                Some(self.value.statement.as_ref().pos.clone()),
                "Atomic blocks cannot be nested".to_string(),
            ));
        }

        let mut builder = InstructionBuilderOk::new();

        if !self.value.delegated {
            builder.instructions.push(Instruction {
                pos: Some(self.value.statement.as_ref().pos.clone()),
                control: InstructionType::AtomicStart,
            });
            state.is_atomic = true;
        }

        builder.extend(self.value.statement.as_ref().compile(state)?);

        state.is_atomic = false;
        builder.instructions.push(Instruction {
            pos: Some(self.value.statement.as_ref().pos.clone()),
            control: InstructionType::AtomicEnd,
        });
        if builder.contains_jump() {
            for idx in builder.break_indexes.get("").unwrap_or(&Vec::new()) {
                if let InstructionType::Break { stop_atomic, .. } =
                    &mut builder.instructions[*idx as usize].control
                {
                    *stop_atomic = true;
                } else {
                    panic!("Expected Break instruction");
                }
            }
            for idx in builder.continue_indexes.get("").unwrap_or(&Vec::new()) {
                if let InstructionType::Break { stop_atomic, .. } =
                    &mut builder.instructions[*idx as usize].control
                {
                    *stop_atomic = true;
                } else {
                    panic!("Expected Break instruction");
                }
            }
        }
        Ok(builder)
    }
}

impl AstDisplay for Atomic {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}atomic")?;

        let prefix = prefix.switch();
        {
            let prefix = prefix.add_leaf();
            self.statement.as_ref().ast_fmt(f, &prefix)?;
        }

        Ok(())
    }
}
