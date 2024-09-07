pub mod binary_expression;
pub mod primary_expression;
pub mod unary_expression;

use std::{collections::HashSet, fmt};

use binary_expression::{BinaryExpression, LocalBinaryExpressionNode};
use pest::{iterators::Pairs, pratt_parser::PrattParser};
use primary_expression::{LocalPrimaryExpressionNode, PrimaryExpression};
use unary_expression::{LocalUnaryExpressionNode, UnaryExpression};

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::{datatype::DataType, literal::Literal},
    }, compiler::{CompilerState, Variable}, error::{AlthreadError, AlthreadResult, ErrorType}, parser::Rule, vm::instruction::{ExpressionControl, GlobalReadsControl, Instruction, InstructionType}
};

lazy_static::lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};

        PrattParser::new()
            .op(Op::infix(Rule::or_operator, Left))
            .op(Op::infix(Rule::and_operator, Left))
            .op(Op::infix(Rule::equality_operator, Left))
            .op(Op::infix(Rule::comparison_operator, Left))
            .op(Op::infix(Rule::term_operator, Left))
            .op(Op::infix(Rule::factor_operator, Left))
            .op(Op::prefix(Rule::unary_operator))
    };
}

#[derive(Debug)]
pub enum Expression {
    Binary(Node<BinaryExpression>),
    Unary(Node<UnaryExpression>),
    Primary(Node<PrimaryExpression>),
}

//#[derive(Debug)]
pub struct LocalExpression {
    pub root: LocalExpressionNode,
}


#[derive(Debug)]
pub enum LocalExpressionNode {
    Binary(LocalBinaryExpressionNode),
    Unary(LocalUnaryExpressionNode),
    Primary(LocalPrimaryExpressionNode),
}


pub fn parse_expr(pairs: Pairs<Rule>) -> AlthreadResult<Node<Expression>> {
    PRATT_PARSER
        .map_primary(|primary| {
            Ok(Node {
                line: primary.line_col().0,
                column: primary.line_col().1,
                value: Expression::Primary(PrimaryExpression::build(primary)?),
            })
        })
        .map_infix(|left, op, right| {
            Ok(Node {
                line: op.line_col().0,
                column: op.line_col().1,
                value: Expression::Binary(BinaryExpression::build(left?, op, right?)?),
            })
        })
        .map_prefix(|op, right| {
            Ok(Node {
                line: op.line_col().0,
                column: op.line_col().1,
                value: Expression::Unary(UnaryExpression::build(op, right?)?),
            })
        })
        .parse(pairs)
}

impl NodeBuilder for Expression {
    fn build(pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        parse_expr(pairs).map(|node| node.value)
    }
}


impl LocalExpressionNode {
    pub fn from_expression(expression: &Expression, program_stack: &Vec<Variable>) -> AlthreadResult<Self> {
        let root = match expression {
            Expression::Binary(node) =>    
                LocalExpressionNode::Binary(LocalBinaryExpressionNode::from_binary(&node.value, program_stack)?),
            Expression::Unary(node) =>
                LocalExpressionNode::Unary(LocalUnaryExpressionNode::from_unary(&node.value, program_stack)?),
            Expression::Primary(node) =>
                LocalExpressionNode::Primary(LocalPrimaryExpressionNode::from_primary(&node.value, program_stack)?),
        };
        Ok(root)
    }
    pub fn datatype(&self, state: &CompilerState) -> Result<DataType, String> {
        match self {
            Self::Binary(node) =>node.datatype(state),
            Self::Unary(node) => node.datatype(state),
            Self::Primary(node) =>
                node.datatype(state),
        }
    }
}



// we build directly the traits on the node
// because we need line/column information
impl InstructionBuilder for Node<Expression> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {

        let mut instructions = Vec::new();

        let mut vars = HashSet::new();
        self.value.get_vars(&mut vars);

        vars.retain(|var| state.global_table.contains_key(var));

        for var in vars.iter() {
            state.program_stack.push(Variable {
                name: var.clone(),
                depth: state.current_stack_depth,
                mutable: false,
                datatype: state.global_table.get(var).expect(&format!("Error: Variable '{}' not found in global table", var)).datatype.clone(),
            });
        }
        if vars.len() > 0 {
            instructions.push(Instruction {
                line: self.line,
                column: self.column,
                control: InstructionType::GlobalReads(GlobalReadsControl {
                    variables: vars.into_iter().collect(),
                }),
            });
        }
        
        let local_expr = LocalExpressionNode::from_expression(&self.value, &state.program_stack)?;
        let restult_type = local_expr.datatype(state).map_err(|err| AlthreadError::new(
            ErrorType::ExpressionError,
            self.line,
            self.column,
            format!("Type of expression is not well-defined: {}", err)
        ))?;

        instructions.push(Instruction {
            line: self.line,
            column: self.column,
            control: InstructionType::Expression(ExpressionControl {
                root: local_expr,
            })
        });

        state.program_stack.push(Variable {
            name: "".to_string(),
            depth: state.current_stack_depth,
            mutable: false,
            datatype: restult_type, // TODO: get datatype from expression
        });
        
        Ok(instructions)
    }
}


impl Expression {
    pub fn get_vars(&self, vars: &mut HashSet<String>) {
        match self {
            Self::Binary(node) => node.value.get_vars(vars),
            Self::Unary(node) => node.value.get_vars(vars),
            Self::Primary(node) => node.value.get_vars(vars),
        }
    }
}

impl AstDisplay for Expression {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        match self {
            Self::Binary(node) => node.ast_fmt(f, prefix),
            Self::Unary(node) => node.ast_fmt(f, prefix),
            Self::Primary(node) => node.ast_fmt(f, prefix),
        }
    }
}
