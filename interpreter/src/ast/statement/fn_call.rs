use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::{datatype::DataType, identifier::Identifier},
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType},
    parser::Rule,
    vm::instruction::{FnCallControl, Instruction, InstructionType},
};

use super::expression::Expression;

#[derive(Debug, Clone)]
pub struct FnCall {
    pub fn_name: Vec<Node<Identifier>>,
    pub values: Node<Expression>,
}

impl NodeBuilder for FnCall {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let mut object_identifier = pairs.next().unwrap();

        let mut fn_name = Vec::new();

        loop {
            let n: Node<Identifier> = Node::build(object_identifier.clone())?;
            fn_name.push(n);

            let mut pairs = object_identifier.into_inner();
            pairs.next().unwrap();
            if let Some(p) = pairs.next() {
                object_identifier = p;
            } else {
                break;
            }
        }

        let values: Node<Expression> = Expression::build_top_level(pairs.next().unwrap())?;

        Ok(Self { fn_name, values })
    }
}

impl InstructionBuilder for Node<FnCall> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();
        state.current_stack_depth += 1;
        builder.extend(self.value.values.compile(state)?);

        let basename = &self.value.fn_name[0].value.value;

        if self.value.fn_name.len() == 1 {
            // this is a top level function

            if basename != "print" {
                return Err(AlthreadError::new(
                    ErrorType::UndefinedFunction,
                    Some(self.pos),
                    "undefined function".to_string(),
                ));
            }
            let unstack_len = state.unstack_current_depth();

            builder.instructions.push(Instruction {
                control: InstructionType::FnCall(FnCallControl {
                    name: basename.to_string(),
                    unstack_len,
                    variable_idx: None,
                    arguments: None, // use the top of the stack
                }),
                pos: Some(self.pos),
            });

            state.program_stack.push(Variable {
                mutable: true,
                name: "".to_string(),
                datatype: DataType::Void,
                depth: state.current_stack_depth,
                declare_pos: None,
            });
        } else {
            // this is a method call

            //get the type of the variable in the stack with this name
            let var_id = state
                .program_stack
                .iter()
                .rev()
                .position(|var| var.name.eq(basename))
                .ok_or(AlthreadError::new(
                    ErrorType::VariableError,
                    Some(self.pos),
                    format!("Variable '{}' not found", basename),
                ))?;
            let var = &state.program_stack[state.program_stack.len() - var_id - 1];

            let interfaces = state.stdlib.interfaces(&var.datatype);

            // retreive the name of the function
            let fn_name = self.value.fn_name.last().unwrap().value.value.clone();

            let fn_idx = interfaces.iter().position(|i| i.name == fn_name);
            if fn_idx.is_none() {
                return Err(AlthreadError::new(
                    ErrorType::UndefinedFunction,
                    Some(self.pos),
                    format!("undefined function {}", fn_name),
                ));
            }
            let fn_idx = fn_idx.unwrap();
            let fn_info = &interfaces[fn_idx];
            let ret_type = fn_info.ret.clone();

            let unstack_len = state.unstack_current_depth();

            state.program_stack.push(Variable {
                mutable: true,
                name: "".to_string(),
                datatype: ret_type,
                depth: state.current_stack_depth,
                declare_pos: None,
            });

            builder.instructions.push(Instruction {
                control: InstructionType::FnCall(FnCallControl {
                    name: fn_name,
                    unstack_len,
                    variable_idx: Some(var_id),
                    arguments: None, // use the top of the stack
                }),
                pos: Some(self.pos),
            });
        }

        Ok(builder)
    }
}

impl AstDisplay for FnCall {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}print")?;
        self.values.ast_fmt(f, &prefix.add_leaf())?;

        Ok(())
    }
}
