use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::{AlthreadError, AlthreadResult, ErrorType, Pos},
    parser::Rule,
    vm::instruction::{Instruction, InstructionType},
};

use super::expression::Expression;

#[derive(Debug, Clone)]
pub struct FnReturn {
    pub value: Option<Node<Expression>>,
    pub pos: Pos,
}

impl NodeBuilder for FnReturn {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        // return statement doesn't necessarily have a value
        let value = if let Some(pair) = pairs.next() {
            Some(Expression::build_top_level(pair, filepath)?)
        } else {
            None
        };

        // the caller takes care of setting the proper position
        Ok(Self {
            value,
            pos: Pos::default(),
        })
    }
}

impl InstructionBuilder for FnReturn {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        if !state.in_function {
            return Err(AlthreadError::new(
                ErrorType::ReturnOutsideFunction,
                Some(self.pos.clone()),
                "Return statement outside function".to_string(),
            ));
        }

        let mut builder = InstructionBuilderOk::new();
        let mut has_value: bool = false;

        if let Some(ref value_node) = self.value {
            builder.extend(value_node.compile(state)?);
            has_value = true;
        }

        let ret_instr = Instruction {
            control: InstructionType::Return { has_value },
            pos: Some(self.pos.clone()),
        };

        builder.return_indexes.push(builder.instructions.len());

        builder.instructions.push(ret_instr);

        Ok(builder)
    }
}

impl AstDisplay for FnReturn {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}return")?;
        let prefix = prefix.add_branch();

        if let Some(ref value_node) = self.value {
            let prefix_val = prefix.switch();
            writeln!(f, "{}value:", &prefix_val)?;
            value_node.ast_fmt(f, &prefix_val.add_leaf())?;
        } else {
            writeln!(f, "{}(no value)", prefix.switch())?;
        }

        Ok(())
    }
}
