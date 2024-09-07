use std::{collections::HashSet, fmt};

use pest::iterators::Pair;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::Node,
        token::{binary_operator::BinaryOperator, datatype::DataType, literal::Literal},
    }, compiler::{CompilerState, Variable}, error::{AlthreadError, AlthreadResult, ErrorType}, parser::Rule
};

use super::{Expression, LocalExpressionNode};

#[derive(Debug)]
pub struct BinaryExpression {
    pub left: Box<Node<Expression>>,
    pub operator: Node<BinaryOperator>,
    pub right: Box<Node<Expression>>,
}

#[derive(Debug)]
pub struct LocalBinaryExpressionNode {
    pub left: Box<LocalExpressionNode>,
    pub operator: BinaryOperator,
    pub right: Box<LocalExpressionNode>,
}

impl BinaryExpression {
    pub fn build(
        left: Node<Expression>,
        operator: Pair<Rule>,
        right: Node<Expression>,
    ) -> AlthreadResult<Node<Self>> {
        Ok(Node {
            line: operator.line_col().0,
            column: operator.line_col().1,
            value: Self {
                left: Box::new(left),
                operator: Node::build(operator)?,
                right: Box::new(right),
            },
        })
    }
}

impl LocalBinaryExpressionNode {
    
    pub fn from_binary(bin_expression: &BinaryExpression, program_stack: &Vec<Variable>) -> AlthreadResult<Self> {
        Ok(Self {
            left: Box::new(LocalExpressionNode::from_expression(&bin_expression.left.value, program_stack)?),
            operator: bin_expression.operator.value.clone(),
            right: Box::new(LocalExpressionNode::from_expression(&bin_expression.right.value, program_stack)?),
        })    
    }

    pub fn datatype(&self, state: &CompilerState) -> Result<DataType, String> {
        let left_type = self.left.datatype(state)?;
        let right_type = self.right.datatype(state)?;
        match self.operator {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide => if left_type.is_a_number() && left_type == right_type {
                Ok(left_type)
            } else {
                Err(format!("arithmetic operation can only be performed between two number types that are exactly the same (found {} {} {})", left_type, self.operator, right_type))
            },
            BinaryOperator::Modulo => if left_type.is_a_number() && right_type == DataType::Integer {
                Ok(left_type)
            } else {
                Err("modulo can only be performed between a number and an integer".to_string())
            },
            BinaryOperator::Equals
            | BinaryOperator::NotEquals => if left_type == right_type {
                Ok(DataType::Boolean)
            } else {
                Err(format!("equality check can only be performed between values that have exaclty the same type (found {} {} {})", left_type, self.operator, right_type))
            },
            BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual => if left_type.is_a_number() && left_type == right_type {
                Ok(DataType::Boolean)
            } else {
                Err("arithmetic comparison can only be performed between two number types that are exactly the same".to_string())
            },
            BinaryOperator::And
            | BinaryOperator::Or => if left_type.is_boolean() && right_type.is_boolean() {
                Ok(DataType::Boolean)
            } else {
                Err("boolean operations can only be performed between boolean values".to_string())
            },
        }
    }
}


impl BinaryExpression {
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
