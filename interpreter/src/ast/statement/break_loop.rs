use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, NodeBuilder},
    }, compiler::{CompilerState, InstructionBuilderOk}, error::AlthreadResult, no_rule, parser::Rule, vm::instruction::{self, Instruction, InstructionType}
};





#[derive(Debug, Clone)]
enum BreakLoopType {
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub struct BreakLoopControl {
    pub kind: BreakLoopType,
    pub label: Option<String>,
}


impl NodeBuilder for BreakLoopControl {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        let kind = match pair.as_rule() {
            Rule::BREAK_KW => BreakLoopType::Break,
            Rule::CONTINUE_KW => BreakLoopType::Continue,
            _ => return Err(no_rule!(pair, "BreakLoopControl")),
        };

        let label = pairs.next().map(|pair| pair.as_str().to_string());

        Ok(Self { kind, label })
    }
}

impl InstructionBuilder for BreakLoopControl {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();

        match self.kind {
            BreakLoopType::Break => {
                builder.break_indexes.insert(self.label.clone().unwrap_or_default(), vec![0]);
            }
            BreakLoopType::Continue => {
                builder.continue_indexes.insert(self.label.clone().unwrap_or_default(), vec![0]);
            }
        }
        builder.instructions.push(Instruction {
            pos: None,
            control: InstructionType::Break(instruction::BreakLoopControl {
                jump: 0,
                unstack_len: state.program_stack.len(),
                stop_atomic: false,
            }),
        });

        Ok(builder)
    }
}

impl AstDisplay for BreakLoopControl {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}{kind}", prefix = prefix, kind = match self.kind {
            BreakLoopType::Break => "break",
            BreakLoopType::Continue => "continue",
        })?;

        if let Some(label) = &self.label {
            let prefix = prefix.add_leaf();
            writeln!(f, "{prefix}label: {label}", prefix = prefix, label = label)?;
        }

        Ok(())
    }
}