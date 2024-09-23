use std::collections::HashSet;
use std::fmt;

use pest::iterators::Pairs;

use crate::ast::statement::waiting_case::WaitDependency;
use crate::ast::token::datatype::DataType;
use crate::ast::token::literal::Literal;
use crate::compiler::Variable;
use crate::error::{AlthreadError, ErrorType, Pos};
use crate::vm::instruction::InstructionType;
use crate::vm::Memory;
use crate::{
    compiler::CompilerState, error::AlthreadResult, parser::Rule, vm::instruction::Instruction,
};

use crate::ast::{
    display::{AstDisplay, Prefix},
    node::{Node, NodeBuilder},
    statement::expression::Expression,
};

use super::LocalExpressionNode;

#[derive(Debug, PartialEq, Clone)]
pub struct TupleExpression {
    pub values: Vec<Node<Expression>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LocalTupleExpressionNode {
    pub values: Vec<LocalExpressionNode>,
}

impl fmt::Display for LocalTupleExpressionNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "({})",
            self.values
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

impl NodeBuilder for TupleExpression {
    fn build(pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let mut values = Vec::new();
        for pair in pairs {
            values.push(Node::build(pair)?);
        }
        Ok(Self { values })
    }
}

impl LocalTupleExpressionNode {
    pub fn from_tuple(
        tuple: &TupleExpression,
        program_stack: &Vec<Variable>,
    ) -> AlthreadResult<Self> {
        let mut values = Vec::new();
        for value in &tuple.values {
            values.push(LocalExpressionNode::from_expression(
                &value.value,
                program_stack,
            )?);
        }
        Ok(Self { values })
    }

    pub fn datatype(&self, state: &CompilerState) -> Result<DataType, String> {
        Ok(DataType::Tuple(
            self.values
                .iter()
                .map(|v| v.datatype(state))
                .collect::<Result<Vec<DataType>, String>>()?,
        ))
    }

    pub fn eval(&self, mem: &Memory) -> Result<Literal, String> {
        Ok(Literal::Tuple(
            self.values
                .iter()
                .map(|v| v.eval(mem))
                .collect::<Result<Vec<Literal>, String>>()?,
        ))
    }
}

impl TupleExpression {
    pub fn add_dependencies(&self, dependencies: &mut WaitDependency) {
        for value in &self.values {
            value.value.add_dependencies(dependencies);
        }
    }
    pub fn get_vars(&self, vars: &mut HashSet<String>) {
        for value in &self.values {
            value.value.get_vars(vars);
        }
    }
    pub fn destruct_tuple(
        variable_names: &Vec<String>,
        types: &Vec<DataType>,
        state: &mut CompilerState,
        pos: Pos,
    ) -> AlthreadResult<Instruction> {
        if types.len() != variable_names.len() {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(pos),
                format!(
                    "Tuple has {} values, but {} were expected",
                    variable_names.len(),
                    types.len()
                ),
            ));
        }
        // remove the top of the stack and checking that the types are correct
        let top = state.program_stack.pop().expect("empty stack");
        if top.datatype != DataType::Tuple(types.clone()) {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(pos),
                format!(
                    "Expected tuple of types {:?}, but found {:?}",
                    types, top.datatype
                ),
            ));
        }

        for (i, variable) in variable_names.iter().enumerate() {
            state.program_stack.push(Variable {
                mutable: true,
                name: variable.clone(),
                datatype: types[i].clone(),
                depth: state.current_stack_depth,
                declare_pos: Some(pos),
            })
        }
        Ok(Instruction {
            control: InstructionType::Destruct(types.len()),
            pos: Some(pos),
        })
    }
}

impl AstDisplay for TupleExpression {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}tuple")?;
        for value in &self.values {
            value.ast_fmt(f, &prefix.add_leaf())?;
        }
        Ok(())
    }
}
