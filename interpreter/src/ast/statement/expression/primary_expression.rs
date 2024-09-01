use std::{collections::HashSet, fmt::{self, Debug}};

use pest::iterators::Pair;
use rand::seq::index;

use crate::{
    ast::{
        display::AstDisplay,
        node::{Node, NodeExecutor},
        token::{identifier::Identifier, literal::Literal},
    }, compiler::Variable, env::process_env::ProcessEnv, error::AlthreadResult, no_rule, parser::Rule
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

impl NodeExecutor for PrimaryExpression {
    fn eval(&self, env: &mut ProcessEnv) -> AlthreadResult<Option<Literal>> {
        match self {
            Self::Literal(node) => node.eval(env),
            Self::Identifier(node) => node.eval(env),
            Self::Expression(node) => node.eval(env),
        }
    }
}


impl LocalPrimaryExpressionNode {
    pub fn from_primary(primary: &PrimaryExpression, program_stack: &Vec<Variable>) -> Self {
        match primary {
            PrimaryExpression::Literal(node) => 
                LocalPrimaryExpressionNode::Literal(LocalLiteralNode::from_literal(node)),
            PrimaryExpression::Identifier(node) => 
                LocalPrimaryExpressionNode::Var(LocalVarNode::from_identifier(node, program_stack)),
            PrimaryExpression::Expression(node) => 
                LocalPrimaryExpressionNode::Expression(Box::new(LocalExpressionNode::from_expression(&node.as_ref().value, program_stack))),
        }
    }
}

impl LocalLiteralNode {
    pub fn from_literal(literal: &Node<Literal>) -> Self {
        LocalLiteralNode {
            value: literal.value.clone(),
        }
    }
}
impl LocalVarNode {
    pub fn from_identifier(ident: &Node<Identifier>, program_stack: &Vec<Variable>) -> Self {
        let index = program_stack.iter().rev().position(|var| var.name == ident.value.value).expect("Variable not found");
        LocalVarNode {
            index
        }
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
