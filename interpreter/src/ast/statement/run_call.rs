use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder}, token::datatype::DataType,
    }, compiler::{CompilerState, Variable}, error::{AlthreadResult, Pos}, parser::Rule, vm::instruction::{Instruction, InstructionType, RunCallControl}
};

#[derive(Debug, Clone)]
pub struct RunCall {
    pub identifier: Node<String>,
}

impl NodeBuilder for RunCall {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        let identifier = Node {
            pos: Pos {
                line: pair.line_col().0,
                col: pair.line_col().1,
                start: pair.as_span().start(),
                end: pair.as_span().end(),
            },
            value: pair.as_str().to_string(),
        };

        Ok(Self { identifier })
    }
}

impl InstructionBuilder for Node<RunCall> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {

        state.program_stack.push(Variable {
            name: "".to_string(),
            depth: state.current_stack_depth,
            mutable: false,
            datatype: DataType::Process(self.value.identifier.value.clone()),
            declare_pos: Some(self.pos),
        });

        Ok(vec![Instruction {
            control: InstructionType::RunCall(RunCallControl{
                name: self.value.identifier.value.clone()
            }), 
            pos: Some(self.pos),
        }])
    }
}

impl AstDisplay for RunCall {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}run: {}", self.identifier)?;

        Ok(())
    }
}
