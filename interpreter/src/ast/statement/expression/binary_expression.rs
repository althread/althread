use std::{collections::HashSet, fmt};

use pest::iterators::Pair;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{Node},
        token::{binary_operator::BinaryOperator, literal::Literal},
    }, compiler::Variable, error::{AlthreadError, AlthreadResult, ErrorType}, parser::Rule
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
