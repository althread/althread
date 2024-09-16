use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        block::Block,
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::datatype::DataType,
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::{AlthreadError, AlthreadResult, ErrorType},
    parser::Rule,
    vm::instruction::{Instruction, InstructionType, JumpControl, JumpIfControl},
};

use super::expression::Expression;

#[derive(Debug, Clone)]
pub struct WhileControl {
    pub condition: Node<Expression>,
    pub then_block: Box<Node<Block>>,
}

impl NodeBuilder for WhileControl {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let condition = Node::build(pairs.next().unwrap())?;
        let then_block = Node::build(pairs.next().unwrap())?;

        Ok(Self {
            condition,
            then_block: Box::new(then_block),
        })
    }
}

impl InstructionBuilder for Node<WhileControl> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();

        state.current_stack_depth += 1;
        let cond_builder = self.value.condition.compile(state)?;
        // Check if the top of the stack is a boolean
        if state
            .program_stack
            .last()
            .expect("stack should contain a value after an expression is compiled")
            .datatype
            != DataType::Boolean
        {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(self.value.condition.pos),
                "condition must be a boolean".to_string(),
            ));
        }
        // pop all variables from the stack at the given depth
        let unstack_len = state.unstack_current_depth();

        let block_builder = self.value.then_block.compile(state)?;
        let block_len = block_builder.instructions.len();

        builder.extend(cond_builder);
        builder.instructions.push(Instruction {
            pos: Some(self.value.condition.pos),
            control: InstructionType::JumpIf(JumpIfControl {
                jump_false: (block_len + 2) as i64,
                unstack_len,
            }),
        });
        builder.extend(block_builder);

        builder.instructions.push(Instruction {
            pos: Some(self.pos),
            control: InstructionType::Jump(JumpControl {
                jump: -(builder.instructions.len() as i64),
            }),
        });
        if builder.contains_jump() {
            unimplemented!("breaks in while blocks are not yet implemented (yeah, I know, its weird)");
        }
        Ok(builder)
    }
}

impl AstDisplay for WhileControl {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}while_control")?;

        let prefix = prefix.add_branch();
        writeln!(f, "{prefix}condition")?;
        {
            let prefix = prefix.add_leaf();
            self.condition.ast_fmt(f, &prefix)?;
        }

        let prefix = prefix.switch();
        writeln!(f, "{prefix}then")?;
        {
            let prefix = prefix.add_leaf();
            self.then_block.ast_fmt(f, &prefix)?;
        }

        Ok(())
    }
}
