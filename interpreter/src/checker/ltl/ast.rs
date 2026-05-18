use std::fmt;

use crate::ast::{node::Node, statement::expression::Expression};

/// Represents an LTL formula
#[derive(Debug, Clone, PartialEq)]
pub enum LtlExpression {
    Always(Box<LtlExpression>),
    Eventually(Box<LtlExpression>),
    Next(Box<LtlExpression>),
    Not(Box<LtlExpression>),
    Until(Box<LtlExpression>, Box<LtlExpression>),
    And(Box<LtlExpression>, Box<LtlExpression>),
    Or(Box<LtlExpression>, Box<LtlExpression>),
    Implies(Box<LtlExpression>, Box<LtlExpression>),
    Predicate(Node<Expression>),
    ForLoop {
        var_name: String,
        list: Node<Expression>,
        body: Box<LtlExpression>,
    },
}

/// A list of LTL formulas defined in a check block
#[derive(Debug, Clone)]
pub struct CheckBlock {
    pub formulas: Vec<LtlExpression>,
}

impl fmt::Display for LtlExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LtlExpression::Always(e) => write!(f, "[] ({})", e),
            LtlExpression::Eventually(e) => write!(f, "<> ({})", e),
            LtlExpression::Next(e) => write!(f, "X ({})", e),
            LtlExpression::Not(e) => write!(f, "! ({})", e),
            LtlExpression::Until(l, r) => write!(f, "({}) U ({})", l, r),
            LtlExpression::And(l, r) => write!(f, "({}) && ({})", l, r),
            LtlExpression::Or(l, r) => write!(f, "({}) || ({})", l, r),
            LtlExpression::Implies(l, r) => write!(f, "({}) -> ({})", l, r),
            LtlExpression::Predicate(e) => write!(f, "{}", e),
            LtlExpression::ForLoop {
                var_name,
                list,
                body,
            } => {
                write!(f, "for {} in {} {{ {}; }}", var_name, list.value, body)
            }
        }
    }
}
