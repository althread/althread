use std::{collections::HashSet, fmt, vec};

use pest::iterators::Pairs;

use crate::{ast::{display::{AstDisplay, Prefix}, node::{InstructionBuilder, Node, NodeBuilder}, statement::waiting_case::WaitDependency, token::{datatype::DataType, literal::Literal}}, compiler::{CompilerState, InstructionBuilderOk, Variable}, error::AlthreadResult, no_rule, parser::Rule, vm::{instruction::Instruction, Memory}};

use super::{Expression, LocalExpressionNode};


#[derive(Debug, PartialEq, Clone)]
pub enum ListExpression {
    Variable(Box<Node<Expression>>),
    Range(RangeListExpression),
}


#[derive(Debug, PartialEq, Clone)]
pub struct RangeListExpression {
    pub expression_start: Box<Node<Expression>>,
    pub expression_end: Box<Node<Expression>>,
}


#[derive(Debug, PartialEq, Clone)]
pub enum LocalListExpressionNode {
    Variable(Box<LocalExpressionNode>),
    Range(LocalRangeListExpressionNode),
}

#[derive(Debug, PartialEq, Clone)]
pub struct LocalRangeListExpressionNode {
    pub expression_start: Box<LocalExpressionNode>,
    pub expression_end: Box<LocalExpressionNode>,
}


impl NodeBuilder for RangeListExpression {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let expression_start = Box::new(Node::build(pairs.next().unwrap())?);
        let expression_end = Box::new(Node::build(pairs.next().unwrap())?);

        Ok(Self {
            expression_start,
            expression_end,
        })
    }
}/*
impl InstructionBuilder for ListExpression {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        match self {
            Self::Variable(v) => {
                v.compile(state)
            },
            Self::Range(r) => {

                let start = r.expression_start.compile(state);
                let end 
                return Ok(InstructionBuilderOk::from_instructions(vec![
                    Instruction {
                        pos: r.expression_start.pos,
                        control: crate::vm::instruction::InstructionType::Push(
                            LocalListExpressionNode::from_range_list
                        )
                    }
                ]))
            }
        }
    }
}
 */
impl RangeListExpression {

    pub fn add_dependencies(&self, dependencies: &mut WaitDependency) {
        self.expression_start.value.add_dependencies(dependencies);
        self.expression_end.value.add_dependencies(dependencies);
    }
    pub fn get_vars(&self, vars: &mut HashSet<String>) {
        self.expression_start.value.get_vars(vars);
        self.expression_end.value.get_vars(vars);
    }
}

impl LocalRangeListExpressionNode { 
    pub fn from_range(range: &RangeListExpression, program_stack: &Vec<Variable>) -> AlthreadResult<Self> {
        Ok(LocalRangeListExpressionNode {
            expression_start: Box::new(LocalExpressionNode::from_expression(&range.expression_start.value, program_stack)?),
            expression_end: Box::new(LocalExpressionNode::from_expression(&range.expression_end.value, program_stack)?),
        })
    }
}

impl LocalListExpressionNode {

    pub fn from_list(list_expr: &ListExpression, program_stack: &Vec<Variable>) -> AlthreadResult<Self> {
        match list_expr {
            ListExpression::Variable(node) => {
                Ok(Self::Variable(Box::new(LocalExpressionNode::from_expression(&node.value, program_stack)?)))
            },
            ListExpression::Range(node) => {
                Ok(Self::Range(LocalRangeListExpressionNode {
                    expression_start: Box::new(LocalExpressionNode::from_expression(&node.expression_start.value, program_stack)?),
                    expression_end: Box::new(LocalExpressionNode::from_expression(&node.expression_end.value, program_stack)?),
                }))
            },
        }
    }

}

impl LocalRangeListExpressionNode {
    pub fn datatype(&self, state: &CompilerState) -> Result<DataType, String> {
        let start = self.expression_start.datatype(state)?;
        let end = self.expression_end.datatype(state)?;
        if start.is_integer() && end.is_integer() {
            Ok(DataType::List(Box::new(DataType::Integer)))
        } else {
            Err(format!("Range expression must be of type integer, found {} and {}", start, end))
        }
        
    }
    pub fn eval(&self, mem: &Memory) -> Result<Literal, String> {
        let start = self.expression_start.eval(mem)?;
        let end = self.expression_end.eval(mem)?;
        Ok(Literal::List(DataType::Integer, (start.to_integer()?..end.to_integer()?).map(|v| Literal::Int(v)).collect()))
    }
}


impl AstDisplay for ListExpression {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        match self {
            Self::Variable(node) => node.ast_fmt(f, prefix),
            Self::Range(node) => node.ast_fmt(f, prefix),
        }
    }
}

impl AstDisplay for RangeListExpression {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        self.expression_start.ast_fmt(f, prefix)?;
        write!(f, "..")?;
        self.expression_end.ast_fmt(f, prefix)
    }
}

impl fmt::Display for LocalRangeListExpressionNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}..{}", self.expression_start, self.expression_end)
    }
}
impl fmt::Display for LocalListExpressionNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Variable(node) => write!(f, "{}", node),
            Self::Range(node) => write!(f, "{}", node),
        }
    }
}
