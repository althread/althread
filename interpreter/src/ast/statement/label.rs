use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::identifier::Identifier,
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::AlthreadResult,
    parser::Rule,
    vm::instruction::{Instruction, InstructionType},
};

#[derive(Debug, Clone, PartialEq)]
pub struct LabelStatement {
    pub name: Node<Identifier>,
}

impl NodeBuilder for LabelStatement {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let name = Node::build(pairs.next().unwrap(), filepath)?;
        Ok(Self { name })
    }
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
