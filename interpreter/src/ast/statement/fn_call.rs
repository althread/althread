use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::identifier::Identifier,
    },
    compiler::CompilerState,
    error::{AlthreadError, AlthreadResult, ErrorType},
    parser::Rule,
    vm::instruction::{FnCallControl, Instruction, InstructionType},
};

use super::expression::Expression;

#[derive(Debug, Clone)]
pub struct FnCall {
    pub fn_name: Node<Identifier>,
    pub values: Node<Expression>,
}

impl NodeBuilder for FnCall {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let fn_name = Node::build(pairs.next().unwrap())?;

        let values: Node<Expression> = Expression::build_top_level(pairs.next().unwrap())?;

        Ok(Self { fn_name, values })
    }
}

impl InstructionBuilder for Node<FnCall> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {
        let name = self.value.fn_name.value.value.clone();
        if name != "print" {
            return Err(AlthreadError::new(
                ErrorType::UndefinedFunction,
                Some(self.pos),
                "undefined function".to_string(),
            ));
        }

        let mut instructions = Vec::new();

        state.current_stack_depth += 1;
        instructions.append(&mut self.value.values.compile(state)?);
        let unstack_len = state.unstack_current_depth();
        instructions.push(Instruction {
            control: InstructionType::FnCall(FnCallControl { name, unstack_len }),
            pos: Some(self.pos),
        });
        Ok(instructions)
    }
}

impl AstDisplay for FnCall {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}print")?;
        self.values.ast_fmt(f, &prefix.add_leaf())?;

        Ok(())
    }
}
