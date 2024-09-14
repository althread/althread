use std::fmt;

use pest::iterators::Pairs;

use crate::{ast::node::NodeBuilder, error::AlthreadResult, no_rule, parser::Rule};

use super::literal::Literal;

#[derive(Debug, Clone)]
pub enum BinaryAssignmentOperator {
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
    OrAssign,
}

impl BinaryAssignmentOperator {
    pub fn apply(&self, left: &Literal, right: &Literal) -> Result<Literal, String> {
        match self {
            Self::Assign => Ok(right.clone()),
            Self::AddAssign => left.add(right),
            Self::SubtractAssign => left.subtract(right),
            Self::MultiplyAssign => left.multiply(right),
            Self::DivideAssign => left.divide(right),
            Self::ModuloAssign => left.modulo(right),
            Self::OrAssign => left.or(right),
        }
    }
}

impl NodeBuilder for BinaryAssignmentOperator {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        match pair.as_rule() {
            Rule::ASSIGN_OP => Ok(Self::Assign),
            Rule::ADD_ASSIGN_OP => Ok(Self::AddAssign),
            Rule::SUB_ASSIGN_OP => Ok(Self::SubtractAssign),
            Rule::MUL_ASSIGN_OP => Ok(Self::MultiplyAssign),
            Rule::DIV_ASSIGN_OP => Ok(Self::DivideAssign),
            Rule::MOD_ASSIGN_OP => Ok(Self::ModuloAssign),
            Rule::OR_ASSIGN_OP => Ok(Self::OrAssign),
            _ => Err(no_rule!(pair, "BinaryAssignmentOperator")),
        }
    }
}

impl fmt::Display for BinaryAssignmentOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Assign => write!(f, "="),
            Self::AddAssign => write!(f, "+="),
            Self::SubtractAssign => write!(f, "-="),
            Self::MultiplyAssign => write!(f, "*="),
            Self::DivideAssign => write!(f, "/="),
            Self::ModuloAssign => write!(f, "%="),
            Self::OrAssign => write!(f, "|="),
        }
    }
}
