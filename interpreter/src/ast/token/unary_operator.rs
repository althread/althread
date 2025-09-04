use std::fmt;

use pest::iterators::Pairs;

use crate::{ast::node::NodeBuilder, error::AlthreadResult, no_rule, parser::Rule};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Positive,
    Negative,
    Not,
}

impl NodeBuilder for UnaryOperator {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        match pair.as_rule() {
            Rule::POS_OP => Ok(Self::Positive),
            Rule::NEG_OP => Ok(Self::Negative),
            Rule::NOT_OP => Ok(Self::Not),
            _ => Err(no_rule!(pair, "UnaryOperator", filepath)),
        }
    }
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            UnaryOperator::Positive => "+",
            UnaryOperator::Negative => "-",
            UnaryOperator::Not => "!",
        };

        write!(f, "{}", op)
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LtlUnaryOperator {
    Not,
    Always,
    Eventually
}

impl NodeBuilder for LtlUnaryOperator {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        match pair.as_rule() {
            Rule::NOT_OP => Ok(Self::Not),
            Rule::ltl_always_operator => Ok(Self::Always),
            Rule::ltl_eventually_operator => Ok(Self::Eventually),
            _ => Err(no_rule!(pair, "LTL UnaryOperator", filepath)),
        }
    }
}

impl fmt::Display for LtlUnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            LtlUnaryOperator::Not => "!",
            LtlUnaryOperator::Always => "always",
            LtlUnaryOperator::Eventually => "eventually",
        };

        write!(f, "{}", op)
    }
}
