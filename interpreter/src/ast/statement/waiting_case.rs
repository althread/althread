use std::collections::HashSet;
use std::fmt;

use pest::iterators::Pairs;
use serde::{Deserialize, Serialize};

use crate::compiler::CompilerState;
use crate::error::AlthreadResult;
use crate::parser::Rule;
use crate::{ast::node::InstructionBuilder, compiler::InstructionBuilderOk};

use super::super::{
    display::{AstDisplay, Prefix},
    node::{Node, NodeBuilder},
    statement::Statement,
};
use super::expression::Expression;
use super::receive::ReceiveStatement;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaitDependency {
    pub channels_state: HashSet<String>,
    pub channels_connection: HashSet<String>,
    pub variables: HashSet<String>,
}
impl WaitDependency {
    pub fn new() -> Self {
        Self {
            channels_state: HashSet::new(),
            channels_connection: HashSet::new(),
            variables: HashSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum WaitingBlockCaseRule {
    Expression(Node<Expression>),
    Receive(Node<ReceiveStatement>),
}

#[derive(Debug, Clone)]
pub struct WaitingBlockCase {
    pub rule: WaitingBlockCaseRule,
    pub statement: Option<Node<Statement>>,
}

impl NodeBuilder for WaitingBlockCase {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();

        let rule = match pair.as_rule() {
            Rule::expression => WaitingBlockCaseRule::Expression(Node::build(pair)?),
            Rule::receive_expression => WaitingBlockCaseRule::Receive(Node::build(pair)?),
            _ => panic!("Invalid rule while parsing waiting block case"),
        };

        let pair = pairs.next();

        let statement = match pair {
            Some(p) => Some(Node::build(p)?),
            None => None,
        };

        Ok(Self { rule, statement })
    }
}

impl AstDisplay for WaitingBlockCase {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}wait case")?;

        {
            let prefix = prefix.add_branch();
            self.rule.ast_fmt(f, &prefix)?;
        }
        if let Some(statement) = &self.statement {
            let prefix = prefix.add_leaf();
            writeln!(f, "{prefix}statement")?;
            statement.ast_fmt(f, &prefix.add_leaf())?;
        }

        Ok(())
    }
}

impl WaitingBlockCaseRule {
    pub fn add_dependencies(&self, dependencies: &mut WaitDependency) {
        match self {
            WaitingBlockCaseRule::Expression(expr) => expr.value.add_dependencies(dependencies),
            WaitingBlockCaseRule::Receive(receive) => receive.value.add_dependencies(dependencies),
        }
    }
}
impl InstructionBuilder for WaitingBlockCaseRule {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        match self {
            WaitingBlockCaseRule::Expression(expr) => expr.compile(state),
            WaitingBlockCaseRule::Receive(receive) => receive.compile(state),
        }
    }
}
impl AstDisplay for WaitingBlockCaseRule {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        match self {
            WaitingBlockCaseRule::Expression(expr) => expr.ast_fmt(f, prefix),
            WaitingBlockCaseRule::Receive(receive) => receive.ast_fmt(f, prefix),
        }
    }
}
