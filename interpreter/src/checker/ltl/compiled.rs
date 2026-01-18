use std::fmt;

use crate::{
    ast::statement::expression::LocalExpressionNode,
    ast::token::datatype::DataType,
};

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
    }
}

impl fmt::Display for CompiledLtlExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompiledLtlExpression::Always(e) => write!(f, "[] ({})", e),
            CompiledLtlExpression::Eventually(e) => write!(f, "<> ({})", e),
            CompiledLtlExpression::Next(e) => write!(f, "X ({})", e),
            CompiledLtlExpression::Not(e) => write!(f, "! ({})", e),
            CompiledLtlExpression::Until(l, r) => write!(f, "({}) U ({})", l, r),
            CompiledLtlExpression::And(l, r) => write!(f, "({}) && ({})", l, r),
            CompiledLtlExpression::Or(l, r) => write!(f, "({}) || ({})", l, r),
            CompiledLtlExpression::Implies(l, r) => write!(f, "({}) -> ({})", l, r),
            CompiledLtlExpression::Predicate { read_variables, .. } => {
                write!(f, "Pred[deps={:?}]", read_variables)
            }
            CompiledLtlExpression::ForLoop { loop_var_name, list_expression, body, .. } => {
                write!(f, "For({} in {}): {}", loop_var_name, list_expression, body)
            }
        }
    }
}
