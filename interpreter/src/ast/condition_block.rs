use std::fmt;

use crate::compiler::{CompilerState, InstructionBuilderOk};
use crate::error::AlthreadResult;
use crate::vm::instruction::{Instruction, InstructionType};

use super::statement::expression::Expression;
use super::{
    display::{AstDisplay, Prefix},
    node::{InstructionBuilder, Node},
};

#[derive(Debug)]
pub struct ConditionBlock {
    pub children: Vec<Node<Expression>>,
}

impl InstructionBuilder for ConditionBlock {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();
        state.current_stack_depth += 1;
        for node in &self.children {
            let sub_builder = node.compile(state)?;
            builder.extend(sub_builder);
        }
        let unstack_len = state.unstack_current_depth();
        if unstack_len > 0 {
            builder.instructions.push(Instruction {
                control: InstructionType::Unstack { unstack_len },
                pos: None,
            });
        }
        Ok(builder)
    }
}

impl ConditionBlock {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }
}

impl AstDisplay for ConditionBlock {
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
