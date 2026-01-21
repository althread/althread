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
        token::{datatype::DataType, identifier::Identifier, literal::Literal},
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

#[derive(Debug, PartialEq, Clone)]
pub enum SideEffectExpression {
    Expression(Node<Expression>),
    RunCall(Node<RunCall>),
    FnCall(Node<FnCall>),
    Bracket(Node<BracketExpression>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct BracketExpression {
    pub content: BracketContent,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BracketContent {
    Range(Node<RangeListExpression>),
    ListLiteral(Vec<Node<SideEffectExpression>>),
}

impl NodeBuilder for SideEffectExpression {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();

        match pair.as_rule() {
            Rule::expression => Ok(Self::Expression(Node::build(pair, filepath)?)),
            Rule::run_call => Ok(Self::RunCall(Node::build(pair, filepath)?)),
            Rule::fn_call => Ok(Self::FnCall(Node::build(pair, filepath)?)),
            Rule::bracket_expression => Ok(Self::Bracket(Node::build(pair, filepath)?)),
            _ => Err(no_rule!(pair, "SideEffectExpression", filepath)),
        }
    }
}

impl NodeBuilder for BracketExpression {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();

        let content = match pair.as_rule() {
            Rule::range_expression => {
                let range_node = Node::build(pair, filepath)?;
                BracketContent::Range(range_node)
            }
            Rule::list_literal_inner => {
                let expressions: Result<Vec<_>, _> = pair
                    .into_inner()
                    .map(|expr_pair| Node::build(expr_pair, filepath))
                    .collect();
                BracketContent::ListLiteral(expressions?)
            }
            _ => return Err(no_rule!(pair, "BracketExpression", filepath)),
        };

        Ok(BracketExpression { content })
    }
}

impl InstructionBuilder for SideEffectExpression {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        match self {
            Self::Expression(node) => node.compile(state),
            Self::RunCall(node) => node.compile(state),
            Self::FnCall(node) => node.compile(state),
            Self::Bracket(node) => node.compile(state),
        }
    }
}

impl InstructionBuilder for BracketExpression {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        match &self.content {
            BracketContent::Range(range_node) => {
                // Create an Expression::Range and compile it
                let range_expr = Node {
                    pos: range_node.pos.clone(),
                    value: Expression::Range(range_node.clone()),
                };
                range_expr.compile(state)
            }
            BracketContent::ListLiteral(expressions) => {
                let mut instructions = Vec::new();

                // Determine element type from first expression
                let element_type = if let Some(first_expr) = expressions.first() {
                    match &first_expr.value {
                        SideEffectExpression::Expression(node) => {
                            let local_expr = LocalExpressionNode::from_expression(
                                &node.value,
                                &state.program_stack,
                            )?;
                            local_expr.datatype(state).map_err(|err| {
                                AlthreadError::new(
                                    ErrorType::ExpressionError,
                                    Some(node.pos.clone()),
                                    format!("Cannot infer type of list element: {}", err),
                                )
                            })?
                        }
                        SideEffectExpression::FnCall(node) => {
                            let local_expr = LocalExpressionNode::FnCall(Box::new(node.clone()));
                            local_expr.datatype(state).map_err(|err| {
                                AlthreadError::new(
                                    ErrorType::ExpressionError,
                                    Some(node.pos.clone()),
                                    format!(
                                        "Cannot infer list element type from function call: {}",
                                        err
                                    ),
                                )
                            })?
                        }
                        SideEffectExpression::RunCall(_) => {
                            return Err(AlthreadError::new(
                                ErrorType::ExpressionError,
                                Some(first_expr.pos.clone()),
                                "Run calls cannot be used in list literals".to_string(),
                            ));
                        }
                        SideEffectExpression::Bracket(node) => {
                            // Compile the nested bracket to get its type
                            let _nested_builder = node.compile(state)?;
                            let nested_type = if let Some(last_var) = state.program_stack.last() {
                                let t = last_var.datatype.clone();
                                state.program_stack.pop(); // Remove the temporary variable
                                t
                            } else {
                                DataType::Void
                            };
                            nested_type
                        }
                    }
                } else {
                    DataType::Void // Empty list
                };

                // Compile each expression onto the stack
                for (i, expr) in expressions.iter().enumerate() {
                    // Forbid run calls in list literals
                    if matches!(expr.value, SideEffectExpression::RunCall(_)) {
                        return Err(AlthreadError::new(
                            ErrorType::ExpressionError,
                            Some(expr.pos.clone()),
                            "Run calls cannot be used in list literals".to_string(),
                        ));
                    }

                    // Compile the expression
                    let builder = expr.compile(state)?;
                    instructions.extend(builder.instructions);

                    // Type check if we have a determined element type
                    if element_type != DataType::Void {
                        let expr_type = match &expr.value {
                            SideEffectExpression::Expression(node) => {
                                let local_expr = LocalExpressionNode::from_expression(
                                    &node.value,
                                    &state.program_stack,
                                )?;
                                local_expr.datatype(state).map_err(|err| {
                                    AlthreadError::new(
                                        ErrorType::ExpressionError,
                                        Some(expr.pos.clone()),
                                        format!(
                                            "Cannot determine type of list element {}: {}",
                                            i, err
                                        ),
                                    )
                                })?
                            }
                            SideEffectExpression::FnCall(node) => {
                                let local_expr =
                                    LocalExpressionNode::FnCall(Box::new(node.clone()));
                                local_expr.datatype(state).map_err(|err| {
                                    AlthreadError::new(
                                        ErrorType::ExpressionError,
                                        Some(expr.pos.clone()),
                                        format!("Cannot determine type of function call in list element {}: {}", i, err),
                                    )
                                })?
                            }
                            SideEffectExpression::Bracket(_) => {
                                // Get type from the variable that was just pushed to stack
                                if let Some(last_var) = state.program_stack.last() {
                                    last_var.datatype.clone()
                                } else {
                                    return Err(AlthreadError::new(
                                        ErrorType::ExpressionError,
                                        Some(expr.pos.clone()),
                                        "Cannot determine type of bracket expression".to_string(),
                                    ));
                                }
                            }
                            SideEffectExpression::RunCall(_) => {
                                unreachable!("Run calls already filtered out above");
                            }
                        };

                        if expr_type != element_type {
                            return Err(AlthreadError::new(
                                ErrorType::ExpressionError,
                                Some(expr.pos.clone()),
                                format!(
                                    "List element {} has type {:?}, expected {:?}",
                                    i, expr_type, element_type
                                ),
                            ));
                        }
                    }
                }

                // Create list from stack elements
                instructions.push(Instruction {
                    pos: None,
                    control: InstructionType::CreateListFromStack {
                        element_count: expressions.len(),
                        element_type: element_type.clone(),
                    },
                });

                // Update stack - remove individual elements and add the list
                for _ in 0..expressions.len() {
                    state.program_stack.pop();
                }

                let list_type = DataType::List(Box::new(element_type));
                state.program_stack.push(Variable {
                    name: "".to_string(),
                    depth: state.current_stack_depth,
                    mutable: false,
                    datatype: list_type,
                    declare_pos: None,
                });

                Ok(InstructionBuilderOk::from_instructions(instructions))
            }
        }
    }
}

impl AstDisplay for SideEffectExpression {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        match self {
            Self::Expression(node) => node.ast_fmt(f, prefix),
            Self::RunCall(node) => node.ast_fmt(f, prefix),
            Self::FnCall(node) => node.ast_fmt(f, prefix),
            Self::Bracket(node) => node.ast_fmt(f, prefix),
        }
    }
}

impl AstDisplay for BracketExpression {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        match &self.content {
            BracketContent::Range(range) => {
                writeln!(f, "{}RangeExpression", prefix)?;
                range.ast_fmt(f, &prefix.add_branch())
            }
            BracketContent::ListLiteral(exprs) => {
                writeln!(f, "{}ListLiteral", prefix)?;
                let new_prefix = prefix.add_branch();
                for expr in exprs {
                    expr.ast_fmt(f, &new_prefix)?;
                }
                Ok(())
            }
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
    CallChain(Node<CallChainExpression>),
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // We can't implement decent Display easily without more effort,
        // but for LTL Predicates printing we might need it.
        // For now let's use Debug-like print or placeholder.
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CallChainExpression {
    pub base: Box<Node<Expression>>,
    pub segments: Vec<CallChainSegment>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum CallChainSegment {
    Call {
        name: Node<Identifier>,
        args: Node<Expression>,
    },
    Reaches {
        label: Node<Identifier>,
    },
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
    Reaches(LocalReachesNode),
    CallChain(LocalCallChainNode),
    IfExpr(LocalIfExprNode),
    ForAll(LocalForAllNode),
    Exists(LocalExistsNode),
}

#[derive(Debug, PartialEq, Clone)]
pub struct LocalReachesNode {
    pub var: LocalVarNode,
    pub index: Option<Box<LocalExpressionNode>>,
    pub label: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LocalCallChainNode {
    pub base: Box<LocalExpressionNode>,
    pub segments: Vec<LocalCallChainSegment>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum LocalCallChainSegment {
    Call {
        name: String,
        args: Box<LocalExpressionNode>,
    },
    Reaches {
        label: String,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub struct LocalIfExprNode {
    pub condition: Box<LocalExpressionNode>,
    pub then_expr: Box<LocalExpressionNode>,
    pub else_expr: Option<Box<LocalExpressionNode>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LocalForAllNode {
    pub var_name: String,
    pub list: Box<LocalExpressionNode>,
    pub body: Box<LocalExpressionNode>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LocalExistsNode {
    pub var_name: String,
    pub list: Box<LocalExpressionNode>,
    pub body: Box<LocalExpressionNode>,
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
            Self::Reaches(node) => {
                if node.index.is_some() {
                    write!(f, "[{}].at(...).reaches({})", node.var.index, node.label)
                } else {
                    write!(f, "[{}].reaches({})", node.var.index, node.label)
                }
            }
            Self::CallChain(_) => write!(f, "<call_chain>"),
            Self::IfExpr(_) => write!(f, "<if_expr>"),
            Self::ForAll(_) => write!(f, "<forall_expr>"),
            Self::Exists(_) => write!(f, "<exists_expr>"),
        }
    }
}

fn build_postfix_expression(pair: Pair<Rule>, filepath: &str) -> AlthreadResult<Node<Expression>> {
    let pos = Pos {
        line: pair.line_col().0,
        col: pair.line_col().1,
        start: pair.as_span().start(),
        end: pair.as_span().end(),
        file_path: filepath.to_string(),
    };

    let mut inner = pair.into_inner();
    let base_primary_pair = inner.next().unwrap();
    let remaining: Vec<Pair<Rule>> = inner.collect();

    let mut segments = Vec::new();
    let base_expr = match base_primary_pair.as_rule() {
        Rule::fn_call => {
            let call_node: Node<FnCall> = Node::build(base_primary_pair, filepath)?;
            let parts = call_node.value.fn_name.value.parts.clone();

            if parts.len() > 1 && !remaining.is_empty() {
                let base_parts = parts[..parts.len() - 1].to_vec();
                let base_ident = Node {
                    pos: call_node.value.fn_name.pos.clone(),
                    value: crate::ast::token::object_identifier::ObjectIdentifier {
                        parts: base_parts,
                    },
                };
                let base_primary = Node {
                    pos: base_ident.pos.clone(),
                    value: PrimaryExpression::Identifier(base_ident),
                };
                let base_expr = Node {
                    pos: base_primary.pos.clone(),
                    value: Expression::Primary(base_primary),
                };

                let name = parts.last().unwrap().clone();
                let args = (*call_node.value.values).clone();
                segments.push(CallChainSegment::Call { name, args });
                base_expr
            } else {
                Node {
                    pos: call_node.pos.clone(),
                    value: Expression::FnCall(call_node),
                }
            }
        }
        _ => {
            let base_primary = PrimaryExpression::build(base_primary_pair, filepath)?;
            Node {
                pos: base_primary.pos.clone(),
                value: Expression::Primary(base_primary),
            }
        }
    };

    for segment in remaining {
        let seg = if segment.as_rule() == Rule::postfix_segment {
            segment.into_inner().next().unwrap()
        } else {
            segment
        };

        match seg.as_rule() {
            Rule::postfix_call => {
                let mut call_inner = seg.into_inner();
                let name_pair = call_inner.next().unwrap();
                let name = Node {
                    pos: Pos {
                        line: name_pair.line_col().0,
                        col: name_pair.line_col().1,
                        start: name_pair.as_span().start(),
                        end: name_pair.as_span().end(),
                        file_path: filepath.to_string(),
                    },
                    value: Identifier {
                        value: name_pair.as_str().to_string(),
                    },
                };
                let args = Expression::build_top_level(call_inner.next().unwrap(), filepath)?;
                segments.push(CallChainSegment::Call { name, args });
            }
            Rule::postfix_reaches => {
                let mut reach_inner = seg.into_inner();
                let label_pair = reach_inner
                    .find(|p| p.as_rule() == Rule::identifier)
                    .unwrap();
                let label = Node {
                    pos: Pos {
                        line: label_pair.line_col().0,
                        col: label_pair.line_col().1,
                        start: label_pair.as_span().start(),
                        end: label_pair.as_span().end(),
                        file_path: filepath.to_string(),
                    },
                    value: Identifier {
                        value: label_pair.as_str().to_string(),
                    },
                };
                segments.push(CallChainSegment::Reaches { label });
            }
            _ => return Err(no_rule!(seg, "postfix_segment", filepath)),
        }
    }

    if segments.is_empty() {
        Ok(base_expr)
    } else {
        Ok(Node {
            pos: pos.clone(),
            value: Expression::CallChain(Node {
                pos,
                value: CallChainExpression {
                    base: Box::new(base_expr),
                    segments,
                },
            }),
        })
    }
}

pub fn parse_expr(pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Node<Expression>> {
    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::fn_call => Ok(Node {
                pos: Pos {
                    line: primary.line_col().0,
                    col: primary.line_col().1,
                    start: primary.as_span().start(),
                    end: primary.as_span().end(),
                    file_path: filepath.to_string(),
                },
                value: Expression::FnCall(Node::build(primary, filepath)?),
            }),
            Rule::postfix_expression => build_postfix_expression(primary, filepath),
            _ => Ok(Node {
                pos: Pos {
                    line: primary.line_col().0,
                    col: primary.line_col().1,
                    start: primary.as_span().start(),
                    end: primary.as_span().end(),
                    file_path: filepath.to_string(),
                },
                value: Expression::Primary(PrimaryExpression::build(primary, filepath)?),
            }),
        })
        .map_infix(|left, op, right| {
            Ok(Node {
                pos: Pos {
                    line: op.line_col().0,
                    col: op.line_col().1,
                    start: op.as_span().start(),
                    end: op.as_span().end(),
                    file_path: filepath.to_string(),
                },
                value: Expression::Binary(BinaryExpression::build(left?, op, right?, filepath)?),
            })
        })
        .map_prefix(|op, right| {
            Ok(Node {
                pos: Pos {
                    line: op.line_col().0,
                    col: op.line_col().1,
                    start: op.as_span().start(),
                    end: op.as_span().end(),
                    file_path: filepath.to_string(),
                },
                value: Expression::Unary(UnaryExpression::build(op, right?, filepath)?),
            })
        })
        .parse(pairs)
}

impl NodeBuilder for Expression {
    fn build(pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        parse_expr(pairs, filepath).map(|node| node.value)
    }
}
impl Expression {
    pub fn build_list_expression(pair: Pair<Rule>, filepath: &str) -> AlthreadResult<Node<Self>> {
        let pos = Pos {
            line: pair.line_col().0,
            col: pair.line_col().1,
            start: pair.as_span().start(),
            end: pair.as_span().end(),
            file_path: filepath.to_string(),
        };
        match pair.as_rule() {
            Rule::range_expression => {
                let mut pair = pair.into_inner();
                let expression_start: Box<Node<Expression>> =
                    Box::new(Node::build(pair.next().unwrap(), filepath)?);
                let expression_end: Box<Node<Expression>> =
                    Box::new(Node::build(pair.next().unwrap(), filepath)?);
                Ok(Node {
                    pos: pos.clone(),
                    value: Expression::Range(Node {
                        pos: pos,
                        value: RangeListExpression {
                            expression_start,
                            expression_end,
                        },
                    }),
                })
            }
            _ => Err(no_rule!(pair, "list_expression", filepath)),
        }
    }
    pub fn build_top_level(pair: Pair<Rule>, filepath: &str) -> AlthreadResult<Node<Self>> {
        let pos = Pos {
            line: pair.line_col().0,
            col: pair.line_col().1,
            start: pair.as_span().start(),
            end: pair.as_span().end(),
            file_path: filepath.to_string(),
        };
        match pair.as_rule() {
            Rule::expression => {
                let expr = Self::build(pair.into_inner(), filepath)?;
                Ok(Node { pos, value: expr })
            }
            Rule::tuple_expression => {
                let mut values = Vec::new();
                for pair in pair.into_inner() {
                    values.push(Node::build(pair, filepath)?);
                }
                Ok(Node {
                    pos: pos.clone(),
                    value: Expression::Tuple(Node {
                        pos,
                        value: TupleExpression { values },
                    }),
                })
            }
            _ => Err(no_rule!(pair, "Expression::build_top_level", filepath)),
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
            Expression::Primary(node) => match &node.value {
                PrimaryExpression::Reaches(proc_ident, index_expr, label_ident) => {
                    let full_name = proc_ident
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
                            Some(proc_ident.pos.clone()),
                            format!("Variable '{}' not found", full_name),
                        ))?;
                    let local_index_expr = if let Some(expr) = index_expr {
                        Some(Box::new(LocalExpressionNode::from_expression(
                            &expr.value,
                            program_stack,
                        )?))
                    } else {
                        None
                    };
                    LocalExpressionNode::Reaches(LocalReachesNode {
                        var: LocalVarNode { index },
                        index: local_index_expr,
                        label: label_ident.value.value.clone(),
                    })
                }
                PrimaryExpression::IfExpr {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    let cond =
                        LocalExpressionNode::from_expression(&condition.value, program_stack)?;
                    let then_e =
                        LocalExpressionNode::from_expression(&then_expr.value, program_stack)?;
                    let else_e = if let Some(else_expr) = else_expr {
                        Some(Box::new(LocalExpressionNode::from_expression(
                            &else_expr.value,
                            program_stack,
                        )?))
                    } else {
                        None
                    };
                    LocalExpressionNode::IfExpr(LocalIfExprNode {
                        condition: Box::new(cond),
                        then_expr: Box::new(then_e),
                        else_expr: else_e,
                    })
                }
                PrimaryExpression::ForAllExpr { var, list, body } => {
                    let list_local =
                        LocalExpressionNode::from_expression(&list.value, program_stack)?;

                    let mut temp_stack = program_stack.clone();
                    temp_stack.push(Variable {
                        mutable: false,
                        name: var.value.value.clone(),
                        datatype: DataType::Void,
                        depth: 0,
                        declare_pos: Some(var.pos.clone()),
                    });
                    let body_local =
                        LocalExpressionNode::from_expression(&body.value, &temp_stack)?;

                    LocalExpressionNode::ForAll(LocalForAllNode {
                        var_name: var.value.value.clone(),
                        list: Box::new(list_local),
                        body: Box::new(body_local),
                    })
                }
                PrimaryExpression::ExistsExpr { var, list, body } => {
                    let list_local =
                        LocalExpressionNode::from_expression(&list.value, program_stack)?;

                    let mut temp_stack = program_stack.clone();
                    temp_stack.push(Variable {
                        mutable: false,
                        name: var.value.value.clone(),
                        datatype: DataType::Void,
                        depth: 0,
                        declare_pos: Some(var.pos.clone()),
                    });
                    let body_local =
                        LocalExpressionNode::from_expression(&body.value, &temp_stack)?;

                    LocalExpressionNode::Exists(LocalExistsNode {
                        var_name: var.value.value.clone(),
                        list: Box::new(list_local),
                        body: Box::new(body_local),
                    })
                }
                _ => LocalExpressionNode::Primary(LocalPrimaryExpressionNode::from_primary(
                    &node.value,
                    program_stack,
                )?),
            },
            Expression::FnCall(node) => LocalExpressionNode::FnCall(Box::new(node.clone())),
            Expression::Tuple(node) => LocalExpressionNode::Tuple(
                LocalTupleExpressionNode::from_tuple(&node.value, program_stack)?,
            ),
            Expression::Range(node) => LocalExpressionNode::Range(
                LocalRangeListExpressionNode::from_range(&node.value, program_stack)?,
            ),
            Expression::CallChain(node) => {
                let base =
                    LocalExpressionNode::from_expression(&node.value.base.value, program_stack)?;
                let mut segments = Vec::new();
                for segment in node.value.segments.iter() {
                    match segment {
                        CallChainSegment::Call { name, args } => {
                            let local_args =
                                LocalExpressionNode::from_expression(&args.value, program_stack)?;
                            segments.push(LocalCallChainSegment::Call {
                                name: name.value.value.clone(),
                                args: Box::new(local_args),
                            });
                        }
                        CallChainSegment::Reaches { label } => {
                            segments.push(LocalCallChainSegment::Reaches {
                                label: label.value.value.clone(),
                            });
                        }
                    }
                }
                LocalExpressionNode::CallChain(LocalCallChainNode {
                    base: Box::new(base),
                    segments,
                })
            }
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
            LocalExpressionNode::Reaches(_) => false,
            LocalExpressionNode::CallChain(n) => {
                let mut has_call = n.base.contains_fn_call();
                for seg in n.segments.iter() {
                    if let LocalCallChainSegment::Call { args, .. } = seg {
                        has_call |= args.contains_fn_call();
                    }
                }
                has_call
            }
            LocalExpressionNode::IfExpr(n) => {
                n.condition.contains_fn_call()
                    || n.then_expr.contains_fn_call()
                    || n.else_expr
                        .as_ref()
                        .map(|e| e.contains_fn_call())
                        .unwrap_or(false)
            }
            LocalExpressionNode::ForAll(n) => {
                n.list.contains_fn_call() || n.body.contains_fn_call()
            }
            LocalExpressionNode::Exists(n) => {
                n.list.contains_fn_call() || n.body.contains_fn_call()
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

                if state.user_functions().contains_key(&full_name)
                    || node.value.fn_name.value.parts.len() == 1
                {
                    let fn_name = if state.user_functions().contains_key(&full_name) {
                        &full_name
                    } else {
                        &node.value.fn_name.value.parts[0].value.value
                    };

                    if let Some(func_def) = state.user_functions().get(fn_name) {
                        Ok(func_def.return_type.clone())
                    } else {
                        Err(format!("Function {} not found", fn_name))
                    }
                } else {
                    // Method call
                    let receiver_name = &node.value.fn_name.value.parts[0].value.value;
                    let var = state
                        .program_stack
                        .iter()
                        .rev()
                        .find(|v| &v.name == receiver_name);
                    if let Some(var) = var {
                        let interfaces = state.stdlib().interfaces(&var.datatype);
                        if !interfaces.is_empty() {
                            let method_name =
                                &node.value.fn_name.value.parts.last().unwrap().value.value;
                            if let Some(method) = interfaces.iter().find(|m| &m.name == method_name)
                            {
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
            Self::Reaches(node) => {
                if !state.in_condition_block {
                    return Err(
                        "'reaches' is only available inside always/check blocks".to_string(),
                    );
                }
                let mem_len = state.program_stack.len();
                let var = state
                    .program_stack
                    .get(mem_len - 1 - node.var.index)
                    .ok_or("process variable index does not exist".to_string())?;

                let (program_name, require_index) = match &var.datatype {
                    DataType::Process(name) => (name.clone(), false),
                    DataType::List(inner) => match inner.as_ref() {
                        DataType::Process(name) => (name.clone(), true),
                        _ => {
                            return Err(
                                "'reaches' requires a proc(<Program>) or list(proc(<Program>))"
                                    .to_string(),
                            )
                        }
                    },
                    _ => {
                        return Err(
                            "'reaches' requires a proc(<Program>) or list(proc(<Program>))"
                                .to_string(),
                        )
                    }
                };

                if require_index && node.index.is_none() {
                    return Err("'reaches' on list(proc(..)) requires .at(index)".to_string());
                }
                if !require_index && node.index.is_some() {
                    return Err("'reaches' on proc(..) does not accept .at(index)".to_string());
                }

                if let Some(index_expr) = &node.index {
                    let index_type = index_expr.datatype(state)?;
                    if index_type != DataType::Integer {
                        return Err("'.at(index)' requires an int index".to_string());
                    }
                }

                let program_code = state
                    .programs_code()
                    .get(&program_name)
                    .ok_or(format!("Program '{}' not found", program_name))?;

                if !program_code.labels.contains_key(&node.label) {
                    return Err(format!(
                        "Label '{}' not found in program '{}'",
                        node.label, program_name
                    ));
                }

                Ok(DataType::Boolean)
            }
            Self::CallChain(node) => {
                let mut current_type = node.base.datatype(state)?;
                for (idx, segment) in node.segments.iter().enumerate() {
                    match segment {
                        LocalCallChainSegment::Call { name, args } => {
                            let interfaces = state.stdlib().interfaces(&current_type);
                            let method = interfaces.iter().find(|m| m.name == *name);
                            let method =
                                method.ok_or_else(|| format!("undefined function {}", name))?;

                            let args_type = args.datatype(state)?;
                            if let DataType::Tuple(arg_types) = args_type {
                                if method.args.len() != arg_types.len() {
                                    return Err(format!(
                                        "Method '{}' expects {} arguments, got {}",
                                        name,
                                        method.args.len(),
                                        arg_types.len()
                                    ));
                                }
                                for (expected, provided) in method.args.iter().zip(arg_types.iter())
                                {
                                    if expected != provided {
                                        return Err(format!(
                                            "Method '{}' expects argument of type {}, got {}",
                                            name, expected, provided
                                        ));
                                    }
                                }
                            } else {
                                return Err("Method call expects tuple arguments".to_string());
                            }

                            current_type = method.ret.clone();
                        }
                        LocalCallChainSegment::Reaches { label } => {
                            if idx + 1 != node.segments.len() {
                                return Err("'reaches' must be the last segment in a call chain"
                                    .to_string());
                            }
                            let program_name = match &current_type {
                                DataType::Process(name) => name.clone(),
                                _ => {
                                    return Err(
                                        "'reaches' requires a value of type proc(<Program>)"
                                            .to_string(),
                                    )
                                }
                            };

                            let program_code = state
                                .programs_code()
                                .get(&program_name)
                                .ok_or(format!("Program '{}' not found", program_name))?;

                            if !program_code.labels.contains_key(label) {
                                return Err(format!(
                                    "Label '{}' not found in program '{}'",
                                    label, program_name
                                ));
                            }

                            current_type = DataType::Boolean;
                        }
                    }
                }
                Ok(current_type)
            }
            Self::IfExpr(node) => {
                if !state.in_condition_block {
                    return Err(
                        "if-expressions are only supported inside always/check blocks"
                            .to_string(),
                    );
                }
                let cond_type = node.condition.datatype(state)?;
                if cond_type != DataType::Boolean {
                    return Err("if condition must be boolean".to_string());
                }
                let then_type = node.then_expr.datatype(state)?;
                
                if let Some(else_expr) = &node.else_expr {
                    let else_type = else_expr.datatype(state)?;
                    if then_type != else_type {
                        return Err("if branches must have the same type".to_string());
                    }
                    Ok(then_type)
                } else {
                    if then_type != DataType::Boolean {
                         return Err("if A { B } (without else) is an implication, so B must be boolean".to_string());
                    }
                    Ok(DataType::Boolean)
                }
            }
            Self::ForAll(node) => {
                if !state.in_condition_block {
                    return Err(
                        "forall is only supported inside always/check blocks".to_string()
                    );
                }
                let list_type = node.list.datatype(state)?;
                let elem_type = match list_type {
                    DataType::List(t) => *t,
                    _ => return Err("forall expects a list".to_string()),
                };

                let mut temp_stack = state.program_stack.clone();
                temp_stack.push(Variable {
                    name: node.var_name.clone(),
                    depth: state.current_stack_depth,
                    mutable: false,
                    datatype: elem_type,
                    declare_pos: None,
                });

                let temp_state = CompilerState {
                    program_stack: temp_stack,
                    current_stack_depth: state.current_stack_depth,
                    current_program_name: state.current_program_name.clone(),
                    is_atomic: state.is_atomic,
                    is_shared: state.is_shared,
                    in_function: state.in_function,
                    method_call_stack_offset: state.method_call_stack_offset,
                    in_condition_block: state.in_condition_block,
                    context: state.context.clone(),
                    always_conditions: state.always_conditions.clone(),
                    ltl_formulas: state.ltl_formulas.clone(),
                    user_functions: state.user_functions.clone(),
                    global_table: state.global_table.clone(),
                    program_arguments: state.program_arguments.clone(),
                    programs_code: state.programs_code.clone(),
                    global_memory: state.global_memory.clone(),
                };

                let body_type = node.body.datatype(&temp_state)?;
                if body_type != DataType::Boolean {
                    return Err("forall body must be boolean".to_string());
                }
                Ok(DataType::Boolean)
            }
            Self::Exists(node) => {
                if !state.in_condition_block {
                    return Err(
                        "exists is only supported inside always/check blocks".to_string()
                    );
                }
                let list_type = node.list.datatype(state)?;
                let elem_type = match list_type {
                    DataType::List(t) => *t,
                    _ => return Err("exists expects a list".to_string()),
                };

                let mut temp_stack = state.program_stack.clone();
                temp_stack.push(Variable {
                    name: node.var_name.clone(),
                    depth: state.current_stack_depth,
                    mutable: false,
                    datatype: elem_type,
                    declare_pos: None,
                });

                let temp_state = CompilerState {
                    program_stack: temp_stack,
                    current_stack_depth: state.current_stack_depth,
                    current_program_name: state.current_program_name.clone(),
                    is_atomic: state.is_atomic,
                    is_shared: state.is_shared,
                    in_function: state.in_function,
                    method_call_stack_offset: state.method_call_stack_offset,
                    in_condition_block: state.in_condition_block,
                    context: state.context.clone(),
                    always_conditions: state.always_conditions.clone(),
                    ltl_formulas: state.ltl_formulas.clone(),
                    user_functions: state.user_functions.clone(),
                    global_table: state.global_table.clone(),
                    program_arguments: state.program_arguments.clone(),
                    programs_code: state.programs_code.clone(),
                    global_memory: state.global_memory.clone(),
                };

                let body_type = node.body.datatype(&temp_state)?;
                if body_type != DataType::Boolean {
                    return Err("exists body must be boolean".to_string());
                }
                Ok(DataType::Boolean)
            }
        }
    }
    pub fn eval(&self, mem: &Memory) -> Result<Literal, String> {
        match self {
            LocalExpressionNode::Binary(binary_exp) => match binary_exp.operator {
                crate::ast::token::binary_operator::BinaryOperator::And => {
                    let left = binary_exp.left.eval(mem)?;
                    if !left.is_true() {
                        return Ok(Literal::Bool(false));
                    }
                    let right = binary_exp.right.eval(mem)?;
                    left.and(&right)
                }
                crate::ast::token::binary_operator::BinaryOperator::Or => {
                    let left = binary_exp.left.eval(mem)?;
                    if left.is_true() {
                        return Ok(Literal::Bool(true));
                    }
                    let right = binary_exp.right.eval(mem)?;
                    left.or(&right)
                }
                _ => binary_exp.eval(mem),
            },
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
            LocalExpressionNode::FnCall(node) => Err(format!(
                "Cannot evaluate function call in this context: {:?}",
                &node.value.fn_name
            )),
            LocalExpressionNode::Reaches(_) => {
                Err("'reaches' is only supported in always/check blocks".to_string())
            }
            LocalExpressionNode::CallChain(_) => {
                Err("call chains are only supported in always/check blocks".to_string())
            }
            LocalExpressionNode::IfExpr(_) => {
                Err("if-expressions are only supported in always/check blocks".to_string())
            }
            LocalExpressionNode::ForAll(_) => {
                Err("forall is only supported in always/check blocks".to_string())
            }
            LocalExpressionNode::Exists(_) => {
                Err("exists is only supported in always/check blocks".to_string())
            }
        }
    }

    pub fn eval_with_context(&self, mem: &Memory, vm: &crate::vm::VM) -> Result<Literal, String> {
        match self {
            LocalExpressionNode::Binary(binary_exp) => match binary_exp.operator {
                crate::ast::token::binary_operator::BinaryOperator::And => {
                    let left = binary_exp.left.eval_with_context(mem, vm)?;
                    if !left.is_true() {
                        return Ok(Literal::Bool(false));
                    }
                    let right = binary_exp.right.eval_with_context(mem, vm)?;
                    left.and(&right)
                }
                crate::ast::token::binary_operator::BinaryOperator::Or => {
                    let left = binary_exp.left.eval_with_context(mem, vm)?;
                    if left.is_true() {
                        return Ok(Literal::Bool(true));
                    }
                    let right = binary_exp.right.eval_with_context(mem, vm)?;
                    left.or(&right)
                }
                _ => {
                    let left = binary_exp.left.eval_with_context(mem, vm)?;
                    let right = binary_exp.right.eval_with_context(mem, vm)?;
                    match binary_exp.operator {
                        crate::ast::token::binary_operator::BinaryOperator::Add => left.add(&right),
                        crate::ast::token::binary_operator::BinaryOperator::Subtract => {
                            left.subtract(&right)
                        }
                        crate::ast::token::binary_operator::BinaryOperator::Multiply => {
                            left.multiply(&right)
                        }
                        crate::ast::token::binary_operator::BinaryOperator::Divide => {
                            left.divide(&right)
                        }
                        crate::ast::token::binary_operator::BinaryOperator::Modulo => {
                            left.modulo(&right)
                        }
                        crate::ast::token::binary_operator::BinaryOperator::Equals => {
                            left.equals(&right)
                        }
                        crate::ast::token::binary_operator::BinaryOperator::NotEquals => {
                            left.not_equals(&right)
                        }
                        crate::ast::token::binary_operator::BinaryOperator::LessThan => {
                            left.less_than(&right)
                        }
                        crate::ast::token::binary_operator::BinaryOperator::LessThanOrEqual => {
                            left.less_than_or_equal(&right)
                        }
                        crate::ast::token::binary_operator::BinaryOperator::GreaterThan => {
                            left.greater_than(&right)
                        }
                        crate::ast::token::binary_operator::BinaryOperator::GreaterThanOrEqual => {
                            left.greater_than_or_equal(&right)
                        }
                        _ => unreachable!("short-circuit handled above"),
                    }
                }
            },
            LocalExpressionNode::Unary(unary_exp) => {
                let operand = unary_exp.operand.eval_with_context(mem, vm)?;
                match unary_exp.operator {
                    crate::ast::token::unary_operator::UnaryOperator::Positive => {
                        operand.positive()
                    }
                    crate::ast::token::unary_operator::UnaryOperator::Negative => {
                        operand.negative()
                    }
                    crate::ast::token::unary_operator::UnaryOperator::Not => operand.not(),
                }
            }
            LocalExpressionNode::Primary(primary_exp) => match primary_exp {
                LocalPrimaryExpressionNode::Literal(literal) => Ok(literal.value.clone()),
                LocalPrimaryExpressionNode::Var(local_var) => {
                    let lit = mem
                        .get(mem.len() - 1 - local_var.index)
                        .ok_or("local variable index does not exist in memory".to_string())?;
                    Ok(lit.clone())
                }
                LocalPrimaryExpressionNode::Expression(expr) => {
                    expr.as_ref().eval_with_context(mem, vm)
                }
            },
            LocalExpressionNode::Tuple(tuple_exp) => Ok(Literal::Tuple(
                tuple_exp
                    .values
                    .iter()
                    .map(|v| v.eval_with_context(mem, vm))
                    .collect::<Result<Vec<Literal>, String>>()?,
            )),
            LocalExpressionNode::Range(list_exp) => {
                let start = list_exp.expression_start.eval_with_context(mem, vm)?;
                let end = list_exp.expression_end.eval_with_context(mem, vm)?;
                Ok(Literal::List(
                    DataType::Integer,
                    (start.to_integer()?..end.to_integer()?)
                        .map(|v| Literal::Int(v))
                        .collect(),
                ))
            }
            LocalExpressionNode::FnCall(node) => Err(format!(
                "Cannot evaluate function call in this context: {:?}",
                &node.value.fn_name
            )),
            LocalExpressionNode::Reaches(node) => {
                let lit = mem
                    .get(mem.len() - 1 - node.var.index)
                    .ok_or("process variable index does not exist in memory".to_string())?;
                let (program_name, pid) = match (lit, node.index.as_ref()) {
                    (Literal::Process(name, pid), None) => (name.clone(), *pid),
                    (Literal::List(DataType::Process(_name), values), Some(index_expr)) => {
                        let idx_lit = index_expr.eval_with_context(mem, vm)?;
                        let idx = idx_lit.to_integer()? as usize;
                        let proc_lit = match values.get(idx) {
                            Some(v) => v,
                            None => return Ok(Literal::Bool(false)),
                        };
                        match proc_lit {
                            Literal::Process(pname, pid) => (pname.clone(), *pid),
                            _ => return Ok(Literal::Bool(false)),
                        }
                    }
                    _ => return Ok(Literal::Bool(false)),
                };

                let prog_state = match vm.running_programs.get(pid) {
                    Some(p) => p,
                    None => {
                        if node.label == "end" {
                            return Ok(Literal::Bool(true));
                        }
                        return Ok(Literal::Bool(false));
                    }
                };

                if prog_state.name != program_name {
                    return Err("process name mismatch in current state".to_string());
                }

                let program_code = vm
                    .programs_code
                    .get(&program_name)
                    .ok_or("program code not found".to_string())?;

                let label_pc = program_code.labels.get(&node.label).ok_or(format!(
                    "Label '{}' not found in program '{}'",
                    node.label, program_name
                ))?;

                let (_, ip, _) = prog_state.current_state();
                let reached = ip == *label_pc;
                Ok(Literal::Bool(reached))
            }
            LocalExpressionNode::CallChain(node) => {
                let mut current = node.base.eval_with_context(mem, vm)?;
                for segment in node.segments.iter() {
                    match segment {
                        LocalCallChainSegment::Call { name, args } => {
                            let mut interfaces = vm.stdlib.interfaces(&current.get_datatype());
                            let method = interfaces
                                .iter_mut()
                                .find(|m| m.name == *name)
                                .ok_or(format!("undefined function {}", name))?;
                            let mut args_value = args.eval_with_context(mem, vm)?;

                            if name == "at" {
                                if let (Literal::List(_, values), Literal::Tuple(arg_vals)) =
                                    (&current, &args_value)
                                {
                                    if let Some(idx_lit) = arg_vals.first() {
                                        let idx = idx_lit.to_integer()? as isize;
                                        if idx < 0 || (idx as usize) >= values.len() {
                                            current = Literal::Null;
                                            continue;
                                        }
                                    }
                                }
                            }

                            current = (method.f.as_ref())(&mut current, &mut args_value, None)
                                .map_err(|e| e.message)?;
                        }
                        LocalCallChainSegment::Reaches { label } => {
                            let (program_name, pid) = match &current {
                                Literal::Process(name, pid) => (name.clone(), *pid),
                                _ => {
                                    current = Literal::Bool(false);
                                    continue;
                                }
                            };

                            let prog_state = match vm.running_programs.get(pid) {
                                Some(p) => p,
                                None => {
                                    if label == "end" {
                                        current = Literal::Bool(true);
                                    } else {
                                        current = Literal::Bool(false);
                                    }
                                    continue;
                                }
                            };

                            if prog_state.name != program_name {
                                current = Literal::Bool(false);
                                continue;
                            }

                            let program_code = match vm.programs_code.get(&program_name) {
                                Some(code) => code,
                                None => {
                                    current = Literal::Bool(false);
                                    continue;
                                }
                            };

                            let label_pc = match program_code.labels.get(label) {
                                Some(pc) => pc,
                                None => {
                                    current = Literal::Bool(false);
                                    continue;
                                }
                            };

                            let (_, ip, _) = prog_state.current_state();
                            current = Literal::Bool(ip == *label_pc);
                        }
                    }
                }
                Ok(current)
            }
            LocalExpressionNode::IfExpr(node) => {
                let cond = node.condition.eval_with_context(mem, vm)?;
                if cond.is_true() {
                    node.then_expr.eval_with_context(mem, vm)
                } else {
                    if let Some(else_expr) = &node.else_expr {
                        else_expr.eval_with_context(mem, vm)
                    } else {
                        // if A { B } means A -> B. If A is false, result is true.
                        Ok(Literal::Bool(true))
                    }
                }
            }
            LocalExpressionNode::ForAll(node) => {
                let list = node.list.eval_with_context(mem, vm)?;
                let values = match list {
                    Literal::List(_, values) => values,
                    _ => return Err("forall expects a list".to_string()),
                };

                for value in values.into_iter() {
                    let mut temp_mem = mem.clone();
                    temp_mem.push(value);
                    let body_value = node.body.eval_with_context(&temp_mem, vm)?;
                    if !body_value.is_true() {
                        return Ok(Literal::Bool(false));
                    }
                }
                Ok(Literal::Bool(true))
            }
            LocalExpressionNode::Exists(node) => {
                let list = node.list.eval_with_context(mem, vm)?;
                let values = match list {
                    Literal::List(_, values) => values,
                    _ => return Err("exists expects a list".to_string()),
                };

                for value in values.into_iter() {
                    let mut temp_mem = mem.clone();
                    temp_mem.push(value);
                    let body_value = node.body.eval_with_context(&temp_mem, vm)?;
                    if body_value.is_true() {
                        return Ok(Literal::Bool(true));
                    }
                }
                Ok(Literal::Bool(false))
            }
        }
    }
}

impl CallChainExpression {
    fn compile_chain(
        &self,
        state: &mut CompilerState,
        pos: &Pos,
    ) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();

        let base_builder = self.base.compile(state)?;
        builder.extend(base_builder);

        for segment in &self.segments {
            match segment {
                CallChainSegment::Call { name, args } => {
                    let args_builder = args.compile(state)?;
                    builder.extend(args_builder);

                    if state.program_stack.len() < 2 {
                        return Err(AlthreadError::new(
                            ErrorType::ExpressionError,
                            Some(pos.clone()),
                            "Invalid call chain state".to_string(),
                        ));
                    }

                    let receiver_var = state
                        .program_stack
                        .get(state.program_stack.len() - 2)
                        .unwrap();
                    let interfaces = state.stdlib().interfaces(&receiver_var.datatype);
                    let method = interfaces.iter().find(|m| m.name == name.value.value);
                    let method = method.ok_or(AlthreadError::new(
                        ErrorType::UndefinedFunction,
                        Some(pos.clone()),
                        format!("undefined function {}", name.value.value),
                    ))?;

                    let args_var = state.program_stack.last().unwrap();
                    if let DataType::Tuple(arg_types) = &args_var.datatype {
                        if method.args.len() != arg_types.len() {
                            return Err(AlthreadError::new(
                                ErrorType::FunctionArgumentCountError,
                                Some(pos.clone()),
                                format!(
                                    "Method '{}' expects {} arguments, got {}",
                                    name.value.value,
                                    method.args.len(),
                                    arg_types.len()
                                ),
                            ));
                        }
                        for (expected, provided) in method.args.iter().zip(arg_types.iter()) {
                            if expected != provided {
                                return Err(AlthreadError::new(
                                    ErrorType::FunctionArgumentTypeMismatch,
                                    Some(pos.clone()),
                                    format!(
                                        "Method '{}' expects argument of type {}, got {}",
                                        name.value.value, expected, provided
                                    ),
                                ));
                            }
                        }
                    } else {
                        return Err(AlthreadError::new(
                            ErrorType::FunctionArgumentTypeMismatch,
                            Some(pos.clone()),
                            "Method call expects tuple arguments".to_string(),
                        ));
                    }

                    builder.instructions.push(Instruction {
                        pos: Some(pos.clone()),
                        control: InstructionType::MethodCall {
                            name: name.value.value.clone(),
                            receiver_idx: 1,
                            unstack_len: 1,
                            drop_receiver: true,
                            arguments: None,
                        },
                    });

                    let _args = state.program_stack.pop();
                    let _receiver = state.program_stack.pop();

                    state.program_stack.push(Variable {
                        name: "".to_string(),
                        depth: state.current_stack_depth,
                        mutable: false,
                        datatype: method.ret.clone(),
                        declare_pos: Some(pos.clone()),
                    });
                }
                CallChainSegment::Reaches { .. } => {
                    return Err(AlthreadError::new(
                        ErrorType::InstructionNotAllowed,
                        Some(pos.clone()),
                        "'reaches' is only allowed inside always/check blocks".to_string(),
                    ));
                }
            }
        }

        Ok(builder)
    }
}

// we build directly the traits on the node
// because we need line/column information
impl InstructionBuilder for Node<Expression> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        if let Expression::CallChain(node) = &self.value {
            if !state.in_condition_block {
                return node.value.compile_chain(state, &self.pos);
            }
        }
        let mut instructions = Vec::new();
        let mut vars = HashSet::new();
        self.value.get_vars(&mut vars);

        if !state.in_condition_block
            && vars
                .iter()
                .any(|var| var.starts_with("GS.procs.") || var.starts_with("$.procs."))
        {
            return Err(AlthreadError::new(
                ErrorType::InstructionNotAllowed,
                Some(self.pos.clone()),
                "$.procs.* is only available inside always/check blocks"
                    .to_string(),
            ));
        }

        vars.retain(|var| state.global_table().contains_key(var));

        if !vars.is_empty() {
            let mut ordered_vars: Vec<String> = vars.iter().cloned().collect();
            ordered_vars.sort();
            for var in ordered_vars.iter() {
                let global_var = state.global_table().get(var).cloned().expect(&format!(
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
                pos: Some(self.pos.clone()),
                control: InstructionType::GlobalReads {
                    only_const: ordered_vars
                        .iter()
                        .all(|v| state.global_table()[v].mutable == false),
                    variables: ordered_vars,
                },
            });
        }

        let local_expr = LocalExpressionNode::from_expression(&self.value, &state.program_stack)?;

        let result_type = local_expr.datatype(state).map_err(|err| {
            AlthreadError::new(
                ErrorType::ExpressionError,
                Some(self.pos.clone()),
                format!("Type of expression is not well-defined: {}", err),
            )
        })?;

        if !local_expr.contains_fn_call() {
            instructions.push(Instruction {
                pos: Some(self.pos.clone()),
                control: InstructionType::Expression(local_expr),
            });
        } else {
            fn shift_var_indices(expr: &LocalExpressionNode, shift: usize) -> LocalExpressionNode {
                match expr {
                    LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Var(var)) => {
                        LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Var(
                            LocalVarNode {
                                index: var.index + shift,
                            },
                        ))
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
                    LocalExpressionNode::Tuple(node) => {
                        LocalExpressionNode::Tuple(LocalTupleExpressionNode {
                            values: node
                                .values
                                .iter()
                                .map(|v| shift_var_indices(v, shift))
                                .collect(),
                        })
                    }
                    LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(expr)) => {
                        LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(
                            Box::new(shift_var_indices(expr, shift)),
                        ))
                    }
                    LocalExpressionNode::Range(node) => {
                        LocalExpressionNode::Range(LocalRangeListExpressionNode {
                            expression_start: Box::new(shift_var_indices(
                                &node.expression_start,
                                shift,
                            )),
                            expression_end: Box::new(shift_var_indices(
                                &node.expression_end,
                                shift,
                            )),
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
                    LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(expr)) => {
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

                        let new_expr = LocalExpressionNode::Range(LocalRangeListExpressionNode {
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
                        pos: Some(self.pos.clone()),
                        control: InstructionType::MakeTupleAndCleanup {
                            elements: if let LocalExpressionNode::Tuple(t) = final_expr {
                                t.values
                            } else {
                                vec![]
                            },
                            unstack_len: fn_call_count,
                        },
                    });
                } else {
                    instructions.push(Instruction {
                        pos: Some(self.pos.clone()),
                        control: InstructionType::ExpressionAndCleanup {
                            expression: final_expr,
                            unstack_len: fn_call_count,
                        },
                    });
                }
            } else {
                instructions.push(Instruction {
                    pos: Some(self.pos.clone()),
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
            Self::CallChain(node) => {
                node.value.base.value.add_dependencies(dependencies);
                for segment in node.value.segments.iter() {
                    if let CallChainSegment::Call { args, .. } = segment {
                        args.value.add_dependencies(dependencies);
                    }
                }
            }
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
            Self::CallChain(node) => {
                node.value.base.value.get_vars(vars);
                for segment in node.value.segments.iter() {
                    if let CallChainSegment::Call { args, .. } = segment {
                        args.value.get_vars(vars);
                    }
                }
            }
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
            Self::CallChain(node) => {
                writeln!(f, "{prefix}call_chain")?;
                node.ast_fmt(f, &prefix.add_branch())
            }
        }
    }
}

impl AstDisplay for CallChainExpression {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}base")?;
        self.base.ast_fmt(f, &prefix.add_branch())?;

        let mut seg_prefix = prefix.add_branch();
        for segment in &self.segments {
            match segment {
                CallChainSegment::Call { name, .. } => {
                    writeln!(f, "{}call: {}", seg_prefix, name.value.value)?;
                }
                CallChainSegment::Reaches { label } => {
                    writeln!(f, "{}reaches: {}", seg_prefix, label.value.value)?;
                }
            }
            seg_prefix = seg_prefix.switch();
        }

        Ok(())
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
                file_path: "test".to_string(),
            },
            value: Literal::Int(42),
        };
        let primary_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
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
                file_path: "test".to_string(),
            },
            value: Literal::Int(42),
        };
        let primary_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
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
                file_path: "test".to_string(),
            },
            value: BinaryExpression {
                left: Box::new(Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                        file_path: "test".to_string(),
                    },
                    value: litteral_expr.clone(),
                }),
                right: Box::new(Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                        file_path: "test".to_string(),
                    },
                    value: litteral_expr.clone(),
                }),
                operator: Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                        file_path: "test".to_string(),
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
