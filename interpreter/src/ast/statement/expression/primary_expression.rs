use std::{
    collections::HashSet,
    fmt::{self, Debug},
};

use super::{Expression, LocalExpressionNode};
use crate::{
    ast::{
        display::AstDisplay,
        node::Node,
        statement::waiting_case::WaitDependency,
        token::{
            datatype::DataType, identifier::Identifier, literal::Literal,
            object_identifier::ObjectIdentifier,
        },
    },
    compiler::{CompilerState, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType},
};

#[derive(Debug, Clone, PartialEq)]
pub enum PrimaryExpression {
    Literal(Node<Literal>),
    Identifier(Node<ObjectIdentifier>),
    Expression(Box<Node<Expression>>),
    Reaches(
        Node<ObjectIdentifier>,
        Option<Box<Node<Expression>>>,
        Node<Identifier>,
    ),
    IfExpr {
        condition: Box<Node<Expression>>,
        then_expr: Box<Node<Expression>>,
        else_expr: Option<Box<Node<Expression>>>,
    },
    ForAllExpr {
        var: Node<Identifier>,
        list: Box<Node<Expression>>,
        body: Box<Node<Expression>>,
    },
    ExistsExpr {
        var: Node<Identifier>,
        list: Box<Node<Expression>>,
        body: Box<Node<Expression>>,
    },
}

impl PrimaryExpression {}

impl PrimaryExpression {
    pub fn add_dependencies(&self, dependencies: &mut WaitDependency) {
        match self {
            Self::Literal(_) => (),
            Self::Identifier(node) => {
                dependencies.variables.insert(
                    node.value
                        .parts
                        .iter()
                        .map(|p| p.value.value.as_str())
                        .collect::<Vec<_>>()
                        .join("."),
                );
            }
            Self::Expression(node) => node.value.add_dependencies(dependencies),
            Self::Reaches(proc_ident, index_expr, _) => {
                dependencies.variables.insert(
                    proc_ident
                        .value
                        .parts
                        .iter()
                        .map(|p| p.value.value.as_str())
                        .collect::<Vec<_>>()
                        .join("."),
                );
                if let Some(expr) = index_expr {
                    expr.value.add_dependencies(dependencies);
                }
            }
            Self::IfExpr {
                condition,
                then_expr,
                else_expr,
            } => {
                condition.value.add_dependencies(dependencies);
                then_expr.value.add_dependencies(dependencies);
                if let Some(else_expr) = else_expr {
                    else_expr.value.add_dependencies(dependencies);
                }
            }
            Self::ForAllExpr { var, list, body } => {
                list.value.add_dependencies(dependencies);
                body.value.add_dependencies(dependencies);
                dependencies.variables.remove(&var.value.value);
            }
            Self::ExistsExpr { var, list, body } => {
                list.value.add_dependencies(dependencies);
                body.value.add_dependencies(dependencies);
                dependencies.variables.remove(&var.value.value);
            }
        }
    }
    pub fn get_vars(&self, vars: &mut HashSet<String>) {
        match self {
            Self::Literal(_) => (),
            Self::Identifier(node) => {
                vars.insert(
                    node.value
                        .parts
                        .iter()
                        .map(|p| p.value.value.as_str())
                        .collect::<Vec<_>>()
                        .join("."),
                );
            }
            Self::Expression(node) => node.value.get_vars(vars),
            Self::Reaches(proc_ident, index_expr, _) => {
                vars.insert(
                    proc_ident
                        .value
                        .parts
                        .iter()
                        .map(|p| p.value.value.as_str())
                        .collect::<Vec<_>>()
                        .join("."),
                );
                if let Some(expr) = index_expr {
                    expr.value.get_vars(vars);
                }
            }
            Self::IfExpr {
                condition,
                then_expr,
                else_expr,
            } => {
                condition.value.get_vars(vars);
                then_expr.value.get_vars(vars);
                if let Some(else_expr) = else_expr {
                    else_expr.value.get_vars(vars);
                }
            }
            Self::ForAllExpr { var, list, body } => {
                list.value.get_vars(vars);
                body.value.get_vars(vars);
                vars.remove(&var.value.value);
            }
            Self::ExistsExpr { var, list, body } => {
                list.value.get_vars(vars);
                body.value.get_vars(vars);
                vars.remove(&var.value.value);
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct LocalLiteralNode {
    pub value: Literal,
}
#[derive(Debug, PartialEq, Clone)]
pub struct LocalVarNode {
    pub index: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum LocalPrimaryExpressionNode {
    Literal(LocalLiteralNode),
    Var(LocalVarNode),
    Expression(Box<LocalExpressionNode>),
}

impl fmt::Display for LocalPrimaryExpressionNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Literal(node) => write!(f, "{}", node.value),
            Self::Var(node) => write!(f, "[{}]", node.index),
            Self::Expression(node) => write!(f, "({})", node),
        }
    }
}

impl LocalPrimaryExpressionNode {
    pub fn from_primary(
        primary: &PrimaryExpression,
        program_stack: &Vec<Variable>,
    ) -> AlthreadResult<Self> {
        Ok(match primary {
            PrimaryExpression::Literal(node) => {
                LocalPrimaryExpressionNode::Literal(LocalLiteralNode::from_literal(node)?)
            }
            PrimaryExpression::Identifier(node) => {
                let full_name = node
                    .value
                    .parts
                    .iter()
                    .map(|p| p.value.value.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                let index = program_stack
                    .iter()
                    .rev()
                    .position(|var| var.name == full_name)
                    .ok_or(AlthreadError::new(
                        ErrorType::VariableError,
                        Some(node.pos.clone()),
                        format!("Variable '{}' not found", full_name),
                    ))?;
                LocalPrimaryExpressionNode::Var(LocalVarNode { index })
            }
            PrimaryExpression::Expression(node) => {
                let e = LocalExpressionNode::from_expression(&node.as_ref().value, program_stack)?;
                LocalPrimaryExpressionNode::Expression(Box::new(e))
            }
            PrimaryExpression::Reaches(_, _, _) => {
                return Err(AlthreadError::new(
                    ErrorType::ExpressionError,
                    None,
                    "'reaches' cannot be used as a local primary expression".to_string(),
                ))
            }
            PrimaryExpression::IfExpr { .. }
            | PrimaryExpression::ForAllExpr { .. }
            | PrimaryExpression::ExistsExpr { .. } => {
                return Err(AlthreadError::new(
                    ErrorType::ExpressionError,
                    None,
                    "This expression is only supported in always/eventually blocks".to_string(),
                ))
            }
        })
    }

    pub fn datatype(&self, state: &CompilerState) -> Result<DataType, String> {
        match self {
            Self::Expression(e) => e.datatype(state),
            Self::Literal(l) => Ok(l.value.get_datatype()),
            Self::Var(v) => {
                let mem_len = state.program_stack.len();
                //println!("   var {}:{}", v.index, state.program_stack.get(v.index).expect("variable index does not exists").datatype);
                Ok(state
                    .program_stack
                    .get(mem_len - 1 - v.index)
                    .expect("variable index does not exists")
                    .datatype
                    .clone())
            }
        }
    }
}

impl LocalLiteralNode {
    pub fn from_literal(literal: &Node<Literal>) -> AlthreadResult<Self> {
        Ok(LocalLiteralNode {
            value: literal.value.clone(),
        })
    }
}
impl LocalVarNode {
    pub fn from_identifier(
        ident: &Node<Identifier>,
        program_stack: &Vec<Variable>,
    ) -> AlthreadResult<Self> {
        let index = program_stack
            .iter()
            .rev()
            .position(|var| var.name == ident.value.value)
            .ok_or(AlthreadError::new(
                ErrorType::VariableError,
                Some(ident.pos.clone()),
                format!("Variable '{}' not found", ident.value.value),
            ))?;
        Ok(LocalVarNode { index })
    }
}

impl AstDisplay for PrimaryExpression {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &crate::ast::display::Prefix) -> fmt::Result {
        match self {
            Self::Literal(node) => node.ast_fmt(f, prefix),
            PrimaryExpression::Identifier(value) => {
                return writeln!(
                    f,
                    "{prefix}ident: {}",
                    value
                        .value
                        .parts
                        .iter()
                        .map(|p| p.value.value.as_str())
                        .collect::<Vec<_>>()
                        .join(".")
                );
            }
            PrimaryExpression::Expression(node) => node.ast_fmt(f, prefix),
            PrimaryExpression::Reaches(proc_ident, index_expr, label_ident) => {
                let target = proc_ident
                    .value
                    .parts
                    .iter()
                    .map(|p| p.value.value.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                if index_expr.is_some() {
                    writeln!(
                        f,
                        "{prefix}reaches: {}.at(...) .reaches({})",
                        target, label_ident.value.value
                    )
                } else {
                    writeln!(
                        f,
                        "{prefix}reaches: {}.reaches({})",
                        target, label_ident.value.value
                    )
                }
            }
            PrimaryExpression::IfExpr { .. } => {
                writeln!(f, "{prefix}if_expr")
            }
            PrimaryExpression::ForAllExpr { .. } => {
                writeln!(f, "{prefix}forall_expr")
            }
            PrimaryExpression::ExistsExpr { .. } => {
                writeln!(f, "{prefix}exists_expr")
            }
        }
    }
}
