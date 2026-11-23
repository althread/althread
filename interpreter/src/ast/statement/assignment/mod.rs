pub mod binary_assignment;

use std::fmt::{self};

use binary_assignment::BinaryAssignment;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node},
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::AlthreadResult,
};

#[derive(Debug, Clone)]
pub enum Assignment {
    Binary(Node<BinaryAssignment>),
}

impl InstructionBuilder for Assignment {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        match self {
            Self::Binary(node) => node.compile(state),
        }
    }
}

impl AstDisplay for Assignment {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        match self {
            Self::Binary(node) => node.ast_fmt(f, prefix),
        }
    }
}
