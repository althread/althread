use std::fmt;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::InstructionBuilder,
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::AlthreadResult,
    vm::instruction::{Instruction, InstructionType},
};

#[derive(Debug, Clone)]
pub enum BreakLoopType {
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub struct BreakLoopControl {
    pub kind: BreakLoopType,
    pub label: Option<String>,
}

impl InstructionBuilder for BreakLoopControl {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();

        match self.kind {
            BreakLoopType::Break => {
                builder
                    .break_indexes
                    .insert(self.label.clone().unwrap_or_default(), vec![0]);
            }
            BreakLoopType::Continue => {
                builder
                    .continue_indexes
                    .insert(self.label.clone().unwrap_or_default(), vec![0]);
            }
        }
        builder.instructions.push(Instruction {
            pos: None,
            control: InstructionType::Break {
                jump: 0,
                unstack_len: state.program_stack.len(),
                stop_atomic: false,
            },
        });

        Ok(builder)
    }
}

impl AstDisplay for BreakLoopControl {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(
            f,
            "{prefix}{kind}",
            prefix = prefix,
            kind = match self.kind {
                BreakLoopType::Break => "break",
                BreakLoopType::Continue => "continue",
            }
        )?;

        if let Some(label) = &self.label {
            let prefix = prefix.add_leaf();
            writeln!(f, "{prefix}label: {label}", prefix = prefix, label = label)?;
        }

        Ok(())
    }
}
