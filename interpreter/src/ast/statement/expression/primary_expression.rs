use std::{collections::HashSet, fmt::{self, Debug}};

use pest::iterators::Pair;
use rand::seq::index;

use crate::{
    ast::{
        display::AstDisplay,
        node::Node,
        token::{identifier::Identifier, literal::Literal},
    }, compiler::Variable, error::{AlthreadError, AlthreadResult, ErrorType}, no_rule, parser::Rule
};
use super::{Expression, LocalExpressionNode};

#[derive(Debug)]
pub enum PrimaryExpression {
    Literal(Node<Literal>),
    Identifier(Node<Identifier>),
    Expression(Box<Node<Expression>>),
}

impl PrimaryExpression {
    pub fn build(pair: Pair<Rule>) -> AlthreadResult<Node<Self>> {
        Ok(Node {
            line: pair.line_col().0,
            column: pair.line_col().1,
            value: match pair.as_rule() {
                Rule::literal => Self::Literal(Node::build(pair)?),
                Rule::identifier => Self::Identifier(Node::build(pair)?),
                Rule::expression => Self::Expression(Box::new(Node::build(pair)?)),
                _ => return Err(no_rule!(pair)),
            },
        })
    }
}

impl PrimaryExpression {
    pub fn get_vars(&self, vars: &mut HashSet<String>) {
        match self {
            Self::Literal(_) => (),
            Self::Identifier(node) => { vars.insert(node.value.value.clone()); },
            Self::Expression(node) => node.value.get_vars(vars),
        }
    }
}

#[derive(Debug)]
pub struct LocalLiteralNode {
    pub value: Literal,
}
#[derive(Debug)]
pub struct LocalVarNode {
    pub index: usize,
}

#[derive(Debug)]
pub enum LocalPrimaryExpressionNode {
    Literal(LocalLiteralNode),
    Var(LocalVarNode),
    Expression(Box<LocalExpressionNode>),
}


impl LocalPrimaryExpressionNode {
    pub fn from_primary(primary: &PrimaryExpression, program_stack: &Vec<Variable>) -> AlthreadResult<Self> {
        Ok(match primary {
            PrimaryExpression::Literal(node) => 
                LocalPrimaryExpressionNode::Literal(LocalLiteralNode::from_literal(node)?),
            PrimaryExpression::Identifier(node) => 
                LocalPrimaryExpressionNode::Var(LocalVarNode::from_identifier(node, program_stack)?),
            PrimaryExpression::Expression(node) => {
                let e = LocalExpressionNode::from_expression(&node.as_ref().value, program_stack)?;
                LocalPrimaryExpressionNode::Expression(Box::new(e))
            },
        })
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
    pub fn from_identifier(ident: &Node<Identifier>, program_stack: &Vec<Variable>) -> AlthreadResult<Self> {
        let index = program_stack.iter().rev().position(|var| var.name == ident.value.value).ok_or(AlthreadError::new(
            ErrorType::VariableError,
            ident.line,
            ident.column,
            format!("Variable '{}' not found", ident.value.value)
        ))?;
        Ok(LocalVarNode {
            index
        })
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
