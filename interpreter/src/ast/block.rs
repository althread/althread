use std::fmt;

use pest::iterators::Pairs;

use crate::compiler::State;
use crate::{env::process_env::ProcessEnv, error::AlthreadResult, parser::Rule};
use crate::env::instruction::{Instruction, InstructionType, ProcessCode};

use super::{
    display::{AstDisplay, Prefix},
    node::{InstructionBuilder, Node, NodeBuilder, NodeExecutor},
    statement::Statement,
    token::literal::Literal,
};

#[derive(Debug)]
pub struct Block {
    pub children: Vec<Node<Statement>>,
}

impl NodeBuilder for Block {
    fn build(pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let mut block = Self::new();

        for pair in pairs {
            let node = Node::build(pair)?;
            block.children.push(node);
        }

        Ok(block)
    }
}

impl NodeExecutor for Block {
    fn eval(&self, env: &mut ProcessEnv) -> AlthreadResult<Option<Literal>> {
        let node = &self.children[env.position];
        if node.eval(env.get_child())?.is_some() {
            env.consume();
        }

        Ok((env.position >= self.children.len()).then(|| Literal::Null))
    }
}

impl InstructionBuilder for Block {
    fn compile(&self, state: &mut State) -> Vec<Instruction> {
        let mut instructions = Vec::new();
        for node in &self.children {
            instructions.extend(node.compile(state));
        }
        instructions
    }
}

impl Block {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
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
