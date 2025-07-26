use std::fmt;

use pest::iterators::{Pair, Pairs};

use crate::compiler::InstructionBuilderOk;
use crate::error::Pos;
use crate::{compiler::CompilerState, error::AlthreadResult, parser::Rule};

use super::display::{AstDisplay, Prefix};

#[derive(Debug, PartialEq, Clone)]
pub struct Node<T> {
    pub value: T,
    pub pos: Pos,
}

pub trait NodeBuilder: Sized {
    fn build(pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self>;
}

pub trait InstructionBuilder: Sized {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk>;
}

impl<T: NodeBuilder> Node<T> {
    pub fn build(pair: Pair<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let (line, col) = pair.line_col();
        Ok(Node {
            pos: Pos {
                start: pair.as_span().start(),
                end: pair.as_span().end(),
                line,
                col,
                file_path: filepath.to_string(),
            },
            value: T::build(pair.into_inner(), filepath)?,
        })
    }
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

