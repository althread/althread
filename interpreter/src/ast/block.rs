use std::fmt;

use crate::compiler::{CompilerState, InstructionBuilderOk};
use crate::error::AlthreadResult;
use crate::vm::instruction::{Instruction, InstructionType};

use super::{
    display::{AstDisplay, Prefix},
    node::{InstructionBuilder, Node},
    statement::Statement,
};

#[derive(Debug, Clone)]
pub struct Block {
    pub children: Vec<Node<Statement>>,
}

impl Block {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }
}

impl InstructionBuilder for Block {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();
        state.current_stack_depth += 1;
        for node in &self.children {
            let sub_b = node.compile(state)?;
            builder.extend(sub_b);
        }
        let unstack_len = state.unstack_current_depth_with_debug(&mut builder);
        if unstack_len > 0 {
            builder.instructions.push(Instruction {
                control: InstructionType::Unstack { unstack_len },
                pos: None,
            });
        }
        Ok(builder)
    }
}

impl AstDisplay for Block {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        let mut node_count = self.children.len();
        for node in &self.children {
            node_count -= 1;
            if node_count == 0 {
                node.ast_fmt(f, &prefix.switch())?;
            } else {
                node.ast_fmt(f, &prefix)?;
            }
        }

        Ok(())
    }
}
