use std::{collections::HashSet, fmt};

use pest::iterators::Pair;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::Node,
        statement::waiting_case::WaitDependency,
        token::{binary_operator::BinaryOperator, datatype::DataType, literal::Literal},
    },
    compiler::{CompilerState, Variable},
    error::{AlthreadResult, Pos},
    parser::Rule,
    vm::Memory,
};

use super::{Expression, LocalExpressionNode};

#[derive(Debug, PartialEq, Clone)]
pub struct BinaryExpression {
    pub left: Box<Node<Expression>>,
    pub operator: Node<BinaryOperator>,
    pub right: Box<Node<Expression>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LocalBinaryExpressionNode {
    pub left: Box<LocalExpressionNode>,
    pub operator: BinaryOperator,
    pub right: Box<LocalExpressionNode>,
}
impl fmt::Display for LocalBinaryExpressionNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} {}", self.left, self.operator, self.right)
    }
}

impl BinaryExpression {
    pub fn build(
        left: Node<Expression>,
        operator: Pair<Rule>,
        right: Node<Expression>,
    ) -> AlthreadResult<Node<Self>> {
        Ok(Node {
            pos: Pos {
                start: left.pos.start,
                end: right.pos.end,
                line: left.pos.line,
                col: left.pos.col,
            },
            value: Self {
                left: Box::new(left),
                operator: Node::build(operator)?,
                right: Box::new(right),
            },
        })
    }
}

impl LocalBinaryExpressionNode {
    pub fn from_binary(
        bin_expression: &BinaryExpression,
        program_stack: &Vec<Variable>,
    ) -> AlthreadResult<Self> {
        Ok(Self {
            left: Box::new(LocalExpressionNode::from_expression(
                &bin_expression.left.value,
                program_stack,
            )?),
            operator: bin_expression.operator.value.clone(),
            right: Box::new(LocalExpressionNode::from_expression(
                &bin_expression.right.value,
                program_stack,
            )?),
        })
    }

    pub fn datatype(&self, state: &CompilerState) -> Result<DataType, String> {
        let left_type = self.left.datatype(state)?;
        let right_type = self.right.datatype(state)?;
        match self.operator {
            BinaryOperator::Add =>{
                if left_type.is_a_number() && left_type == right_type {
                    Ok(left_type)
                } else if left_type == DataType::String || right_type == DataType::String {
                    Ok(DataType::String)
                } else {
                    Err(format!(
                        "addition can only be performed between two identical number types or when at least one operand is a string (found {} + {})",
                        left_type, right_type
                    ))
                }
            }
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide => {
                if left_type.is_a_number() && left_type == right_type {
                    Ok(left_type)
                } else {
                    Err(format!("arithmetic operation can only be performed between two number types that are exactly the same (found {} {} {})", left_type, self.operator, right_type))
                }
            }
            BinaryOperator::Modulo => {
                if left_type.is_a_number() && right_type == DataType::Integer {
                    Ok(left_type)
                } else {
                    Err("modulo can only be performed between a number and an integer".to_string())
                }
            }
            BinaryOperator::Equals | BinaryOperator::NotEquals => {
                if left_type == right_type {
                    Ok(DataType::Boolean)
                } else {
                    Err(format!("equality check can only be performed between values that have exaclty the same type (found {} {} {})", left_type, self.operator, right_type))
                }
            }
            BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual => {
                if left_type.is_a_number() && left_type == right_type {
                    Ok(DataType::Boolean)
                } else {
                    Err("arithmetic comparison can only be performed between two number types that are exactly the same".to_string())
                }
            }
            BinaryOperator::And | BinaryOperator::Or => {
                if left_type.is_boolean() && right_type.is_boolean() {
                    Ok(DataType::Boolean)
                } else {
                    Err(
                        "boolean operations can only be performed between boolean values"
                            .to_string(),
                    )
                }
            }
        }
    }

    pub fn eval(&self, mem: &Memory) -> Result<Literal, String> {
        let left = self.left.eval(mem)?;
        let right = self.right.eval(mem)?;

        match self.operator {
            BinaryOperator::Add => left.add(&right),
            BinaryOperator::Subtract => left.subtract(&right),
            BinaryOperator::Multiply => left.multiply(&right),
            BinaryOperator::Divide => left.divide(&right),
            BinaryOperator::Modulo => left.modulo(&right),
            BinaryOperator::Equals => left.equals(&right),
            BinaryOperator::NotEquals => left.not_equals(&right),
            BinaryOperator::LessThan => left.less_than(&right),
            BinaryOperator::LessThanOrEqual => left.less_than_or_equal(&right),
            BinaryOperator::GreaterThan => left.greater_than(&right),
            BinaryOperator::GreaterThanOrEqual => right.greater_than_or_equal(&right),
            BinaryOperator::And => left.and(&right),
            BinaryOperator::Or => left.or(&right),
        }
    }
}

impl BinaryExpression {
    pub fn add_dependencies(&self, dependencies: &mut WaitDependency) {
        self.left.value.add_dependencies(dependencies);
        self.right.value.add_dependencies(dependencies);
    }
    pub fn get_vars(&self, vars: &mut HashSet<String>) {
        self.left.value.get_vars(vars);
        self.right.value.get_vars(vars);
    }
}

impl AstDisplay for BinaryExpression {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}binary_expr")?;

        let prefix = &prefix.add_branch();
        writeln!(f, "{}left", prefix)?;
        self.left.ast_fmt(f, &prefix.add_leaf())?;

        writeln!(f, "{}op: {}", prefix, self.operator)?;

        let prefix = &prefix.switch();
        writeln!(f, "{}right", prefix)?;
        self.right.ast_fmt(f, &prefix.add_leaf())?;

        Ok(())
    }
}
