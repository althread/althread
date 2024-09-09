use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        block::Block, display::{AstDisplay, Prefix}, node::{InstructionBuilder, Node, NodeBuilder}, token::{datatype::DataType, literal::Literal}
    }, compiler::CompilerState, error::{AlthreadError, AlthreadResult, ErrorType}, parser::Rule, vm::instruction::{Instruction, InstructionType, JumpControl, JumpIfControl, WaitControl}
};

use super::{expression::Expression, waiting_case::WaitingBlockCase, Statement};


#[derive(Debug, Clone)]
pub enum WaitingBlockKind {
    First,
    Seq,
}

#[derive(Debug, Clone)]
pub struct Wait {
    pub block_kind: WaitingBlockKind,
    pub waiting_cases: Vec<Node<WaitingBlockCase>>,
}

impl NodeBuilder for Wait {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        let mut block_kind = WaitingBlockKind::First;
        let waiting_cases = match pair.as_rule() {
            Rule::waiting_block => {
                let mut pair = pair.into_inner();
                block_kind = match pair.next().unwrap().as_rule() {
                    Rule::FIRST_KW => WaitingBlockKind::First,
                    Rule::SEQ_KW => WaitingBlockKind::Seq,
                    _ => unreachable!(),
                };
                let mut children = Vec::new();
                for sub_pair in pair {
                    let node: Node<WaitingBlockCase> = Node::build(sub_pair)?;
                    children.push(node);
                }
                children
            },
            Rule::expression => {
                let node: Node<WaitingBlockCase> = Node::build(pair)?;
                vec![node]
            },
            _ => unreachable!("waiting block should be followed by a waiting block or an expression"),
        };

        Ok(Self {
            block_kind,
            waiting_cases,
        })
    }
}


impl InstructionBuilder for Node<Wait> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {

        let mut instructions = Vec::new();
        todo!("waiting block not implemented");
/*
        state.current_stack_depth += 1;
        let cond_ins = self.value.condition.compile(state)?;
        // Check if the top of the stack is a boolean
        if state.program_stack.last().expect("stack should contain a value after an expression is compiled").datatype != DataType::Boolean {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(self.value.condition.pos),
                "condition must be a boolean".to_string()
            ));
        }
        // pop all variables from the stack at the given depth
        let unstack_len = state.unstack_current_depth();

        instructions.extend(cond_ins);

        instructions.push(Instruction {
            pos: Some(self.pos),
            control: InstructionType::Wait(WaitControl { 
                jump: -(instructions.len() as i64),
                unstack_len,
            }),
        });
*/

        Ok(instructions)
    }
}



impl AstDisplay for Wait {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}wait_control")?;
        {
            for case in &self.waiting_cases {
                case.ast_fmt(f, &prefix.add_branch())?;
            }
        }


        Ok(())
    }
}
