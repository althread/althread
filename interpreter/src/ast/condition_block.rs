use std::fmt;

use pest::iterators::Pairs;

use crate::compiler::CompilerState;
use crate::error::AlthreadResult;
use crate::parser::Rule;
use crate::vm::instruction::{Instruction, InstructionType, UnstackControl};

use super::statement::expression::Expression;
use super::{
    display::{AstDisplay, Prefix},
    node::{InstructionBuilder, Node, NodeBuilder},
};

#[derive(Debug)]
pub struct ConditionBlock {
    pub children: Vec<Node<Expression>>,
}

impl NodeBuilder for ConditionBlock {
    fn build(pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let mut block = Self::new();

        for pair in pairs {
            let node = Node::build(pair)?;
            block.children.push(node);
        }

        Ok(block)
    }
}

impl InstructionBuilder for ConditionBlock {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {
        let mut instructions = Vec::new();
        state.current_stack_depth += 1;
        for node in &self.children {
            let n_ins = node.compile(state)?;
            instructions.extend(n_ins);
        }
        let unstack_len = state.unstack_current_depth();
        if unstack_len > 0 {
            instructions.push(Instruction {
                control: InstructionType::Unstack(UnstackControl {
                    unstack_len
                }),
                pos: None,
            });
        }
        Ok(instructions)
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
