use std::{
    collections::HashSet,
    fmt::{self, Debug},
};

use pest::iterators::Pair;

use super::{Expression, LocalExpressionNode};
use crate::{
    ast::{
        display::AstDisplay,
        node::Node,
        statement::waiting_case::WaitDependency,
        token::{datatype::DataType, identifier::Identifier, literal::Literal},
    },
    compiler::{CompilerState, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType, Pos},
    no_rule,
    parser::Rule,
};

#[derive(Debug, Clone)]
pub enum PrimaryExpression {
    Literal(Node<Literal>),
    Identifier(Node<Identifier>),
    Expression(Box<Node<Expression>>),
}

impl PrimaryExpression {
    pub fn build(pair: Pair<Rule>) -> AlthreadResult<Node<Self>> {
        Ok(Node {
            pos: Pos {
                line: pair.line_col().0,
                col: pair.line_col().1,
                start: pair.as_span().start(),
                end: pair.as_span().end(),
            },
            value: match pair.as_rule() {
                Rule::literal => Self::Literal(Node::build(pair)?),
                Rule::identifier => Self::Identifier(Node::build(pair)?),
                Rule::expression => Self::Expression(Box::new(Node::build(pair)?)),
                _ => return Err(no_rule!(pair, "PrimaryExpression")),
            },
        })
    }
}

impl PrimaryExpression {
    pub fn add_dependencies(&self, dependencies: &mut WaitDependency) {
        match self {
            Self::Literal(_) => (),
            Self::Identifier(node) => {
                dependencies.variables.insert(node.value.value.clone());
            }
            Self::Expression(node) => node.value.add_dependencies(dependencies),
        }
    }
    pub fn get_vars(&self, vars: &mut HashSet<String>) {
        match self {
            Self::Literal(_) => (),
            Self::Identifier(node) => {
                vars.insert(node.value.value.clone());
            }
            Self::Expression(node) => node.value.get_vars(vars),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalLiteralNode {
    pub value: Literal,
}
#[derive(Debug, Clone)]
pub struct LocalVarNode {
    pub index: usize,
}

#[derive(Debug, Clone)]
pub enum LocalPrimaryExpressionNode {
    Literal(LocalLiteralNode),
    Var(LocalVarNode),
    Expression(Box<LocalExpressionNode>),
}

impl fmt::Display for LocalPrimaryExpressionNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Literal(node) => write!(f, "{}", node.value),
            Self::Var(node) => write!(f, "[{}]", node.index),
            Self::Expression(node) => write!(f, "({})", node),
        }
    }
}

impl LocalPrimaryExpressionNode {
    pub fn from_primary(
        primary: &PrimaryExpression,
        program_stack: &Vec<Variable>,
    ) -> AlthreadResult<Self> {
        Ok(match primary {
            PrimaryExpression::Literal(node) => {
                LocalPrimaryExpressionNode::Literal(LocalLiteralNode::from_literal(node)?)
            }
            PrimaryExpression::Identifier(node) => {
                LocalPrimaryExpressionNode::Var(LocalVarNode::from_identifier(node, program_stack)?)
            }
            PrimaryExpression::Expression(node) => {
                let e = LocalExpressionNode::from_expression(&node.as_ref().value, program_stack)?;
                LocalPrimaryExpressionNode::Expression(Box::new(e))
            }
        })
    }

    pub fn datatype(&self, state: &CompilerState) -> Result<DataType, String> {
        match self {
            Self::Expression(e) => e.datatype(state),
            Self::Literal(l) => Ok(l.value.get_datatype()),
            Self::Var(v) => {
                let mem_len = state.program_stack.len();
                //println!("   var {}:{}", v.index, state.program_stack.get(v.index).expect("variable index does not exists").datatype);
                Ok(state
                    .program_stack
                    .get(mem_len - 1 - v.index)
                    .expect("variable index does not exists")
                    .datatype
                    .clone())
            }
        }
    }
}

impl LocalLiteralNode {
    pub fn from_literal(literal: &Node<Literal>) -> AlthreadResult<Self> {
        Ok(LocalLiteralNode {
            value: literal.value.clone(),
        })
    }
}
impl LocalVarNode {
    pub fn from_identifier(
        ident: &Node<Identifier>,
        program_stack: &Vec<Variable>,
    ) -> AlthreadResult<Self> {
        let index = program_stack
            .iter()
            .rev()
            .position(|var| var.name == ident.value.value)
            .ok_or(AlthreadError::new(
                ErrorType::VariableError,
                Some(ident.pos),
                format!("Variable '{}' not found", ident.value.value),
            ))?;
        Ok(LocalVarNode { index })
    }
}

impl AstDisplay for PrimaryExpression {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &crate::ast::display::Prefix) -> fmt::Result {
        match self {
            Self::Literal(node) => node.ast_fmt(f, prefix),
            PrimaryExpression::Identifier(value) => writeln!(f, "{prefix}ident: {value}"),
            PrimaryExpression::Expression(node) => node.ast_fmt(f, prefix),
        }
    }
}
