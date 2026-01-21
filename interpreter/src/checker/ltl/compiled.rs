use std::fmt;

use crate::{ast::statement::expression::LocalExpressionNode, ast::token::datatype::DataType};

/// Represents a compiled LTL formula ready for verification
#[derive(Debug, Clone, PartialEq)]
pub enum CompiledLtlExpression {
    Always(Box<CompiledLtlExpression>),
    Eventually(Box<CompiledLtlExpression>),
    Next(Box<CompiledLtlExpression>),
    Not(Box<CompiledLtlExpression>),
    Until(Box<CompiledLtlExpression>, Box<CompiledLtlExpression>),
    And(Box<CompiledLtlExpression>, Box<CompiledLtlExpression>),
    Or(Box<CompiledLtlExpression>, Box<CompiledLtlExpression>),
    Implies(Box<CompiledLtlExpression>, Box<CompiledLtlExpression>),
    Release(Box<CompiledLtlExpression>, Box<CompiledLtlExpression>),
    Boolean(bool),

    /// A leaf predicate (expression returning boolean)
    Predicate {
        expression: LocalExpressionNode,
        /// The list of global variables that must be pushed to the stack
        /// before evaluating this expression.
        /// These correspond to indices 0..N in the evaluation stack.
        read_variables: Vec<String>,
        /// If this predicate uses variables from enclosing loops,
        /// we map them to their expected index in the stack.
        /// (Not fully implemented yet for loops)
        scope_mapping: Option<Vec<usize>>,
    },

    /// A for loop over a list expression.
    /// At runtime, this expands to a conjunction (And) or disjunction
    /// depending on semantics (usually conjunction for 'forall', disjunction for 'exists').
    /// But here syntax is just 'for', often implying 'forall'.
    ForLoop {
        list_expression: LocalExpressionNode,
        list_read_variables: Vec<String>,
        loop_var_name: String,
        body: Box<CompiledLtlExpression>,
    },

    Exists {
        list_expression: LocalExpressionNode,
        list_read_variables: Vec<String>,
        loop_var_name: String,
        body: Box<CompiledLtlExpression>,
    },
}

impl fmt::Display for CompiledLtlExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompiledLtlExpression::Always(e) => write!(f, "[] ({})", e),
            CompiledLtlExpression::Eventually(e) => write!(f, "<> ({})", e),
            CompiledLtlExpression::Next(e) => write!(f, "X ({})", e),
            CompiledLtlExpression::Not(e) => write!(f, "! ({})", e),
            CompiledLtlExpression::Until(l, r) => write!(f, "({}) U ({})", l, r),
            CompiledLtlExpression::Release(l, r) => write!(f, "({}) R ({})", l, r),
            CompiledLtlExpression::And(l, r) => write!(f, "({}) && ({})", l, r),
            CompiledLtlExpression::Or(l, r) => write!(f, "({}) || ({})", l, r),
            CompiledLtlExpression::Implies(l, r) => write!(f, "({}) -> ({})", l, r),
            CompiledLtlExpression::Boolean(b) => write!(f, "{}", b),
            CompiledLtlExpression::Predicate { read_variables, .. } => {
                write!(f, "Pred[deps={:?}]", read_variables)
            }
            CompiledLtlExpression::ForLoop {
                loop_var_name,
                list_expression,
                body,
                ..
            } => {
                write!(f, "For({} in {}): {}", loop_var_name, list_expression, body)
            }
            CompiledLtlExpression::Exists {
                loop_var_name,
                list_expression,
                body,
                ..
            } => {
                write!(
                    f,
                    "Exists({} in {}): {}",
                    loop_var_name, list_expression, body
                )
            }
        }
    }
}

impl CompiledLtlExpression {
    pub fn negate(self) -> Self {
        CompiledLtlExpression::Not(Box::new(self)).simplify()
    }

    pub fn simplify(self) -> Self {
        match self {
            CompiledLtlExpression::Not(inner) => match *inner {
                CompiledLtlExpression::Not(e) => e.simplify(),
                CompiledLtlExpression::Boolean(b) => CompiledLtlExpression::Boolean(!b),
                CompiledLtlExpression::And(a, b) => CompiledLtlExpression::Or(
                    Box::new(CompiledLtlExpression::Not(a).simplify()),
                    Box::new(CompiledLtlExpression::Not(b).simplify()),
                ),
                CompiledLtlExpression::Or(a, b) => CompiledLtlExpression::And(
                    Box::new(CompiledLtlExpression::Not(a).simplify()),
                    Box::new(CompiledLtlExpression::Not(b).simplify()),
                ),
                CompiledLtlExpression::Next(a) => {
                    CompiledLtlExpression::Next(Box::new(CompiledLtlExpression::Not(a).simplify()))
                }
                CompiledLtlExpression::Always(a) => CompiledLtlExpression::Eventually(Box::new(
                    CompiledLtlExpression::Not(a).simplify(),
                ))
                .simplify(),
                CompiledLtlExpression::Eventually(a) => CompiledLtlExpression::Always(Box::new(
                    CompiledLtlExpression::Not(a).simplify(),
                )),
                CompiledLtlExpression::Until(a, b) => CompiledLtlExpression::Release(
                    Box::new(CompiledLtlExpression::Not(a).simplify()),
                    Box::new(CompiledLtlExpression::Not(b).simplify()),
                ),
                CompiledLtlExpression::Release(a, b) => CompiledLtlExpression::Until(
                    Box::new(CompiledLtlExpression::Not(a).simplify()),
                    Box::new(CompiledLtlExpression::Not(b).simplify()),
                ),
                CompiledLtlExpression::Implies(a, b) => CompiledLtlExpression::And(
                    Box::new(a.simplify()),
                    Box::new(CompiledLtlExpression::Not(b).simplify()),
                ),
                e => CompiledLtlExpression::Not(Box::new(e.simplify())),
            },
            CompiledLtlExpression::Implies(a, b) => CompiledLtlExpression::Or(
                Box::new(CompiledLtlExpression::Not(a).simplify()),
                Box::new(b.simplify()),
            ),
            CompiledLtlExpression::Always(a) => CompiledLtlExpression::Release(
                Box::new(CompiledLtlExpression::Boolean(false)),
                Box::new(a.simplify()),
            ),
            CompiledLtlExpression::Eventually(a) => CompiledLtlExpression::Until(
                Box::new(CompiledLtlExpression::Boolean(true)),
                Box::new(a.simplify()),
            ),

            CompiledLtlExpression::And(a, b) => {
                CompiledLtlExpression::And(Box::new(a.simplify()), Box::new(b.simplify()))
            }
            CompiledLtlExpression::Or(a, b) => {
                CompiledLtlExpression::Or(Box::new(a.simplify()), Box::new(b.simplify()))
            }
            CompiledLtlExpression::Next(a) => CompiledLtlExpression::Next(Box::new(a.simplify())),
            CompiledLtlExpression::Until(a, b) => {
                CompiledLtlExpression::Until(Box::new(a.simplify()), Box::new(b.simplify()))
            }
            CompiledLtlExpression::Release(a, b) => {
                CompiledLtlExpression::Release(Box::new(a.simplify()), Box::new(b.simplify()))
            }
            CompiledLtlExpression::ForLoop {
                list_expression,
                list_read_variables,
                loop_var_name,
                body,
            } => CompiledLtlExpression::ForLoop {
                list_expression,
                list_read_variables,
                loop_var_name,
                body: Box::new(body.simplify()),
            },
            CompiledLtlExpression::Exists {
                list_expression,
                list_read_variables,
                loop_var_name,
                body,
            } => CompiledLtlExpression::Exists {
                list_expression,
                list_read_variables,
                loop_var_name,
                body: Box::new(body.simplify()),
            },
            e => e,
        }
    }
}
