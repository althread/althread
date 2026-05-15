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
            if let Some(special_builder) =
                compile_block_with_leading_wait(self.value.statement.as_ref(), state)?
            {
                builder.extend(special_builder);
                patch_atomic_jumps(&mut builder);
                return Ok(builder);
            }
        }

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
        patch_atomic_jumps(&mut builder);
        Ok(builder)
    }
}

fn compile_block_with_leading_wait(
    statement: &Node<Statement>,
    state: &mut CompilerState,
) -> AlthreadResult<Option<InstructionBuilderOk>> {
    let Statement::Block(block_node) = &statement.value else {
        return Ok(None);
    };

    let Some((first, rest)) = block_node.value.children.split_first() else {
        return Ok(None);
    };

    let Statement::Wait(wait_node) = &first.value else {
        return Ok(None);
    };

    let mut builder = InstructionBuilderOk::new();
    state.current_stack_depth += 1;

    let mut leading_wait = wait_node.clone();
    leading_wait.value.start_atomic = true;
    builder.extend(leading_wait.compile(state)?);

    state.is_atomic = true;
    for child in rest {
        builder.extend(child.compile(state)?);
    }

    let unstack_len = state.unstack_current_depth_with_debug(&mut builder);
    if unstack_len > 0 {
        builder.instructions.push(Instruction {
            control: InstructionType::Unstack { unstack_len },
            pos: None,
        });
    }

    state.is_atomic = false;
    builder.instructions.push(Instruction {
        pos: Some(statement.pos.clone()),
        control: InstructionType::AtomicEnd,
    });

    Ok(Some(builder))
}

fn patch_atomic_jumps(builder: &mut InstructionBuilderOk) {
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
