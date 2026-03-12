use std::fmt;

use pest::iterators::Pairs;
use serde::{Deserialize, Serialize};

use crate::{ast::node::NodeBuilder, error::AlthreadResult, no_rule, parser::Rule};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    And,
    Or,
    ShiftLeft,
    ShiftRight,
    BitAnd,
    BitOr,
}

impl NodeBuilder for BinaryOperator {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        match pair.as_rule() {
            Rule::ADD_OP => Ok(Self::Add),
            Rule::SUB_OP => Ok(Self::Subtract),
            Rule::MUL_OP => Ok(Self::Multiply),
            Rule::DIV_OP => Ok(Self::Divide),
            Rule::MOD_OP => Ok(Self::Modulo),
            Rule::EQ_OP => Ok(Self::Equals),
            Rule::NE_OP => Ok(Self::NotEquals),
            Rule::LT_OP => Ok(Self::LessThan),
            Rule::LE_OP => Ok(Self::LessThanOrEqual),
            Rule::GT_OP => Ok(Self::GreaterThan),
            Rule::GE_OP => Ok(Self::GreaterThanOrEqual),
            Rule::AND_OP => Ok(Self::And),
            Rule::OR_OP => Ok(Self::Or),
            Rule::SHL_OP => Ok(Self::ShiftLeft),
            Rule::SHR_OP => Ok(Self::ShiftRight),
            Rule::BITWISE_AND_OP => Ok(Self::BitAnd),
            Rule::BITWISE_OR_OP => Ok(Self::BitOr),
            _ => Err(no_rule!(pair, "BinaryOperator", filepath)),
        }
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            BinaryOperator::Add => "+",
            BinaryOperator::Subtract => "-",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::Modulo => "%",
            BinaryOperator::Equals => "==",
            BinaryOperator::NotEquals => "!=",
            BinaryOperator::LessThan => "<",
            BinaryOperator::LessThanOrEqual => "<=",
            BinaryOperator::GreaterThan => ">",
            BinaryOperator::GreaterThanOrEqual => ">=",
            BinaryOperator::And => "&&",
            BinaryOperator::Or => "||",
            BinaryOperator::ShiftLeft => "<<",
            BinaryOperator::ShiftRight => ">>",
            BinaryOperator::BitAnd => "&",
            BinaryOperator::BitOr => "|",
        };

        write!(f, "{}", op)
    }
}
