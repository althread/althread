pub mod binary_expression;
pub mod list_expression;
pub mod primary_expression;
pub mod tuple_expression;
pub mod unary_expression;

use std::{collections::HashSet, fmt};

use binary_expression::{BinaryExpression, LocalBinaryExpressionNode};
use list_expression::{LocalRangeListExpressionNode, RangeListExpression};
use pest::{
    iterators::{Pair, Pairs},
    pratt_parser::PrattParser,
};
use primary_expression::{LocalPrimaryExpressionNode, PrimaryExpression};
use tuple_expression::{LocalTupleExpressionNode, TupleExpression};
use unary_expression::{LocalUnaryExpressionNode, UnaryExpression};

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::{datatype::DataType, literal::Literal},
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType, Pos},
    no_rule,
    parser::Rule,
    vm::{
        instruction::{Instruction, InstructionType},
        Memory,
    },
};

use super::{fn_call::FnCall, run_call::RunCall, waiting_case::WaitDependency};

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

#[derive(Debug, Clone)]
pub enum SideEffectExpression {
    Expression(Node<Expression>),
    RunCall(Node<RunCall>),
    FnCall(Node<FnCall>),
}

impl NodeBuilder for SideEffectExpression {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();

        match pair.as_rule() {
            Rule::expression => Ok(Self::Expression(Node::build(pair)?)),
            Rule::run_call => Ok(Self::RunCall(Node::build(pair)?)),
            Rule::fn_call => Ok(Self::FnCall(Node::build(pair)?)),
            _ => Err(no_rule!(pair, "SideEffectExpression")),
        }
    }
}

impl InstructionBuilder for SideEffectExpression {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        match self {
            Self::Expression(node) => node.compile(state),
            Self::RunCall(node) => node.compile(state),
            Self::FnCall(node) => node.compile(state),
        }
    }
}

impl AstDisplay for SideEffectExpression {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        match self {
            Self::Expression(node) => node.ast_fmt(f, prefix),
            Self::RunCall(node) => node.ast_fmt(f, prefix),
            Self::FnCall(node) => node.ast_fmt(f, prefix),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    Binary(Node<BinaryExpression>),
    Unary(Node<UnaryExpression>),
    Primary(Node<PrimaryExpression>),
    Tuple(Node<TupleExpression>),
    Range(Node<RangeListExpression>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct LocalExpression {
    pub root: LocalExpressionNode,
}

#[derive(Debug, PartialEq, Clone)]
pub enum LocalExpressionNode {
    Binary(LocalBinaryExpressionNode),
    Unary(LocalUnaryExpressionNode),
    Primary(LocalPrimaryExpressionNode),
    Tuple(LocalTupleExpressionNode),
    Range(LocalRangeListExpressionNode),
}
impl fmt::Display for LocalExpression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.root)
    }
}
impl fmt::Display for LocalExpressionNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Binary(node) => write!(f, "{}", node),
            Self::Unary(node) => write!(f, "{}", node),
            Self::Primary(node) => write!(f, "{}", node),
            Self::Tuple(node) => write!(f, "{}", node),
            Self::Range(node) => write!(f, "{}", node),
        }
    }
}

pub fn parse_expr(pairs: Pairs<Rule>) -> AlthreadResult<Node<Expression>> {
    PRATT_PARSER
        .map_primary(|primary| {
            Ok(Node {
                pos: Pos {
                    line: primary.line_col().0,
                    col: primary.line_col().1,
                    start: primary.as_span().start(),
                    end: primary.as_span().end(),
                },
                value: Expression::Primary(PrimaryExpression::build(primary)?),
            })
        })
        .map_infix(|left, op, right| {
            Ok(Node {
                pos: Pos {
                    line: op.line_col().0,
                    col: op.line_col().1,
                    start: op.as_span().start(),
                    end: op.as_span().end(),
                },
                value: Expression::Binary(BinaryExpression::build(left?, op, right?)?),
            })
        })
        .map_prefix(|op, right| {
            Ok(Node {
                pos: Pos {
                    line: op.line_col().0,
                    col: op.line_col().1,
                    start: op.as_span().start(),
                    end: op.as_span().end(),
                },
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
impl Expression {
    pub fn build_list_expression(pair: Pair<Rule>) -> AlthreadResult<Node<Self>> {
        let pos = Pos {
            line: pair.line_col().0,
            col: pair.line_col().1,
            start: pair.as_span().start(),
            end: pair.as_span().end(),
        };
        match pair.as_rule() {
            Rule::range_expression => {
                let mut pair = pair.into_inner();
                let expression_start: Box<Node<Expression>> =
                    Box::new(Node::build(pair.next().unwrap())?);
                let expression_end: Box<Node<Expression>> =
                    Box::new(Node::build(pair.next().unwrap())?);
                Ok(Node {
                    pos,
                    value: Expression::Range(Node {
                        pos,
                        value: RangeListExpression {
                            expression_start,
                            expression_end,
                        },
                    }),
                })
            }
            _ => Err(no_rule!(pair, "list_expression")),
        }
    }
    pub fn build_top_level(pair: Pair<Rule>) -> AlthreadResult<Node<Self>> {
        let pos = Pos {
            line: pair.line_col().0,
            col: pair.line_col().1,
            start: pair.as_span().start(),
            end: pair.as_span().end(),
        };
        match pair.as_rule() {
            Rule::expression => {
                let expr = Self::build(pair.into_inner())?;
                Ok(Node { pos, value: expr })
            }
            Rule::tuple_expression => {
                let mut values = Vec::new();
                for pair in pair.into_inner() {
                    values.push(Node::build(pair)?);
                }
                Ok(Node {
                    pos,
                    value: Expression::Tuple(Node {
                        pos,
                        value: TupleExpression { values },
                    }),
                })
            }
            _ => Err(no_rule!(pair, "Expression::build_top_level")),
        }
    }
}

impl LocalExpressionNode {
    pub fn from_expression(
        expression: &Expression,
        program_stack: &Vec<Variable>,
    ) -> AlthreadResult<Self> {
        let root = match expression {
            Expression::Binary(node) => LocalExpressionNode::Binary(
                LocalBinaryExpressionNode::from_binary(&node.value, program_stack)?,
            ),
            Expression::Unary(node) => LocalExpressionNode::Unary(
                LocalUnaryExpressionNode::from_unary(&node.value, program_stack)?,
            ),
            Expression::Primary(node) => LocalExpressionNode::Primary(
                LocalPrimaryExpressionNode::from_primary(&node.value, program_stack)?,
            ),
            Expression::Tuple(node) => LocalExpressionNode::Tuple(
                LocalTupleExpressionNode::from_tuple(&node.value, program_stack)?,
            ),
            Expression::Range(node) => LocalExpressionNode::Range(
                LocalRangeListExpressionNode::from_range(&node.value, program_stack)?,
            ),
        };
        Ok(root)
    }
    pub fn datatype(&self, state: &CompilerState) -> Result<DataType, String> {
        match self {
            Self::Binary(node) => node.datatype(state),
            Self::Unary(node) => node.datatype(state),
            Self::Primary(node) => node.datatype(state),
            Self::Tuple(node) => node.datatype(state),
            Self::Range(node) => node.datatype(state),
        }
    }
    pub fn eval(&self, mem: &Memory) -> Result<Literal, String> {
        match self {
            LocalExpressionNode::Binary(binary_exp) => binary_exp.eval(mem),
            LocalExpressionNode::Unary(unary_exp) => unary_exp.eval(mem),
            LocalExpressionNode::Primary(primary_exp) => match primary_exp {
                LocalPrimaryExpressionNode::Literal(literal) => Ok(literal.value.clone()),
                LocalPrimaryExpressionNode::Var(local_var) => {
                    let lit = mem
                        .get(mem.len() - 1 - local_var.index)
                        .ok_or("local variable index does not exist in memory".to_string())?;
                    Ok(lit.clone())
                }
                LocalPrimaryExpressionNode::Expression(expr) => expr.as_ref().eval(mem),
            },
            LocalExpressionNode::Tuple(tuple_exp) => tuple_exp.eval(mem),
            LocalExpressionNode::Range(list_exp) => list_exp.eval(mem),
        }
    }
}

// we build directly the traits on the node
// because we need line/column information
impl InstructionBuilder for Node<Expression> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut instructions = Vec::new();

        let mut vars = HashSet::new();
        self.value.get_vars(&mut vars);

        vars.retain(|var| state.global_table.contains_key(var));

        for var in vars.iter() {
            let global_var = state.global_table.get(var).expect(&format!(
                "Error: Variable '{}' not found in global table",
                var
            ));
            state.program_stack.push(Variable {
                name: var.clone(),
                depth: state.current_stack_depth,
                mutable: false,
                datatype: global_var.datatype.clone(),
                declare_pos: global_var.declare_pos,
            });
        }
        if vars.len() > 0 {
            instructions.push(Instruction {
                pos: Some(self.pos),
                control: InstructionType::GlobalReads {
                    only_const: vars.iter().all(|v| state.global_table[v].mutable == false),
                    variables: vars.into_iter().collect(),
                },
            });
        }

        let local_expr = LocalExpressionNode::from_expression(&self.value, &state.program_stack)?;
        let restult_type = local_expr.datatype(state).map_err(|err| {
            AlthreadError::new(
                ErrorType::ExpressionError,
                Some(self.pos),
                format!("Type of expression is not well-defined: {}", err),
            )
        })?;

        instructions.push(Instruction {
            pos: Some(self.pos),
            control: InstructionType::Expression(local_expr),
        });

        state.program_stack.push(Variable {
            name: "".to_string(),
            depth: state.current_stack_depth,
            mutable: false,
            datatype: restult_type,
            declare_pos: None,
        });

        Ok(InstructionBuilderOk::from_instructions(instructions))
    }
}

impl Expression {
    pub fn add_dependencies(&self, dependencies: &mut WaitDependency) {
        match self {
            Self::Binary(node) => node.value.add_dependencies(dependencies),
            Self::Unary(node) => node.value.add_dependencies(dependencies),
            Self::Primary(node) => node.value.add_dependencies(dependencies),
            Self::Tuple(node) => node.value.add_dependencies(dependencies),
            Self::Range(node) => node.value.add_dependencies(dependencies),
        }
    }
    pub fn is_tuple(&self) -> bool {
        match self {
            Self::Tuple(_) => true,
            _ => false,
        }
    }
}

impl Expression {
    pub fn get_vars(&self, vars: &mut HashSet<String>) {
        match self {
            Self::Binary(node) => node.value.get_vars(vars),
            Self::Unary(node) => node.value.get_vars(vars),
            Self::Primary(node) => node.value.get_vars(vars),
            Self::Tuple(node) => node.value.get_vars(vars),
            Self::Range(node) => node.value.get_vars(vars),
        }
    }
}

impl AstDisplay for Expression {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        match self {
            Self::Binary(node) => node.ast_fmt(f, prefix),
            Self::Unary(node) => node.ast_fmt(f, prefix),
            Self::Primary(node) => node.ast_fmt(f, prefix),
            Self::Tuple(node) => node.ast_fmt(f, prefix),
            Self::Range(node) => node.ast_fmt(f, prefix),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::token::{binary_operator::BinaryOperator, literal::Literal};

    #[test]
    fn test_literal_expression() {
        let litteral_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
            },
            value: Literal::Int(42),
        };
        let primary_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
            },
            value: PrimaryExpression::Literal(litteral_node),
        };
        let litteral_expr = Expression::Primary(primary_node);
        let local_expr = LocalExpressionNode::from_expression(&litteral_expr, &vec![]).unwrap();
        assert_eq!(local_expr.eval(&Memory::new()).unwrap(), Literal::Int(42));
    }
    #[test]
    fn test_binary_expression() {
        let litteral_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
            },
            value: Literal::Int(42),
        };
        let primary_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
            },
            value: PrimaryExpression::Literal(litteral_node),
        };

        let litteral_expr = Expression::Primary(primary_node);

        let binary_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
            },
            value: BinaryExpression {
                left: Box::new(Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                    },
                    value: litteral_expr.clone(),
                }),
                right: Box::new(Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                    },
                    value: litteral_expr.clone(),
                }),
                operator: Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                    },
                    value: BinaryOperator::Add,
                },
            },
        };
        let binary_expr = Expression::Binary(binary_node);
        let local_expr = LocalExpressionNode::from_expression(&binary_expr, &vec![]).unwrap();
        assert_eq!(
            local_expr.eval(&Memory::new()).unwrap(),
            Literal::Int(42 + 42)
        );
    }
}
