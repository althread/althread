use std::fmt;

use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::PrattParser;

use crate::{
    ast::{
        node::{Node, NodeBuilder},
        statement::expression::{Expression, list_expression::RangeListExpression},
    },
    error::AlthreadResult,
    no_rule,
    parser::Rule,
};

lazy_static::lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};

        PrattParser::new()
            .op(Op::infix(Rule::OR_OP, Left))
            .op(Op::infix(Rule::AND_OP, Left))
            .op(Op::infix(Rule::UNTIL_KW, Left))
    };
}

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

impl NodeBuilder for CheckBlock {
    fn build(pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let mut formulas = Vec::new();

        for pair in pairs {
            match pair.as_rule() {
                Rule::ltl_statement => {
                    // formulas.push(build_ltl_statement(pair, filepath)?);
                    let mut inner = pair.into_inner();
                    let expr_pair = inner.next().unwrap();
                    let formula = build_ltl_expression(expr_pair, filepath)?;
                    formulas.push(formula);
                }
                _ => {}
            }
        }

        Ok(Self { formulas })
    }
}
/*
fn build_ltl_statement(pair: Pair<Rule>, filepath: &str) -> AlthreadResult<LtlExpression> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::ltl_for_loop => build_ltl_for_loop(inner, filepath),
        Rule::ltl_expression => build_ltl_expression(inner, filepath),
        _ => unreachable!("Invalid ltl statement"),
    }
}
*/
fn build_ltl_for_loop(pair: Pair<Rule>, filepath: &str) -> AlthreadResult<LtlExpression> {
    let mut inner = pair.into_inner();
    let ident_pair = inner.next().unwrap();
    let var_name = ident_pair.as_str().to_string();

    let list_pair = inner.next().unwrap();
    let list_node = match list_pair.as_rule() {
        Rule::range_expression => {
            let range = Node::<RangeListExpression>::build(list_pair, filepath)?;
            Node {
                pos: range.pos.clone(),
                value: Expression::Range(range),
            }
        }
        Rule::expression => Node::<Expression>::build(list_pair, filepath)?,
        _ => unreachable!("Invalid list expression"),
    };

    /*
    let mut body = Vec::new();
    for stmt_pair in inner {
        if stmt_pair.as_rule() == Rule::ltl_statement {
            body.push(build_ltl_statement(stmt_pair, filepath)?);
        }
    }
    */
    let expr_pair = inner.next().unwrap();
    let body = build_ltl_expression(expr_pair, filepath)?;

    Ok(LtlExpression::ForLoop {
        var_name,
        list: list_node,
        body: Box::new(body),
    })
}

fn build_ltl_expression(pair: Pair<Rule>, filepath: &str) -> AlthreadResult<LtlExpression> {
    match pair.as_rule() {
        Rule::ltl_expression => {
            let pairs = pair.into_inner();
            PRATT_PARSER
                .map_primary(|primary| build_ltl_term(primary, filepath))
                .map_infix(|lhs, op, rhs| {
                    let lhs = lhs?;
                    let rhs = rhs?;
                    match op.as_rule() {
                        Rule::OR_OP => Ok(LtlExpression::Or(Box::new(lhs), Box::new(rhs))),
                        Rule::AND_OP => Ok(LtlExpression::And(Box::new(lhs), Box::new(rhs))),
                        Rule::UNTIL_KW => Ok(LtlExpression::Until(Box::new(lhs), Box::new(rhs))),
                        _ => unreachable!("Invalid binary operator"),
                    }
                })
                .parse(pairs)
        }
        _ => Err(no_rule!(pair, "LtlExpression", filepath)),
    }
}

fn build_ltl_term(pair: Pair<Rule>, filepath: &str) -> AlthreadResult<LtlExpression> {
    match pair.as_rule() {
        Rule::ltl_term => build_ltl_term(pair.into_inner().next().unwrap(), filepath),
        Rule::ltl_for_loop => build_ltl_for_loop(pair, filepath),
        Rule::ltl_unary_expression => {
            let mut inner = pair.into_inner();
            let op = inner.next().unwrap();
            let expr = inner.next().unwrap();
            let built_expr = build_ltl_term(expr, filepath)?;
            
            match op.as_rule() {
                Rule::ALWAYS_KW => Ok(LtlExpression::Always(Box::new(built_expr))),
                Rule::EVENTUALLY_KW => Ok(LtlExpression::Eventually(Box::new(built_expr))),
                Rule::NOT_OP => Ok(LtlExpression::Not(Box::new(built_expr))),
                _ => unreachable!("Invalid unary operator"),
            }
        },
        Rule::ltl_if_expression => {
            let mut inner = pair.into_inner();
            let lhs = build_ltl_expression(inner.next().unwrap(), filepath)?;
            let rhs = build_ltl_expression(inner.next().unwrap(), filepath)?;
            Ok(LtlExpression::Implies(Box::new(lhs), Box::new(rhs)))
        },
        Rule::ltl_predicate => {
            let expr_pair = pair.into_inner().next().unwrap(); // expression
            let expr_node = Node::<Expression>::build(expr_pair, filepath)?;
            Ok(LtlExpression::Predicate(expr_node))
        },
        Rule::ltl_expression => {
            // Parenthesized expression
             build_ltl_expression(pair, filepath)
        }
        _ => Err(no_rule!(pair, "LtlTerm", filepath)),
    }
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
            LtlExpression::ForLoop { var_name, list, body } => {
                write!(f, "for {} in {} {{ {}; }}", var_name, list.value, body)
            }
        }
    }
}
