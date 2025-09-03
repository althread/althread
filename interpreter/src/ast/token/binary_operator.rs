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
        };

        write!(f, "{}", op)
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum LtlBinaryOperator {
    Until,
    WeakUntil,
    Release,
    And,
    Or,
    Implies,
    Equivalent
}

impl NodeBuilder for LtlBinaryOperator {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        match pair.as_rule() {
            Rule::LTL_UNTIL_OP => Ok(Self::Until),
            Rule::LTL_WEAK_UNTIL_OP => Ok(Self::WeakUntil),
            Rule::LTL_RELEASE_OP => Ok(Self::Release),
            Rule::LTL_IMPLIES_OP => Ok(Self::Implies),
            Rule::LTL_EQUIVALENT_OP => Ok(Self::Equivalent),
            Rule::AND_OP => Ok(Self::And),
            Rule::OR_OP => Ok(Self::Or),
            _ => Err(no_rule!(pair, "LtlBinaryOperator", filepath)),
        }
    }
}

impl fmt::Display for LtlBinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            LtlBinaryOperator::Until => "U",
            LtlBinaryOperator::WeakUntil => "W",
            LtlBinaryOperator::Release => "V",
            LtlBinaryOperator::Implies => "=>",
            LtlBinaryOperator::Equivalent => "<=>",
            LtlBinaryOperator::And => "&&",
            LtlBinaryOperator::Or => "||",
        };

        write!(f, "{}", op)
    }
}
