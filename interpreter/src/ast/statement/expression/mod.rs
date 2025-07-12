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
use primary_expression::{LocalPrimaryExpressionNode, LocalVarNode, PrimaryExpression};
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
    FnCall(Node<FnCall>),
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
    FnCall(Box<Node<FnCall>>),
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
            Self::FnCall(node) => write!(f, "{:?}", node),
        }
    }
}

pub fn parse_expr(pairs: Pairs<Rule>) -> AlthreadResult<Node<Expression>> {
    PRATT_PARSER
        .map_primary(|primary| {
            match primary.as_rule() {
                Rule::fn_call => {
                    Ok(Node {
                        pos: Pos {
                            line: primary.line_col().0,
                            col: primary.line_col().1,
                            start: primary.as_span().start(),
                            end: primary.as_span().end(),
                        },
                        value: Expression::FnCall(Node::build(primary)?),
                    })
                },
                _ => 
                Ok(Node {
                pos: Pos {
                    line: primary.line_col().0,
                    col: primary.line_col().1,
                    start: primary.as_span().start(),
                    end: primary.as_span().end(),
                },
                value: Expression::Primary(PrimaryExpression::build(primary)?),
                }),
            }
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
            Expression::FnCall(node) => LocalExpressionNode::FnCall(Box::new(node.clone())),
            Expression::Tuple(node) => LocalExpressionNode::Tuple(
                LocalTupleExpressionNode::from_tuple(&node.value, program_stack)?,
            ),
            Expression::Range(node) => LocalExpressionNode::Range(
                LocalRangeListExpressionNode::from_range(&node.value, program_stack)?,
            ),
        };
        Ok(root)
    }

    pub fn contains_fn_call(&self) -> bool {
        match self {
            LocalExpressionNode::FnCall(_) => true,
            LocalExpressionNode::Binary(n) => {
                n.left.contains_fn_call() || n.right.contains_fn_call()
            }
            LocalExpressionNode::Unary(n) => n.operand.contains_fn_call(),
            LocalExpressionNode::Primary(n) => match n {
                LocalPrimaryExpressionNode::Expression(e) => e.contains_fn_call(),
                _ => false,
            },
            LocalExpressionNode::Tuple(n) => n.values.iter().any(|e| e.contains_fn_call()),
            LocalExpressionNode::Range(n) => {
                n.expression_start.contains_fn_call() || n.expression_end.contains_fn_call()
            }
        }
    }

    pub fn datatype(&self, state: &CompilerState) -> Result<DataType, String> {
        match self {
            Self::Binary(node) => node.datatype(state),
            Self::Unary(node) => node.datatype(state),
            Self::Primary(node) => node.datatype(state),
            Self::Tuple(node) => node.datatype(state),
            Self::Range(node) => node.datatype(state),
            Self::FnCall(node) => {
                let full_name = node.value.fn_name_to_string();

                if state.user_functions.contains_key(&full_name) || node.value.fn_name.value.parts.len() == 1 {
                    let fn_name = if state.user_functions.contains_key(&full_name) {
                        &full_name
                    } else {
                        &node.value.fn_name.value.parts[0].value.value
                    };

                    if let Some(func_def) = state.user_functions.get(fn_name) {
                        Ok(func_def.return_type.clone())
                    } else {
                        Err(format!("Function {} not found", fn_name))
                    }
                } else {
                    // Method call
                    let receiver_name = &node.value.fn_name.value.parts[0].value.value;
                    let var = state.program_stack.iter().rev().find(|v| &v.name == receiver_name);
                    if let Some(var) = var {
                        if let Some(interfaces) = state.stdlib.get_interfaces(&var.datatype) {
                            let method_name = &node.value.fn_name.value.parts.last().unwrap().value.value;
                            if let Some(method) = interfaces.iter().find(|m| &m.name == method_name) {
                                Ok(method.ret.clone())
                            } else {
                                Err(format!("Method {} not found in interface", method_name))
                            }
                        } else {
                            Err(format!("No interface found for type {}", var.datatype))
                        }
                    } else {
                        Err(format!("Receiver {} not found in stack", receiver_name))
                    }
                }
            }
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
            LocalExpressionNode::FnCall(node) => {
                Err(format!("Cannot evaluate function call in this context: {:?}", &node.value.fn_name))
            }
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

        if !vars.is_empty() {
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
            instructions.push(Instruction {
                pos: Some(self.pos),
                control: InstructionType::GlobalReads {
                    only_const: vars.iter().all(|v| state.global_table[v].mutable == false),
                    variables: vars.into_iter().collect(),
                },
            });
        }

        let local_expr = LocalExpressionNode::from_expression(&self.value, &state.program_stack)?;

        let result_type = local_expr.datatype(state).map_err(|err| {
            AlthreadError::new(
                ErrorType::ExpressionError,
                Some(self.pos),
                format!("Type of expression is not well-defined: {}", err),
            )
        })?;

        if !local_expr.contains_fn_call() {
            instructions.push(Instruction {
                pos: Some(self.pos),
                control: InstructionType::Expression(local_expr),
            });
        } else {
            fn shift_var_indices(expr: &LocalExpressionNode, shift: usize) -> LocalExpressionNode {
                match expr {
                    LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Var(var)) => {
                        LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Var(LocalVarNode {
                            index: var.index + shift,
                        }))
                    }
                    LocalExpressionNode::Binary(node) => {
                        LocalExpressionNode::Binary(LocalBinaryExpressionNode {
                            left: Box::new(shift_var_indices(&node.left, shift)),
                            operator: node.operator.clone(),
                            right: Box::new(shift_var_indices(&node.right, shift)),
                        })
                    }
                    LocalExpressionNode::Unary(node) => {
                        LocalExpressionNode::Unary(LocalUnaryExpressionNode {
                            operand: Box::new(shift_var_indices(&node.operand, shift)),
                            operator: node.operator.clone(),
                        })
                    }
                    LocalExpressionNode::Tuple(node) => LocalExpressionNode::Tuple(
                        LocalTupleExpressionNode {
                            values: node.values.iter().map(|v| shift_var_indices(v, shift)).collect(),
                        },
                    ),
                    LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(
                        expr,
                    )) => LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(
                        Box::new(shift_var_indices(expr, shift)),
                    )),
                    LocalExpressionNode::Range(node) => {
                        LocalExpressionNode::Range(LocalRangeListExpressionNode {
                            expression_start: Box::new(shift_var_indices(
                                &node.expression_start,
                                shift,
                            )),
                            expression_end: Box::new(shift_var_indices(&node.expression_end, shift)),
                        })
                    }
                    _ => expr.clone(),
                }
            }

            fn compile_recursive(
                expr: &LocalExpressionNode,
                state: &mut CompilerState,
            ) -> AlthreadResult<(LocalExpressionNode, InstructionBuilderOk, usize)> {
                match expr {
                    LocalExpressionNode::FnCall(node) => {
                        let builder = node.compile(state)?;
                        state.program_stack.pop();
                        let placeholder = LocalExpressionNode::Primary(
                            LocalPrimaryExpressionNode::Var(LocalVarNode { index: 0 }),
                        );
                        Ok((placeholder, builder, 1))
                    }
                    LocalExpressionNode::Binary(node) => {
                        // Compile left side first, to match execution order.
                        let (left_expr, mut left_builder, left_calls) =
                            compile_recursive(&node.left, state)?;

                        // Temporarily update the compiler's stack to account for the
                        // return values from the left side's function calls.
                        let temp_vars_added = left_calls;
                        for _ in 0..temp_vars_added {
                            state.program_stack.push(Variable {
                                name: "<temp_fn_return>".to_string(),
                                depth: state.current_stack_depth,
                                mutable: false,
                                // Using a placeholder type. The actual type is unknown here,
                                // but it's only needed to adjust stack indices.
                                datatype: DataType::Void,
                                declare_pos: None,
                            });
                        }

                        // Compile the right side with the adjusted stack.
                        let (right_expr, right_builder, right_calls) =
                            compile_recursive(&node.right, state)?;

                        // Restore the compiler's stack.
                        for _ in 0..temp_vars_added {
                            state.program_stack.pop();
                        }

                        // Combine the instructions.
                        left_builder.extend(right_builder);

                        // The placeholder for the left result must be shifted by the number
                        // of results from the right side.
                        let shifted_left = if right_calls > 0 {
                            shift_var_indices(&left_expr, right_calls)
                        } else {
                            left_expr
                        };

                        let new_expr = LocalExpressionNode::Binary(LocalBinaryExpressionNode {
                            left: Box::new(shifted_left),
                            right: Box::new(right_expr),
                            operator: node.operator.clone(),
                        });

                        Ok((new_expr, left_builder, left_calls + right_calls))
                    }
                    LocalExpressionNode::Unary(node) => {
                        let (operand_expr, builder, calls) =
                            compile_recursive(&node.operand, state)?;
                        let new_expr = LocalExpressionNode::Unary(LocalUnaryExpressionNode {
                            operand: Box::new(operand_expr),
                            operator: node.operator.clone(),
                        });
                        Ok((new_expr, builder, calls))
                    }
                    LocalExpressionNode::Tuple(node) => {
                        let mut compiled_elements = Vec::new();
                        let mut builder = InstructionBuilderOk::new();
                        let mut total_calls = 0;
                        let mut elements_with_calls = Vec::new();

                        for element in node.values.iter().rev() {
                            let (new_elem, new_builder, num_calls) =
                                compile_recursive(element, state)?;
                            elements_with_calls.push((new_elem, num_calls));
                            builder.extend(new_builder);
                            total_calls += num_calls;
                        }
                        elements_with_calls.reverse();

                        let mut calls_processed = 0;
                        for (elem, calls) in elements_with_calls {
                            if calls > 0 {
                                let shifted_elem =
                                    shift_var_indices(&elem, total_calls - calls_processed - calls);
                                compiled_elements.push(shifted_elem);
                                calls_processed += calls;
                            } else {
                                compiled_elements.push(elem);
                            }
                        }

                        let new_tuple = LocalExpressionNode::Tuple(LocalTupleExpressionNode {
                            values: compiled_elements,
                        });
                        Ok((new_tuple, builder, total_calls))
                    }
                    LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(
                        expr,
                    )) => {
                        let (new_expr, builder, calls) = compile_recursive(expr, state)?;
                        let new_primary = LocalExpressionNode::Primary(
                            LocalPrimaryExpressionNode::Expression(Box::new(new_expr)),
                        );
                        Ok((new_primary, builder, calls))
                    }
                    LocalExpressionNode::Range(node) => {
                        let (end_expr, end_builder, end_calls) =
                            compile_recursive(&node.expression_end, state)?;
                        let (start_expr, mut start_builder, start_calls) =
                            compile_recursive(&node.expression_start, state)?;

                        start_builder.extend(end_builder);

                        let shifted_start = if start_calls > 0 {
                            shift_var_indices(&start_expr, end_calls)
                        } else {
                            start_expr
                        };

                        let new_expr =
                            LocalExpressionNode::Range(LocalRangeListExpressionNode {
                                expression_start: Box::new(shifted_start),
                                expression_end: Box::new(end_expr),
                            });

                        Ok((new_expr, start_builder, start_calls + end_calls))
                    }
                    _ => Ok((expr.clone(), InstructionBuilderOk::new(), 0)),
                }
            }

            let (final_expr, builder, fn_call_count) = compile_recursive(&local_expr, state)?;
            instructions.extend(builder.instructions);

            if fn_call_count > 0 {
                if let Expression::FnCall(_) = self.value {
                     // It's a direct function call statement, FnCall instruction handles the stack
                } else if let Expression::Tuple(_) = self.value {
                    instructions.push(Instruction {
                        pos: Some(self.pos),
                        control: InstructionType::MakeTupleAndCleanup {
                            elements: if let LocalExpressionNode::Tuple(t) = final_expr { t.values } else { vec![] },
                            unstack_len: fn_call_count,
                        },
                    });
                }
                else {
                    instructions.push(Instruction {
                        pos: Some(self.pos),
                        control: InstructionType::ExpressionAndCleanup {
                            expression: final_expr,
                            unstack_len: fn_call_count,
                        },
                    });
                }
            } else {
                 instructions.push(Instruction {
                    pos: Some(self.pos),
                    control: InstructionType::Expression(final_expr),
                });
            }
        }

        state.program_stack.push(Variable {
            name: "".to_string(),
            depth: state.current_stack_depth,
            mutable: false,
            datatype: result_type,
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
            Self::FnCall(node) => node.value.add_dependencies(dependencies),
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
            Self::FnCall(node) => node.value.get_vars(vars),
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
            Self::FnCall(node) => node.ast_fmt(f, prefix),
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
