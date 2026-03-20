use std::fmt;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node},
        token::identifier::Identifier,
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::AlthreadResult,
    vm::instruction::{Instruction, InstructionType},
};

#[derive(Debug, Clone, PartialEq)]
pub struct LabelStatement {
    pub name: Node<Identifier>,
}

impl InstructionBuilder for LabelStatement {
    fn compile(&self, _state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        Ok(InstructionBuilderOk::from_instructions(vec![Instruction {
            pos: Some(self.name.pos.clone()),
            control: InstructionType::Label {
                name: self.name.value.value.clone(),
            },
        }]))
    }
}

impl AstDisplay for LabelStatement {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}label: {}", self.name.value.value)
    }
}
