use pest::iterators::Pair;

use crate::{
    env::Environment,
    error::{AlthreadError, ErrorType},
    nodes::{
        assign::{Assign, AssignBinOp},
        datatype::DataType,
        expr::{primary::PrimaryExpr, Expr, ExprKind},
    },
    parser::Rule,
};

impl Assign {
    pub fn build(pair: Pair<Rule>, env: &Environment) -> Result<Self, AlthreadError> {
        let (line, column) = pair.line_col();
        let mut assign = Self::new(line, column);

        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::IDENTIFIER => assign.identifier = pair.as_str().to_string(),
                // TODO: implement assign op
                Rule::assign_op => {
                    assign.op = match pair.as_str() {
                        "=" => AssignBinOp::Assign,
                        _ => unimplemented!(),
                    }
                }
                Rule::expr => assign.value = Expr::build(pair, env)?,
                // TODO: implement assign unary op
                Rule::assign_unary_op => {
                    unimplemented!()
                }
                _ => unreachable!(),
            }
        }

        let value_type = DataType::from_expr(&assign.value.kind, env).map_err(|e| {
            AlthreadError::error(
                ErrorType::TypeError,
                assign.value.line,
                assign.value.column,
                e,
            )
        })?;

        let symbol = env.get_symbol(&assign.identifier).map_err(|e| {
            AlthreadError::error(
                ErrorType::VariableError,
                assign.value.line,
                assign.value.column,
                e,
            )
        })?;

        if !symbol.mutable {
            return Err(AlthreadError::error(
                ErrorType::VariableError,
                assign.value.line,
                assign.value.column,
                "Cannot change immutable variable value".to_string(),
            ));
        }

        if symbol.datatype != value_type {
            return Err(AlthreadError::error(
                ErrorType::TypeError,
                assign.line,
                assign.column,
                format!(
                    "Cannot change {} type from {} to {}",
                    assign.identifier, symbol.datatype, value_type
                ),
            ));
        }

        Ok(assign)
    }
}
