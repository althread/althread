use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        block::Block, display::{AstDisplay, Prefix}, node::{InstructionBuilder, Node, NodeBuilder}, token::{datatype::DataType, literal::Literal}
    }, compiler::CompilerState, error::{AlthreadError, AlthreadResult, ErrorType}, parser::Rule, vm::instruction::{Instruction, InstructionType, JumpControl, JumpIfControl, WaitControl}
};

use super::{expression::Expression, Statement};

#[derive(Debug)]
pub struct Wait {
    pub condition: Node<Expression>,
}

impl NodeBuilder for Wait {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let condition = Node::build(pairs.next().unwrap())?;

        Ok(Self {
            condition,
        })
    }
}


impl InstructionBuilder for Node<Wait> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {

        let mut instructions = Vec::new();

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


        Ok(instructions)
    }
}



impl AstDisplay for Wait {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}wait_control")?;

        let prefix = prefix.add_branch();
        writeln!(f, "{prefix}condition")?;
        {
            let prefix = prefix.add_leaf();
            self.condition.ast_fmt(f, &prefix)?;
        }


        Ok(())
    }
}
