use std::fmt;

use crate::compiler::InstructionBuilderOk;
use crate::error::Pos;
use crate::{compiler::CompilerState, error::AlthreadResult};

use super::display::{AstDisplay, Prefix};

#[derive(Debug, PartialEq, Clone)]
pub struct Node<T> {
    pub value: T,
    pub pos: Pos,
}

pub trait InstructionBuilder: Sized {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk>;
}

impl<T: AstDisplay> AstDisplay for Node<T> {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        self.value.ast_fmt(f, prefix)
    }
}

impl<T: fmt::Display> fmt::Display for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T: InstructionBuilder> Node<T> {
    pub fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        self.value.compile(state)
    }
}
