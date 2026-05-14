pub mod binary_expression;
pub mod list_expression;
pub mod primary_expression;
pub mod tuple_expression;
pub mod unary_expression;

use std::{collections::HashSet, fmt};

use binary_expression::{BinaryExpression, LocalBinaryExpressionNode};
use list_expression::{LocalRangeListExpressionNode, RangeListExpression};
use primary_expression::{
    LocalLiteralNode, LocalPrimaryExpressionNode, LocalVarNode, PrimaryExpression,
};
use tuple_expression::{LocalTupleExpressionNode, TupleExpression};
use unary_expression::{LocalUnaryExpressionNode, UnaryExpression};

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node},
        token::{
            datatype::DataType, identifier::Identifier, literal::Literal,
            object_identifier::ObjectIdentifier,
        },
    },
    compiler::{
        stdlib::{invoke_interface_method, resolve_interface_method, validate_interface_call},
        CompilerState, InstructionBuilderOk, Variable,
    },
    error::{AlthreadError, AlthreadResult, ErrorType, Pos},
    vm::{
        instruction::{Instruction, InstructionType},
        Memory,
    },
};

use super::{fn_call::FnCall, run_call::RunCall, waiting_case::WaitDependency};

#[derive(Debug, PartialEq, Clone)]
pub struct BracketExpression {
    pub content: BracketContent,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BracketContent {
    Range(Node<RangeListExpression>),
    ListLiteral(Vec<Node<Expression>>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    Binary(Node<BinaryExpression>),
    Unary(Node<UnaryExpression>),
    Primary(Node<PrimaryExpression>),
    Tuple(Node<TupleExpression>),
    Range(Node<RangeListExpression>),
    FnCall(Node<FnCall>),
    RunCall(Box<Node<RunCall>>),
    Bracket(Node<BracketExpression>),
    CallChain(Node<CallChainExpression>),
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    Invoke {
        args: Node<Expression>,
    },
    Field {
        name: Node<Identifier>,
    },
    TupleIndex {
        index: usize,
    },
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
    TupleIndex(LocalTupleIndexNode),
    FnCall(Box<Node<FnCall>>),
    RunCall(Box<Node<RunCall>>),
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
    pub base: LocalCallChainBase,
    pub segments: Vec<LocalCallChainSegment>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum LocalCallChainBase {
    Expr(Box<LocalExpressionNode>),
    Name(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct LocalTupleIndexNode {
    pub base: Box<LocalExpressionNode>,
    pub index: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum LocalCallChainSegment {
    Invoke {
        args: Box<LocalExpressionNode>,
    },
    Field {
        name: String,
    },
    TupleIndex {
        index: usize,
    },
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

impl InstructionBuilder for BracketExpression {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        match &self.content {
            BracketContent::Range(range_node) => {
                let range_expr = Node {
                    pos: range_node.pos.clone(),
                    value: Expression::Range(range_node.clone()),
                };
                range_expr.compile(state)
            }
            BracketContent::ListLiteral(expressions) => {
                let mut instructions = Vec::new();

                let element_type = if let Some(first_expr) = expressions.first() {
                    match &first_expr.value {
                        Expression::RunCall(_) => {
                            return Err(AlthreadError::new(
                                ErrorType::ExpressionError,
                                Some(first_expr.pos.clone()),
                                "Run calls cannot be used in list literals".to_string(),
                            ));
                        }
                        value => {
                            let local_expr =
                                LocalExpressionNode::from_expression(value, &state.program_stack)?;
                            local_expr.datatype(state).map_err(|err| {
                                AlthreadError::new(
                                    ErrorType::ExpressionError,
                                    Some(first_expr.pos.clone()),
                                    format!("Cannot infer type of list element: {}", err),
                                )
                            })?
                        }
                    }
                } else {
                    DataType::Void
                };

                for (i, expr) in expressions.iter().enumerate() {
                    if matches!(expr.value, Expression::RunCall(_)) {
                        return Err(AlthreadError::new(
                            ErrorType::ExpressionError,
                            Some(expr.pos.clone()),
                            "Run calls cannot be used in list literals".to_string(),
                        ));
                    }

                    let builder = expr.compile(state)?;
                    instructions.extend(builder.instructions);

                    if element_type != DataType::Void {
                        let local_expr = LocalExpressionNode::from_expression(
                            &expr.value,
                            &state.program_stack,
                        )?;
                        let expr_type = local_expr.datatype(state).map_err(|err| {
                            AlthreadError::new(
                                ErrorType::ExpressionError,
                                Some(expr.pos.clone()),
                                format!("Cannot determine type of list element {}: {}", i, err),
                            )
                        })?;

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

                instructions.push(Instruction {
                    pos: None,
                    control: InstructionType::CreateListFromStack {
                        element_count: expressions.len(),
                        element_type: element_type.clone(),
                    },
                });

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
            Self::TupleIndex(node) => write!(f, "{}.{}", node.base, node.index),
            Self::FnCall(node) => write!(f, "{:?}", node),
            Self::RunCall(node) => write!(f, "{:?}", node),
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

fn tuple_arg_types(datatype: &DataType) -> Result<&[DataType], String> {
    match datatype {
        DataType::Tuple(types) => Ok(types),
        _ => Err("Method call expects tuple arguments".to_string()),
    }
}

fn temp_fn_call_node(name: &str, values: Node<Expression>, pos: &Pos) -> Node<FnCall> {
    Node {
        pos: pos.clone(),
        value: FnCall {
            fn_name: Node {
                pos: pos.clone(),
                value: ObjectIdentifier {
                    parts: name
                        .split('.')
                        .map(|part| Node {
                            pos: pos.clone(),
                            value: Identifier {
                                value: part.to_string(),
                            },
                        })
                        .collect(),
                },
            },
            values: Box::new(values),
        },
    }
}

fn direct_function_return_type(
    function_name: &str,
    args: &LocalExpressionNode,
    state: &CompilerState,
) -> Result<DataType, String> {
    if let Some(func_def) = state.user_functions().get(function_name) {
        let provided_arg_types = args.datatype(state)?;
        let provided_arg_types = tuple_arg_types(&provided_arg_types)?;

        if func_def.arguments.len() != provided_arg_types.len() {
            return Err(format!(
                "Function '{}' expects {} arguments, but {} were provided.",
                function_name,
                func_def.arguments.len(),
                provided_arg_types.len()
            ));
        }

        for ((_, expected_type), provided_type) in
            func_def.arguments.iter().zip(provided_arg_types.iter())
        {
            if expected_type != provided_type {
                return Err(format!(
                    "Function '{}' expects argument of type {}, but got {}.",
                    function_name, expected_type, provided_type
                ));
            }
        }

        return Ok(func_def.return_type.clone());
    }

    match function_name {
        "print" => Ok(DataType::Void),
        "assert" => Ok(DataType::Void),
        _ => Err(format!("Function {} not found", function_name)),
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
            Expression::RunCall(node) => LocalExpressionNode::RunCall(node.clone()),
            Expression::Tuple(node) => LocalExpressionNode::Tuple(
                LocalTupleExpressionNode::from_tuple(&node.value, program_stack)?,
            ),
            Expression::Range(node) => LocalExpressionNode::Range(
                LocalRangeListExpressionNode::from_range(&node.value, program_stack)?,
            ),
            Expression::Bracket(node) => match &node.value.content {
                BracketContent::Range(range) => LocalExpressionNode::Range(
                    LocalRangeListExpressionNode::from_range(&range.value, program_stack)?,
                ),
                BracketContent::ListLiteral(values) => {
                    let local_values = values
                        .iter()
                        .map(|value| {
                            LocalExpressionNode::from_expression(&value.value, program_stack)
                        })
                        .collect::<AlthreadResult<Vec<_>>>()?;
                    LocalExpressionNode::Tuple(LocalTupleExpressionNode {
                        values: local_values,
                    })
                }
            },
            Expression::CallChain(node) => {
                let base = match &node.value.base.value {
                    Expression::Primary(primary_node) => match &primary_node.value {
                        PrimaryExpression::Identifier(identifier)
                            if matches!(
                                node.value.segments.first(),
                                Some(CallChainSegment::Invoke { .. })
                            ) =>
                        {
                            let full_name = identifier
                                .value
                                .parts
                                .iter()
                                .map(|p| p.value.value.as_str())
                                .collect::<Vec<_>>()
                                .join(".");
                            LocalCallChainBase::Name(full_name)
                        }
                        _ => LocalCallChainBase::Expr(Box::new(
                            LocalExpressionNode::from_expression(
                                &node.value.base.value,
                                program_stack,
                            )?,
                        )),
                    },
                    _ => LocalCallChainBase::Expr(Box::new(LocalExpressionNode::from_expression(
                        &node.value.base.value,
                        program_stack,
                    )?)),
                };
                let mut segments = Vec::new();
                for segment in node.value.segments.iter() {
                    match segment {
                        CallChainSegment::Invoke { args } => {
                            let local_args =
                                LocalExpressionNode::from_expression(&args.value, program_stack)?;
                            segments.push(LocalCallChainSegment::Invoke {
                                args: Box::new(local_args),
                            });
                        }
                        CallChainSegment::Call { name, args } => {
                            let local_args =
                                LocalExpressionNode::from_expression(&args.value, program_stack)?;
                            segments.push(LocalCallChainSegment::Call {
                                name: name.value.value.clone(),
                                args: Box::new(local_args),
                            });
                        }
                        CallChainSegment::Field { name } => {
                            segments.push(LocalCallChainSegment::Field {
                                name: name.value.value.clone(),
                            });
                        }
                        CallChainSegment::TupleIndex { index } => {
                            segments.push(LocalCallChainSegment::TupleIndex { index: *index });
                        }
                        CallChainSegment::Reaches { label } => {
                            segments.push(LocalCallChainSegment::Reaches {
                                label: label.value.value.clone(),
                            });
                        }
                    }
                }
                LocalExpressionNode::CallChain(LocalCallChainNode { base, segments })
            }
        };
        Ok(root)
    }

    pub fn contains_call(&self) -> bool {
        match self {
            LocalExpressionNode::FnCall(_) | LocalExpressionNode::RunCall(_) => true,
            LocalExpressionNode::Binary(n) => n.left.contains_call() || n.right.contains_call(),
            LocalExpressionNode::Unary(n) => n.operand.contains_call(),
            LocalExpressionNode::Primary(n) => match n {
                LocalPrimaryExpressionNode::Expression(e) => e.contains_call(),
                _ => false,
            },
            LocalExpressionNode::Tuple(n) => n.values.iter().any(|e| e.contains_call()),
            LocalExpressionNode::Range(n) => {
                n.expression_start.contains_call() || n.expression_end.contains_call()
            }
            LocalExpressionNode::TupleIndex(n) => n.base.contains_call(),
            LocalExpressionNode::Reaches(_) => false,
            LocalExpressionNode::CallChain(n) => {
                let mut has_call = match &n.base {
                    LocalCallChainBase::Expr(base) => base.contains_call(),
                    LocalCallChainBase::Name(_) => false,
                };
                for seg in n.segments.iter() {
                    if let LocalCallChainSegment::Invoke { args }
                    | LocalCallChainSegment::Call { args, .. } = seg
                    {
                        has_call |= args.contains_call();
                    }
                }
                has_call
            }
            LocalExpressionNode::IfExpr(n) => {
                n.condition.contains_call()
                    || n.then_expr.contains_call()
                    || n.else_expr
                        .as_ref()
                        .map(|e| e.contains_call())
                        .unwrap_or(false)
            }
            LocalExpressionNode::ForAll(n) => n.list.contains_call() || n.body.contains_call(),
            LocalExpressionNode::Exists(n) => n.list.contains_call() || n.body.contains_call(),
        }
    }

    fn scope_stack(scope: &[String]) -> Vec<Variable> {
        scope
            .iter()
            .map(|name| Variable {
                mutable: false,
                name: name.clone(),
                datatype: DataType::Void,
                depth: 0,
                declare_pos: None,
            })
            .collect()
    }

    fn localize_expression_for_scope(
        expression: &Node<Expression>,
        scope: &[String],
    ) -> Result<LocalExpressionNode, String> {
        LocalExpressionNode::from_expression(
            &expression.value,
            &LocalExpressionNode::scope_stack(scope),
        )
        .map_err(|e| e.message)
    }

    fn resolve_literal_in_scope(
        name: &str,
        mem: &Memory,
        scope: &[String],
        vm: &crate::vm::VM,
    ) -> Result<Literal, String> {
        if let Some(idx) = scope.iter().rposition(|scoped_name| scoped_name == name) {
            return mem
                .get(idx)
                .cloned()
                .ok_or_else(|| format!("Variable '{}' is missing from evaluation scope", name));
        }

        vm.globals
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Variable '{}' not found in evaluation scope", name))
    }

    fn evaluate_method_call(
        vm: &crate::vm::VM,
        receiver: &mut Literal,
        name: &str,
        args_value: &mut Literal,
        pos: Option<Pos>,
    ) -> Result<Literal, String> {
        invoke_interface_method(vm.stdlib.as_ref(), name, receiver, args_value, pos)
            .map(|(ret, _)| ret)
            .map_err(|e| e.message)
    }

    pub fn datatype(&self, state: &CompilerState) -> Result<DataType, String> {
        match self {
            Self::Binary(node) => node.datatype(state),
            Self::Unary(node) => node.datatype(state),
            Self::Primary(node) => node.datatype(state),
            Self::Tuple(node) => node.datatype(state),
            Self::Range(node) => node.datatype(state),
            Self::TupleIndex(node) => {
                let base_type = node.base.datatype(state)?;
                let DataType::Tuple(items) = base_type else {
                    return Err(format!(
                        "tuple index '.{}' requires a tuple, found {}",
                        node.index, base_type
                    ));
                };
                items.get(node.index).cloned().ok_or_else(|| {
                    format!(
                        "tuple index '.{}' is out of bounds for tuple of size {}",
                        node.index,
                        items.len()
                    )
                })
            }
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
                    let receiver_name = node
                        .value
                        .receiver_name()
                        .ok_or_else(|| format!("Receiver {} not found", full_name))?;
                    let var = state
                        .program_stack
                        .iter()
                        .rev()
                        .find(|v| v.name == receiver_name);
                    let global_var = state.global_table().get(&receiver_name);
                    if let Some(var) = var.or(global_var) {
                        let method_name = node
                            .value
                            .method_name()
                            .ok_or_else(|| format!("Method name missing in {}", full_name))?;
                        resolve_interface_method(&state.stdlib(), &var.datatype, &method_name)
                            .map(|method| method.ret)
                    } else {
                        Err(format!("Variable {} not found", receiver_name))
                    }
                }
            }
            Self::RunCall(node) => {
                let full_program_name = node.value.program_name_to_string();
                Ok(DataType::Process(full_program_name))
            }
            Self::Reaches(node) => {
                if !state.in_condition_block {
                    return Err(
                        "'reaches' is only available inside always/check blocks".to_string()
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
                let mut current_name = match &node.base {
                    LocalCallChainBase::Name(name) => Some(name.clone()),
                    LocalCallChainBase::Expr(_) => None,
                };
                let mut current_type = match &node.base {
                    LocalCallChainBase::Expr(base) => Some(base.datatype(state)?),
                    LocalCallChainBase::Name(_) => None,
                };
                for (idx, segment) in node.segments.iter().enumerate() {
                    match segment {
                        LocalCallChainSegment::Invoke { args } => {
                            let Some(function_name) = current_name.take() else {
                                return Err("calling arbitrary expression values is not supported"
                                    .to_string());
                            };
                            let ret = direct_function_return_type(&function_name, args, state)?;
                            current_type = Some(ret);
                        }
                        LocalCallChainSegment::Field { name } => {
                            if let Some(current_name) = current_name.as_mut() {
                                current_name.push('.');
                                current_name.push_str(name);
                                continue;
                            }

                            let receiver_type = current_type.as_ref().ok_or_else(|| {
                                "missing receiver before field access".to_string()
                            })?;
                            return Err(format!(
                                "field access '.{}' is not supported yet on type {}",
                                name, receiver_type
                            ));
                        }
                        LocalCallChainSegment::TupleIndex { index } => {
                            let receiver_type = current_type.as_ref().ok_or_else(|| {
                                "missing receiver before tuple access".to_string()
                            })?;
                            let DataType::Tuple(items) = receiver_type else {
                                return Err(format!(
                                    "tuple index '.{}' requires a tuple, found {}",
                                    index, receiver_type
                                ));
                            };
                            let item = items.get(*index).ok_or_else(|| {
                                format!(
                                    "tuple index '.{}' is out of bounds for tuple of size {}",
                                    index,
                                    items.len()
                                )
                            })?;
                            current_type = Some(item.clone());
                        }
                        LocalCallChainSegment::Call { name, args } => {
                            let receiver_type = current_type
                                .as_ref()
                                .ok_or_else(|| "missing receiver before method call".to_string())?;
                            let method =
                                resolve_interface_method(&state.stdlib(), receiver_type, name)?;
                            let args_type = args.datatype(state)?;
                            validate_interface_call(&method, tuple_arg_types(&args_type)?)?;
                            current_type = Some(method.ret.clone());
                        }
                        LocalCallChainSegment::Reaches { label } => {
                            if idx + 1 != node.segments.len() {
                                return Err("'reaches' must be the last segment in a call chain"
                                    .to_string());
                            }
                            let program_name = match current_type.as_ref() {
                                Some(DataType::Process(name)) => name.clone(),
                                _ => {
                                    return Err(
                                        "'reaches' must be called on a variable of type proc(<Program>)"
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

                            current_type = Some(DataType::Boolean);
                        }
                    }
                }
                current_type.ok_or_else(|| "postfix expression has no resulting value".to_string())
            }
            Self::IfExpr(node) => {
                if !state.in_condition_block {
                    return Err(
                        "if-expressions are only supported inside always/check blocks".to_string(),
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
                        return Err(
                            "if A { B } (without else) is an implication, so B must be boolean"
                                .to_string(),
                        );
                    }
                    Ok(DataType::Boolean)
                }
            }
            Self::ForAll(node) => {
                if !state.in_condition_block {
                    return Err("forall is only supported inside always/check blocks".to_string());
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
                    debug_variables: state.debug_variables.clone(),
                    program_debug_info: state.program_debug_info.clone(),
                };

                let body_type = node.body.datatype(&temp_state)?;
                if body_type != DataType::Boolean {
                    return Err("forall body must be boolean".to_string());
                }
                Ok(DataType::Boolean)
            }
            Self::Exists(node) => {
                if !state.in_condition_block {
                    return Err("exists is only supported inside always/check blocks".to_string());
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
                    debug_variables: state.debug_variables.clone(),
                    program_debug_info: state.program_debug_info.clone(),
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
            LocalExpressionNode::TupleIndex(node) => {
                let value = node.base.eval(mem)?;
                let Literal::Tuple(items) = value else {
                    return Err(format!(
                        "tuple index '.{}' requires a tuple runtime value",
                        node.index
                    ));
                };
                items.get(node.index).cloned().ok_or_else(|| {
                    format!(
                        "tuple index '.{}' is out of bounds for tuple of size {}",
                        node.index,
                        items.len()
                    )
                })
            }
            LocalExpressionNode::FnCall(node) => Err(format!(
                "Cannot evaluate function call in this context: {:?}",
                &node.value.fn_name
            )),
            LocalExpressionNode::RunCall(node) => Err(format!(
                "Cannot evaluate run call in this context: {:?}",
                &node.value.identifier
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
        self.eval_with_scope(mem, &[], vm)
    }

    pub fn eval_with_scope(
        &self,
        mem: &Memory,
        scope: &[String],
        vm: &crate::vm::VM,
    ) -> Result<Literal, String> {
        match self {
            LocalExpressionNode::Binary(binary_exp) => match binary_exp.operator {
                crate::ast::token::binary_operator::BinaryOperator::And => {
                    let left = binary_exp.left.eval_with_scope(mem, scope, vm)?;
                    if !left.is_true() {
                        return Ok(Literal::Bool(false));
                    }
                    let right = binary_exp.right.eval_with_scope(mem, scope, vm)?;
                    left.and(&right)
                }
                crate::ast::token::binary_operator::BinaryOperator::Or => {
                    let left = binary_exp.left.eval_with_scope(mem, scope, vm)?;
                    if left.is_true() {
                        return Ok(Literal::Bool(true));
                    }
                    let right = binary_exp.right.eval_with_scope(mem, scope, vm)?;
                    left.or(&right)
                }
                _ => {
                    let left = binary_exp.left.eval_with_scope(mem, scope, vm)?;
                    let right = binary_exp.right.eval_with_scope(mem, scope, vm)?;
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
                        crate::ast::token::binary_operator::BinaryOperator::ShiftLeft => {
                            left.shift_left(&right)
                        }
                        crate::ast::token::binary_operator::BinaryOperator::ShiftRight => {
                            left.shift_right(&right)
                        }
                        crate::ast::token::binary_operator::BinaryOperator::BitAnd => {
                            left.bit_and(&right)
                        }
                        crate::ast::token::binary_operator::BinaryOperator::BitOr => {
                            left.bit_or(&right)
                        }
                        _ => unreachable!("short-circuit handled above"),
                    }
                }
            },
            LocalExpressionNode::Unary(unary_exp) => {
                let operand = unary_exp.operand.eval_with_scope(mem, scope, vm)?;
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
                    expr.as_ref().eval_with_scope(mem, scope, vm)
                }
            },
            LocalExpressionNode::Tuple(tuple_exp) => Ok(Literal::Tuple(
                tuple_exp
                    .values
                    .iter()
                    .map(|v| v.eval_with_scope(mem, scope, vm))
                    .collect::<Result<Vec<Literal>, String>>()?,
            )),
            LocalExpressionNode::TupleIndex(node) => {
                let value = node.base.eval_with_scope(mem, scope, vm)?;
                let Literal::Tuple(items) = value else {
                    return Err(format!(
                        "tuple index '.{}' requires a tuple runtime value",
                        node.index
                    ));
                };
                items.get(node.index).cloned().ok_or_else(|| {
                    format!(
                        "tuple index '.{}' is out of bounds for tuple of size {}",
                        node.index,
                        items.len()
                    )
                })
            }
            LocalExpressionNode::Range(list_exp) => {
                let start = list_exp.expression_start.eval_with_scope(mem, scope, vm)?;
                let end = list_exp.expression_end.eval_with_scope(mem, scope, vm)?;
                Ok(Literal::List(
                    DataType::Integer,
                    (start.to_integer()?..end.to_integer()?)
                        .map(|v| Literal::Int(v))
                        .collect(),
                ))
            }
            LocalExpressionNode::FnCall(node) => {
                if node.value.fn_name.value.parts.len() == 1 {
                    return Err(format!(
                        "Cannot evaluate function call in this context: {:?}",
                        &node.value.fn_name
                    ));
                }

                let receiver_name = node.value.receiver_name().ok_or_else(|| {
                    format!("Receiver not found in call {:?}", &node.value.fn_name)
                })?;
                let method_name = node.value.method_name().ok_or_else(|| {
                    format!("Method name not found in call {:?}", &node.value.fn_name)
                })?;
                let mut receiver =
                    LocalExpressionNode::resolve_literal_in_scope(&receiver_name, mem, scope, vm)?;
                let args_expr = LocalExpressionNode::localize_expression_for_scope(
                    node.value.values.as_ref(),
                    scope,
                )?;
                let mut args_value = args_expr.eval_with_scope(mem, scope, vm)?;

                LocalExpressionNode::evaluate_method_call(
                    vm,
                    &mut receiver,
                    &method_name,
                    &mut args_value,
                    Some(node.pos.clone()),
                )
            }
            LocalExpressionNode::RunCall(node) => Err(format!(
                "Cannot evaluate run call in this context: {:?}",
                &node.value.identifier
            )),
            LocalExpressionNode::Reaches(node) => {
                let lit = mem
                    .get(mem.len() - 1 - node.var.index)
                    .ok_or("process variable index does not exist in memory".to_string())?;
                let (program_name, pid) = match (lit, node.index.as_ref()) {
                    (Literal::Process(name, pid), None) => (name.clone(), *pid),
                    (Literal::List(DataType::Process(_name), values), Some(index_expr)) => {
                        let idx_lit = index_expr.eval_with_scope(mem, scope, vm)?;
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
                        log::debug!(
                            "reaches({}) for pid {}: process not in running_programs (terminated)",
                            node.label,
                            pid
                        );
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

                // Debug logging to understand the issue
                log::debug!(
                    "reaches({}) for process {} (pid={}): ip={}, label_pc={}, reached={}",
                    node.label,
                    program_name,
                    pid,
                    ip,
                    label_pc,
                    reached
                );

                Ok(Literal::Bool(reached))
            }
            LocalExpressionNode::CallChain(node) => {
                let mut current = match &node.base {
                    LocalCallChainBase::Expr(base) => Some(base.eval_with_scope(mem, scope, vm)?),
                    LocalCallChainBase::Name(_) => None,
                };
                for segment in node.segments.iter() {
                    match segment {
                        LocalCallChainSegment::Invoke { .. } => {
                            return Err(
                                "Cannot evaluate direct function calls in this context".to_string()
                            );
                        }
                        LocalCallChainSegment::Field { name } => {
                            return Err(format!(
                                "Field access '.{}' is not supported in evaluation yet",
                                name
                            ));
                        }
                        LocalCallChainSegment::TupleIndex { index } => {
                            let current_value = current
                                .take()
                                .ok_or_else(|| "Missing receiver for tuple access".to_string())?;
                            let Literal::Tuple(values) = current_value else {
                                return Err(format!(
                                    "tuple index '.{}' requires a tuple runtime value",
                                    index
                                ));
                            };
                            let value = values.get(*index).cloned().ok_or_else(|| {
                                format!(
                                    "tuple index '.{}' is out of bounds for tuple of size {}",
                                    index,
                                    values.len()
                                )
                            })?;
                            current = Some(value);
                        }
                        LocalCallChainSegment::Call { name, args } => {
                            let mut args_value = args.eval_with_scope(mem, scope, vm)?;
                            let receiver = current
                                .as_mut()
                                .ok_or_else(|| "Missing receiver for method call".to_string())?;
                            current = Some(LocalExpressionNode::evaluate_method_call(
                                vm,
                                receiver,
                                name,
                                &mut args_value,
                                None,
                            )?);
                        }
                        LocalCallChainSegment::Reaches { label } => {
                            let current_value = current
                                .as_ref()
                                .ok_or_else(|| "Missing receiver for reaches call".to_string())?;
                            log::debug!(
                                "CallChain Reaches evaluation started for label '{}'",
                                label
                            );
                            let (program_name, pid) = match current_value {
                                Literal::Process(name, pid) => {
                                    log::debug!("  Current is Process({}, {})", name, pid);
                                    (name.clone(), *pid)
                                }
                                _ => {
                                    log::debug!("  Current is not a Process: {:?}", current_value);
                                    current = Some(Literal::Bool(false));
                                    continue;
                                }
                            };

                            let prog_state = match vm.running_programs.get(pid) {
                                Some(p) => {
                                    log::debug!(
                                        "  Process {} (pid={}) found in running_programs",
                                        program_name,
                                        pid
                                    );
                                    p
                                }
                                None => {
                                    log::debug!("  Process {} (pid={}) NOT in running_programs (terminated)", program_name, pid);
                                    if label == "end" {
                                        current = Some(Literal::Bool(true));
                                    } else {
                                        current = Some(Literal::Bool(false));
                                    }
                                    continue;
                                }
                            };

                            if prog_state.name != program_name {
                                log::debug!(
                                    "  Program name mismatch: expected {}, got {}",
                                    program_name,
                                    prog_state.name
                                );
                                current = Some(Literal::Bool(false));
                                continue;
                            }

                            let program_code = match vm.programs_code.get(&program_name) {
                                Some(code) => code,
                                None => {
                                    log::debug!("  Program code for {} not found", program_name);
                                    current = Some(Literal::Bool(false));
                                    continue;
                                }
                            };

                            let label_pc = match program_code.labels.get(label) {
                                Some(pc) => {
                                    log::debug!("  Label '{}' found at pc={}", label, pc);
                                    pc
                                }
                                None => {
                                    log::debug!(
                                        "  Label '{}' not found in program {}",
                                        label,
                                        program_name
                                    );
                                    current = Some(Literal::Bool(false));
                                    continue;
                                }
                            };

                            let (_, pc, _) = prog_state.current_state();
                            let reached = pc == *label_pc;
                            log::debug!("  pc={}, label_pc={}, reached={}", pc, label_pc, reached);
                            current = Some(Literal::Bool(reached));
                        }
                    }
                }
                current.ok_or_else(|| "postfix expression has no runtime value".to_string())
            }
            LocalExpressionNode::IfExpr(node) => {
                let cond = node.condition.eval_with_scope(mem, scope, vm)?;
                if cond.is_true() {
                    node.then_expr.eval_with_scope(mem, scope, vm)
                } else {
                    if let Some(else_expr) = &node.else_expr {
                        else_expr.eval_with_scope(mem, scope, vm)
                    } else {
                        // if A { B } means A -> B. If A is false, result is true.
                        Ok(Literal::Bool(true))
                    }
                }
            }
            LocalExpressionNode::ForAll(node) => {
                let list = node.list.eval_with_scope(mem, scope, vm)?;
                let values = match list {
                    Literal::List(_, values) => values,
                    _ => return Err("forall expects a list".to_string()),
                };

                let mut temp_scope = scope.to_vec();
                temp_scope.push(node.var_name.clone());

                for value in values.into_iter() {
                    let mut temp_mem = mem.clone();
                    temp_mem.push(value);
                    let body_value = node.body.eval_with_scope(&temp_mem, &temp_scope, vm)?;
                    if !body_value.is_true() {
                        return Ok(Literal::Bool(false));
                    }
                }
                Ok(Literal::Bool(true))
            }
            LocalExpressionNode::Exists(node) => {
                let list = node.list.eval_with_scope(mem, scope, vm)?;
                let values = match list {
                    Literal::List(_, values) => values,
                    _ => return Err("exists expects a list".to_string()),
                };

                let mut temp_scope = scope.to_vec();
                temp_scope.push(node.var_name.clone());

                for value in values.into_iter() {
                    let mut temp_mem = mem.clone();
                    temp_mem.push(value);
                    let body_value = node.body.eval_with_scope(&temp_mem, &temp_scope, vm)?;
                    if body_value.is_true() {
                        return Ok(Literal::Bool(true));
                    }
                }
                Ok(Literal::Bool(false))
            }
        }
    }
}

struct LoweredRuntimeExpression {
    setup: InstructionBuilderOk,
    expr: LocalExpressionNode,
    temp_types: Vec<DataType>,
    result_type: DataType,
}

impl LoweredRuntimeExpression {
    fn pure(expr: LocalExpressionNode, result_type: DataType) -> Self {
        Self {
            setup: InstructionBuilderOk::new(),
            expr,
            temp_types: Vec::new(),
            result_type,
        }
    }

    fn extracted(setup: InstructionBuilderOk, result_type: DataType) -> Self {
        Self {
            setup,
            expr: LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Var(LocalVarNode {
                index: 0,
            })),
            temp_types: vec![result_type.clone()],
            result_type,
        }
    }
}

#[derive(Clone)]
struct NamedReceiver {
    receiver_idx: usize,
    global_receiver: Option<String>,
    datatype: DataType,
    mutable: bool,
}

fn clone_state_with_stack(state: &CompilerState, program_stack: Vec<Variable>) -> CompilerState {
    CompilerState {
        program_stack,
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
        debug_variables: state.debug_variables.clone(),
        program_debug_info: state.program_debug_info.clone(),
    }
}

fn datatype_with_runtime_temps(
    expr: &LocalExpressionNode,
    temp_types: &[DataType],
    state: &CompilerState,
) -> Result<DataType, String> {
    let mut temp_stack = state.program_stack.clone();
    for datatype in temp_types {
        temp_stack.push(Variable {
            name: "<expr-temp>".to_string(),
            depth: state.current_stack_depth,
            mutable: false,
            datatype: datatype.clone(),
            declare_pos: None,
        });
    }
    expr.datatype(&clone_state_with_stack(state, temp_stack))
}

fn shift_non_temp_var_indices(
    expr: &LocalExpressionNode,
    shift: usize,
    temp_count: usize,
) -> LocalExpressionNode {
    match expr {
        LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Var(var)) => {
            let index = if var.index >= temp_count {
                var.index + shift
            } else {
                var.index
            };

            LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Var(LocalVarNode { index }))
        }
        LocalExpressionNode::Binary(node) => {
            LocalExpressionNode::Binary(LocalBinaryExpressionNode {
                left: Box::new(shift_non_temp_var_indices(&node.left, shift, temp_count)),
                operator: node.operator.clone(),
                right: Box::new(shift_non_temp_var_indices(&node.right, shift, temp_count)),
            })
        }
        LocalExpressionNode::Unary(node) => LocalExpressionNode::Unary(LocalUnaryExpressionNode {
            operand: Box::new(shift_non_temp_var_indices(&node.operand, shift, temp_count)),
            operator: node.operator.clone(),
        }),
        LocalExpressionNode::Tuple(node) => LocalExpressionNode::Tuple(LocalTupleExpressionNode {
            values: node
                .values
                .iter()
                .map(|value| shift_non_temp_var_indices(value, shift, temp_count))
                .collect(),
        }),
        LocalExpressionNode::TupleIndex(node) => {
            LocalExpressionNode::TupleIndex(LocalTupleIndexNode {
                base: Box::new(shift_non_temp_var_indices(&node.base, shift, temp_count)),
                index: node.index,
            })
        }
        LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(expr)) => {
            LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(Box::new(
                shift_non_temp_var_indices(expr, shift, temp_count),
            )))
        }
        LocalExpressionNode::Range(node) => {
            LocalExpressionNode::Range(LocalRangeListExpressionNode {
                expression_start: Box::new(shift_non_temp_var_indices(
                    &node.expression_start,
                    shift,
                    temp_count,
                )),
                expression_end: Box::new(shift_non_temp_var_indices(
                    &node.expression_end,
                    shift,
                    temp_count,
                )),
            })
        }
        _ => expr.clone(),
    }
}

fn shift_expr_for_embedding(
    expr: &LocalExpressionNode,
    own_temp_count: usize,
    temps_above_self: usize,
    total_temp_count: usize,
) -> LocalExpressionNode {
    match expr {
        LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Var(var)) => {
            let index = if var.index < own_temp_count {
                var.index + temps_above_self
            } else {
                var.index + total_temp_count
            };
            LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Var(LocalVarNode { index }))
        }
        LocalExpressionNode::Binary(node) => {
            LocalExpressionNode::Binary(LocalBinaryExpressionNode {
                left: Box::new(shift_expr_for_embedding(
                    &node.left,
                    own_temp_count,
                    temps_above_self,
                    total_temp_count,
                )),
                operator: node.operator.clone(),
                right: Box::new(shift_expr_for_embedding(
                    &node.right,
                    own_temp_count,
                    temps_above_self,
                    total_temp_count,
                )),
            })
        }
        LocalExpressionNode::Unary(node) => LocalExpressionNode::Unary(LocalUnaryExpressionNode {
            operand: Box::new(shift_expr_for_embedding(
                &node.operand,
                own_temp_count,
                temps_above_self,
                total_temp_count,
            )),
            operator: node.operator.clone(),
        }),
        LocalExpressionNode::Tuple(node) => LocalExpressionNode::Tuple(LocalTupleExpressionNode {
            values: node
                .values
                .iter()
                .map(|value| {
                    shift_expr_for_embedding(
                        value,
                        own_temp_count,
                        temps_above_self,
                        total_temp_count,
                    )
                })
                .collect(),
        }),
        LocalExpressionNode::TupleIndex(node) => {
            LocalExpressionNode::TupleIndex(LocalTupleIndexNode {
                base: Box::new(shift_expr_for_embedding(
                    &node.base,
                    own_temp_count,
                    temps_above_self,
                    total_temp_count,
                )),
                index: node.index,
            })
        }
        LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(expr)) => {
            LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(Box::new(
                shift_expr_for_embedding(expr, own_temp_count, temps_above_self, total_temp_count),
            )))
        }
        LocalExpressionNode::Range(node) => {
            LocalExpressionNode::Range(LocalRangeListExpressionNode {
                expression_start: Box::new(shift_expr_for_embedding(
                    &node.expression_start,
                    own_temp_count,
                    temps_above_self,
                    total_temp_count,
                )),
                expression_end: Box::new(shift_expr_for_embedding(
                    &node.expression_end,
                    own_temp_count,
                    temps_above_self,
                    total_temp_count,
                )),
            })
        }
        _ => expr.clone(),
    }
}

fn emit_expression_instruction(
    builder: &mut InstructionBuilderOk,
    expr: LocalExpressionNode,
    temp_count: usize,
    pos: &Pos,
) {
    let control = if temp_count == 0 {
        InstructionType::Expression(expr)
    } else if let LocalExpressionNode::Tuple(tuple) = expr {
        InstructionType::MakeTupleAndCleanup {
            elements: tuple.values,
            unstack_len: temp_count,
        }
    } else {
        InstructionType::ExpressionAndCleanup {
            expression: expr,
            unstack_len: temp_count,
        }
    };

    builder.instructions.push(Instruction {
        pos: Some(pos.clone()),
        control,
    });
}

fn materialize_lowered_expression(
    mut lowered: LoweredRuntimeExpression,
    existing_stack_results: usize,
    pos: &Pos,
) -> InstructionBuilderOk {
    if existing_stack_results > 0 {
        lowered.expr = shift_non_temp_var_indices(
            &lowered.expr,
            existing_stack_results,
            lowered.temp_types.len(),
        );
    }
    let mut builder = lowered.setup;
    emit_expression_instruction(&mut builder, lowered.expr, lowered.temp_types.len(), pos);
    builder
}

fn local_var_expr(index: usize) -> LocalExpressionNode {
    LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Var(LocalVarNode { index }))
}

fn literal_expr(value: Literal) -> LocalExpressionNode {
    LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Literal(LocalLiteralNode {
        value,
    }))
}

fn object_identifier_name(identifier: &Node<ObjectIdentifier>) -> String {
    identifier
        .value
        .parts
        .iter()
        .map(|part| part.value.value.as_str())
        .collect::<Vec<_>>()
        .join(".")
}

fn resolve_named_receiver(name: &str, state: &CompilerState) -> Option<NamedReceiver> {
    if let Some(raw_var_id) = state
        .program_stack
        .iter()
        .rev()
        .position(|var| var.name == name)
    {
        let var = &state.program_stack[state.program_stack.len() - 1 - raw_var_id];
        return Some(NamedReceiver {
            receiver_idx: raw_var_id + state.method_call_stack_offset,
            global_receiver: None,
            datatype: var.datatype.clone(),
            mutable: var.mutable,
        });
    }

    state
        .global_table()
        .get(name)
        .map(|global_var| NamedReceiver {
            receiver_idx: 0,
            global_receiver: Some(name.to_string()),
            datatype: global_var.datatype.clone(),
            mutable: global_var.mutable,
        })
}

fn validate_direct_function_call(
    function_name: &str,
    args_type: &DataType,
    state: &CompilerState,
    pos: &Pos,
) -> AlthreadResult<DataType> {
    let provided_arg_types = tuple_arg_types(args_type).map_err(|message| {
        AlthreadError::new(
            ErrorType::FunctionArgumentTypeMismatch,
            Some(pos.clone()),
            message,
        )
    })?;

    if let Some(func_def) = state.user_functions().get(function_name) {
        if func_def.arguments.len() != provided_arg_types.len() {
            return Err(AlthreadError::new(
                ErrorType::FunctionArgumentCountError,
                Some(pos.clone()),
                format!(
                    "Function '{}' expects {} arguments, but {} were provided.",
                    function_name,
                    func_def.arguments.len(),
                    provided_arg_types.len()
                ),
            ));
        }

        for (idx, ((arg_name, expected_type), provided_type)) in func_def
            .arguments
            .iter()
            .zip(provided_arg_types.iter())
            .enumerate()
        {
            if expected_type != provided_type {
                return Err(AlthreadError::new(
                    ErrorType::FunctionArgumentTypeMismatch,
                    Some(pos.clone()),
                    format!(
                        "Function '{}' expects argument {} ('{}') to be of type {}, but got {}.",
                        function_name,
                        idx + 1,
                        arg_name.value,
                        expected_type,
                        provided_type
                    ),
                ));
            }
        }

        return Ok(func_def.return_type.clone());
    }

    match function_name {
        "print" => {
            for (idx, arg_type) in provided_arg_types.iter().enumerate() {
                if *arg_type == DataType::Void {
                    return Err(AlthreadError::new(
                        ErrorType::FunctionArgumentTypeMismatch,
                        Some(pos.clone()),
                        format!(
                            "Function 'print' can't accept argument {} of type Void.",
                            idx + 1
                        ),
                    ));
                }
            }
            Ok(DataType::Void)
        }
        "assert" => {
            if provided_arg_types.len() != 2 {
                return Err(AlthreadError::new(
                    ErrorType::FunctionArgumentCountError,
                    Some(pos.clone()),
                    "Function 'assert' expects exactly 2 arguments.".to_string(),
                ));
            }
            if provided_arg_types[0] != DataType::Boolean {
                return Err(AlthreadError::new(
                    ErrorType::FunctionArgumentTypeMismatch,
                    Some(pos.clone()),
                    format!(
                        "Function 'assert' expects the first argument to be of type bool, but got {}.",
                        provided_arg_types[0]
                    ),
                ));
            }
            if provided_arg_types[1] != DataType::String {
                return Err(AlthreadError::new(
                    ErrorType::FunctionArgumentTypeMismatch,
                    Some(pos.clone()),
                    format!(
                        "Function 'assert' expects the second argument to be of type string, but got {}.",
                        provided_arg_types[1]
                    ),
                ));
            }
            Ok(DataType::Void)
        }
        _ => Err(AlthreadError::new(
            ErrorType::UndefinedFunction,
            Some(pos.clone()),
            format!("undefined function {}", function_name),
        )),
    }
}

fn lower_identifier_path(
    name: &str,
    pos: &Pos,
    state: &CompilerState,
) -> AlthreadResult<LoweredRuntimeExpression> {
    if let Some(local_index) = state
        .program_stack
        .iter()
        .rev()
        .position(|var| var.name == name)
    {
        let var = &state.program_stack[state.program_stack.len() - 1 - local_index];
        return Ok(LoweredRuntimeExpression::pure(
            local_var_expr(local_index),
            var.datatype.clone(),
        ));
    }

    if let Some(global_var) = state.global_table().get(name) {
        let mut builder = InstructionBuilderOk::new();
        builder.instructions.push(Instruction {
            pos: Some(pos.clone()),
            control: InstructionType::GlobalReads {
                variables: vec![name.to_string()],
                only_const: !global_var.mutable,
            },
        });
        return Ok(LoweredRuntimeExpression::extracted(
            builder,
            global_var.datatype.clone(),
        ));
    }

    Err(AlthreadError::new(
        ErrorType::VariableError,
        Some(pos.clone()),
        format!("Variable '{}' not found", name),
    ))
}

fn lower_runtime_expression(
    expression: &Node<Expression>,
    state: &CompilerState,
) -> AlthreadResult<LoweredRuntimeExpression> {
    match &expression.value {
        Expression::Binary(node) => {
            let left = lower_runtime_expression(&node.value.left, state)?;
            let right = lower_runtime_expression(&node.value.right, state)?;
            let left_temp_count = left.temp_types.len();
            let right_temp_count = right.temp_types.len();
            let total_temps = left_temp_count + right_temp_count;
            let mut setup = left.setup;
            setup.extend(right.setup);
            let mut temp_types = left.temp_types;
            temp_types.extend(right.temp_types);

            let left_expr = shift_expr_for_embedding(
                &left.expr,
                left_temp_count,
                right_temp_count,
                total_temps,
            );
            let right_expr =
                shift_expr_for_embedding(&right.expr, right_temp_count, 0, total_temps);
            let expr = LocalExpressionNode::Binary(LocalBinaryExpressionNode {
                left: Box::new(left_expr),
                operator: node.value.operator.value.clone(),
                right: Box::new(right_expr),
            });
            let result_type =
                datatype_with_runtime_temps(&expr, &temp_types, state).map_err(|message| {
                    AlthreadError::new(
                        ErrorType::ExpressionError,
                        Some(expression.pos.clone()),
                        message,
                    )
                })?;
            Ok(LoweredRuntimeExpression {
                setup,
                expr,
                temp_types,
                result_type,
            })
        }
        Expression::Unary(node) => {
            let operand = lower_runtime_expression(&node.value.operand, state)?;
            let expr = LocalExpressionNode::Unary(LocalUnaryExpressionNode {
                operand: Box::new(operand.expr),
                operator: node.value.operator.value.clone(),
            });
            let result_type = datatype_with_runtime_temps(&expr, &operand.temp_types, state)
                .map_err(|message| {
                    AlthreadError::new(
                        ErrorType::ExpressionError,
                        Some(expression.pos.clone()),
                        message,
                    )
                })?;
            Ok(LoweredRuntimeExpression {
                setup: operand.setup,
                expr,
                temp_types: operand.temp_types,
                result_type,
            })
        }
        Expression::Primary(node) => match &node.value {
            PrimaryExpression::Literal(literal) => Ok(LoweredRuntimeExpression::pure(
                literal_expr(literal.value.clone()),
                literal.value.get_datatype(),
            )),
            PrimaryExpression::Identifier(identifier) => {
                let name = object_identifier_name(identifier);
                lower_identifier_path(&name, &expression.pos, state)
            }
            PrimaryExpression::Expression(inner) => {
                let inner_lowered = lower_runtime_expression(inner, state)?;
                let expr = LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(
                    Box::new(inner_lowered.expr),
                ));
                let result_type =
                    datatype_with_runtime_temps(&expr, &inner_lowered.temp_types, state).map_err(
                        |message| {
                            AlthreadError::new(
                                ErrorType::ExpressionError,
                                Some(expression.pos.clone()),
                                message,
                            )
                        },
                    )?;
                Ok(LoweredRuntimeExpression {
                    setup: inner_lowered.setup,
                    expr,
                    temp_types: inner_lowered.temp_types,
                    result_type,
                })
            }
            _ => Err(AlthreadError::new(
                ErrorType::InstructionNotAllowed,
                Some(expression.pos.clone()),
                "This expression kind is only supported inside always/check blocks".to_string(),
            )),
        },
        Expression::Tuple(node) => {
            let lowered_values = node
                .value
                .values
                .iter()
                .map(|value| lower_runtime_expression(value, state))
                .collect::<AlthreadResult<Vec<_>>>()?;

            let total_temps = lowered_values
                .iter()
                .map(|value| value.temp_types.len())
                .sum::<usize>();
            let mut temps_to_right = total_temps;
            let mut setup = InstructionBuilderOk::new();
            let mut temp_types = Vec::new();
            let mut values = Vec::new();

            for lowered in lowered_values {
                temps_to_right -= lowered.temp_types.len();
                let embedded = shift_expr_for_embedding(
                    &lowered.expr,
                    lowered.temp_types.len(),
                    temps_to_right,
                    total_temps,
                );
                setup.extend(lowered.setup);
                temp_types.extend(lowered.temp_types);
                values.push(embedded);
            }

            let expr = LocalExpressionNode::Tuple(LocalTupleExpressionNode { values });
            let result_type =
                datatype_with_runtime_temps(&expr, &temp_types, state).map_err(|message| {
                    AlthreadError::new(
                        ErrorType::ExpressionError,
                        Some(expression.pos.clone()),
                        message,
                    )
                })?;
            Ok(LoweredRuntimeExpression {
                setup,
                expr,
                temp_types,
                result_type,
            })
        }
        Expression::Range(node) => {
            let start = lower_runtime_expression(&node.value.expression_start, state)?;
            let end = lower_runtime_expression(&node.value.expression_end, state)?;
            let start_temp_count = start.temp_types.len();
            let end_temp_count = end.temp_types.len();
            let total_temps = start_temp_count + end_temp_count;
            let mut setup = start.setup;
            setup.extend(end.setup);
            let mut temp_types = start.temp_types;
            temp_types.extend(end.temp_types);

            let start_expr = shift_expr_for_embedding(
                &start.expr,
                start_temp_count,
                end_temp_count,
                total_temps,
            );
            let end_expr = shift_expr_for_embedding(&end.expr, end_temp_count, 0, total_temps);
            let expr = LocalExpressionNode::Range(LocalRangeListExpressionNode {
                expression_start: Box::new(start_expr),
                expression_end: Box::new(end_expr),
            });
            let result_type =
                datatype_with_runtime_temps(&expr, &temp_types, state).map_err(|message| {
                    AlthreadError::new(
                        ErrorType::ExpressionError,
                        Some(expression.pos.clone()),
                        message,
                    )
                })?;
            Ok(LoweredRuntimeExpression {
                setup,
                expr,
                temp_types,
                result_type,
            })
        }
        Expression::FnCall(node) => {
            let args = lower_runtime_expression(node.value.values.as_ref(), state)?;
            let args_type = args.result_type.clone();
            let ret_type = validate_direct_function_call(
                &node.value.fn_name_to_string(),
                &args_type,
                state,
                &expression.pos,
            )?;
            let mut setup = materialize_lowered_expression(args, 0, &expression.pos);
            setup.instructions.push(Instruction {
                pos: Some(expression.pos.clone()),
                control: InstructionType::FnCall {
                    name: node.value.fn_name_to_string(),
                    unstack_len: 1,
                    arguments: None,
                },
            });
            Ok(LoweredRuntimeExpression::extracted(setup, ret_type))
        }
        Expression::RunCall(node) => {
            let args = lower_runtime_expression(&node.value.args, state)?;
            let args_type = args.result_type.clone();
            let call_datatype = tuple_arg_types(&args_type).map_err(|message| {
                AlthreadError::new(ErrorType::TypeError, Some(expression.pos.clone()), message)
            })?;
            let full_program_name = node.value.program_name_to_string();
            let Some((prog_args, _)) = state.program_arguments().get(&full_program_name) else {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(expression.pos.clone()),
                    format!("Program {} does not exist", full_program_name),
                ));
            };
            if prog_args.len() != call_datatype.len() {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(expression.pos.clone()),
                    format!(
                        "Expected {} argument(s), got {}",
                        prog_args.len(),
                        call_datatype.len()
                    ),
                ));
            }
            for (idx, arg) in prog_args.iter().enumerate() {
                if arg != &call_datatype[idx] {
                    return Err(AlthreadError::new(
                        ErrorType::TypeError,
                        Some(expression.pos.clone()),
                        format!(
                            "Expected argument {} to be of type {:?}, got {:?}",
                            idx + 1,
                            arg,
                            call_datatype[idx]
                        ),
                    ));
                }
            }

            let mut setup = materialize_lowered_expression(args, 0, &expression.pos);
            setup.instructions.push(Instruction {
                pos: Some(expression.pos.clone()),
                control: InstructionType::RunCall {
                    name: full_program_name.clone(),
                    unstack_len: 1,
                },
            });
            Ok(LoweredRuntimeExpression::extracted(
                setup,
                DataType::Process(full_program_name),
            ))
        }
        Expression::Bracket(node) => match &node.value.content {
            BracketContent::Range(range) => lower_runtime_expression(
                &Node {
                    pos: range.pos.clone(),
                    value: Expression::Range(range.clone()),
                },
                state,
            ),
            BracketContent::ListLiteral(values) => {
                let mut setup = InstructionBuilderOk::new();
                let mut existing_results = 0;
                let mut element_type = None::<DataType>;

                for value in values {
                    let lowered = lower_runtime_expression(value, state)?;
                    if let Some(expected) = &element_type {
                        if lowered.result_type != *expected {
                            return Err(AlthreadError::new(
                                ErrorType::ExpressionError,
                                Some(value.pos.clone()),
                                format!(
                                    "List literal element has type {}, expected {}",
                                    lowered.result_type, expected
                                ),
                            ));
                        }
                    } else {
                        element_type = Some(lowered.result_type.clone());
                    }
                    setup.extend(materialize_lowered_expression(
                        lowered,
                        existing_results,
                        &value.pos,
                    ));
                    existing_results += 1;
                }

                let element_type = element_type.unwrap_or(DataType::Void);
                setup.instructions.push(Instruction {
                    pos: Some(expression.pos.clone()),
                    control: InstructionType::CreateListFromStack {
                        element_count: values.len(),
                        element_type: element_type.clone(),
                    },
                });

                Ok(LoweredRuntimeExpression::extracted(
                    setup,
                    DataType::List(Box::new(element_type)),
                ))
            }
        },
        Expression::CallChain(node) => {
            let mut name_path = None::<String>;
            let mut current = None::<LoweredRuntimeExpression>;

            if let Expression::Primary(primary) = &node.value.base.value {
                if let PrimaryExpression::Identifier(identifier) = &primary.value {
                    let base_name = object_identifier_name(identifier);
                    if resolve_named_receiver(&base_name, state).is_some() {
                        current = Some(lower_identifier_path(
                            &base_name,
                            &node.value.base.pos,
                            state,
                        )?);
                    } else {
                        name_path = Some(base_name);
                    }
                }
            }

            if current.is_none() && name_path.is_none() {
                current = Some(lower_runtime_expression(node.value.base.as_ref(), state)?);
            }

            for segment in &node.value.segments {
                match segment {
                    CallChainSegment::Field { name } => {
                        if let Some(path) = name_path.as_mut() {
                            path.push('.');
                            path.push_str(&name.value.value);
                        } else {
                            return Err(AlthreadError::new(
                                ErrorType::InstructionNotAllowed,
                                Some(expression.pos.clone()),
                                format!(
                                    "field access '.{}' is not supported in compilation yet",
                                    name.value.value
                                ),
                            ));
                        }
                    }
                    CallChainSegment::TupleIndex { index } => {
                        if let Some(path) = name_path.take() {
                            current = Some(lower_identifier_path(&path, &expression.pos, state)?);
                        }
                        let lowered = current.take().ok_or_else(|| {
                            AlthreadError::new(
                                ErrorType::ExpressionError,
                                Some(expression.pos.clone()),
                                "tuple access is missing its receiver".to_string(),
                            )
                        })?;
                        let expr = LocalExpressionNode::TupleIndex(LocalTupleIndexNode {
                            base: Box::new(lowered.expr),
                            index: *index,
                        });
                        let result_type =
                            datatype_with_runtime_temps(&expr, &lowered.temp_types, state)
                                .map_err(|message| {
                                    AlthreadError::new(
                                        ErrorType::ExpressionError,
                                        Some(expression.pos.clone()),
                                        message,
                                    )
                                })?;
                        current = Some(LoweredRuntimeExpression {
                            setup: lowered.setup,
                            expr,
                            temp_types: lowered.temp_types,
                            result_type,
                        });
                    }
                    CallChainSegment::Invoke { args } => {
                        let function_name = name_path.take().ok_or_else(|| {
                            AlthreadError::new(
                                ErrorType::ExpressionError,
                                Some(expression.pos.clone()),
                                "calling arbitrary expression values is not supported".to_string(),
                            )
                        })?;
                        let args_lowered = lower_runtime_expression(args, state)?;
                        let args_type = args_lowered.result_type.clone();
                        let ret_type = validate_direct_function_call(
                            &function_name,
                            &args_type,
                            state,
                            &expression.pos,
                        )?;
                        let mut setup =
                            materialize_lowered_expression(args_lowered, 0, &expression.pos);
                        setup.instructions.push(Instruction {
                            pos: Some(expression.pos.clone()),
                            control: InstructionType::FnCall {
                                name: function_name,
                                unstack_len: 1,
                                arguments: None,
                            },
                        });
                        current = Some(LoweredRuntimeExpression::extracted(setup, ret_type));
                    }
                    CallChainSegment::Call { name, args } => {
                        let method_name = name.value.value.clone();
                        if let Some(path) = name_path.take() {
                            if let Some(receiver) = resolve_named_receiver(&path, state) {
                                let args_lowered = lower_runtime_expression(args, state)?;
                                let args_type = args_lowered.result_type.clone();
                                let method = resolve_interface_method(
                                    &state.stdlib(),
                                    &receiver.datatype,
                                    &method_name,
                                )
                                .map_err(|message| {
                                    AlthreadError::new(
                                        ErrorType::UndefinedFunction,
                                        Some(expression.pos.clone()),
                                        message,
                                    )
                                })?;
                                let arg_types = tuple_arg_types(&args_type).map_err(|message| {
                                    AlthreadError::new(
                                        ErrorType::FunctionArgumentTypeMismatch,
                                        Some(expression.pos.clone()),
                                        message,
                                    )
                                })?;
                                validate_interface_call(&method, arg_types).map_err(|message| {
                                    AlthreadError::new(
                                        ErrorType::FunctionArgumentTypeMismatch,
                                        Some(expression.pos.clone()),
                                        message,
                                    )
                                })?;
                                if method.mutates_receiver && !receiver.mutable {
                                    return Err(AlthreadError::new(
                                        ErrorType::VariableError,
                                        Some(expression.pos.clone()),
                                        format!(
                                            "Cannot call mutating method '{}' on immutable variable {}",
                                            method_name, path
                                        ),
                                    ));
                                }

                                let mut setup = materialize_lowered_expression(
                                    args_lowered,
                                    0,
                                    &expression.pos,
                                );
                                setup.instructions.push(Instruction {
                                    pos: Some(expression.pos.clone()),
                                    control: InstructionType::MethodCall {
                                        name: method_name,
                                        receiver_idx: receiver.receiver_idx,
                                        unstack_len: 1,
                                        drop_receiver: false,
                                        arguments: None,
                                        global_receiver: receiver.global_receiver,
                                    },
                                });
                                current = Some(LoweredRuntimeExpression::extracted(
                                    setup,
                                    method.ret.clone(),
                                ));
                            } else {
                                let function_name = format!("{path}.{}", method_name);
                                let args_lowered = lower_runtime_expression(args, state)?;
                                let args_type = args_lowered.result_type.clone();
                                let ret_type = validate_direct_function_call(
                                    &function_name,
                                    &args_type,
                                    state,
                                    &expression.pos,
                                )?;
                                let mut setup = materialize_lowered_expression(
                                    args_lowered,
                                    0,
                                    &expression.pos,
                                );
                                setup.instructions.push(Instruction {
                                    pos: Some(expression.pos.clone()),
                                    control: InstructionType::FnCall {
                                        name: function_name,
                                        unstack_len: 1,
                                        arguments: None,
                                    },
                                });
                                current =
                                    Some(LoweredRuntimeExpression::extracted(setup, ret_type));
                            }
                        } else {
                            let receiver_lowered = current.take().ok_or_else(|| {
                                AlthreadError::new(
                                    ErrorType::ExpressionError,
                                    Some(expression.pos.clone()),
                                    "missing receiver before method call".to_string(),
                                )
                            })?;
                            let receiver_type = receiver_lowered.result_type.clone();
                            let args_lowered = lower_runtime_expression(args, state)?;
                            let args_type = args_lowered.result_type.clone();
                            let method = resolve_interface_method(
                                &state.stdlib(),
                                &receiver_type,
                                &method_name,
                            )
                            .map_err(|message| {
                                AlthreadError::new(
                                    ErrorType::UndefinedFunction,
                                    Some(expression.pos.clone()),
                                    message,
                                )
                            })?;
                            let arg_types = tuple_arg_types(&args_type).map_err(|message| {
                                AlthreadError::new(
                                    ErrorType::FunctionArgumentTypeMismatch,
                                    Some(expression.pos.clone()),
                                    message,
                                )
                            })?;
                            validate_interface_call(&method, arg_types).map_err(|message| {
                                AlthreadError::new(
                                    ErrorType::FunctionArgumentTypeMismatch,
                                    Some(expression.pos.clone()),
                                    message,
                                )
                            })?;

                            let mut setup = materialize_lowered_expression(
                                receiver_lowered,
                                0,
                                &expression.pos,
                            );
                            setup.extend(materialize_lowered_expression(
                                args_lowered,
                                1,
                                &expression.pos,
                            ));
                            setup.instructions.push(Instruction {
                                pos: Some(expression.pos.clone()),
                                control: InstructionType::MethodCall {
                                    name: method_name,
                                    receiver_idx: 1,
                                    unstack_len: 1,
                                    drop_receiver: true,
                                    arguments: None,
                                    global_receiver: None,
                                },
                            });
                            current = Some(LoweredRuntimeExpression::extracted(
                                setup,
                                method.ret.clone(),
                            ));
                        }
                    }
                    CallChainSegment::Reaches { .. } => {
                        return Err(AlthreadError::new(
                            ErrorType::InstructionNotAllowed,
                            Some(expression.pos.clone()),
                            "'reaches' is only allowed inside always/check blocks".to_string(),
                        ));
                    }
                }
            }

            if let Some(path) = name_path {
                lower_identifier_path(&path, &expression.pos, state)
            } else {
                current.ok_or_else(|| {
                    AlthreadError::new(
                        ErrorType::ExpressionError,
                        Some(expression.pos.clone()),
                        "postfix expression has no resulting value".to_string(),
                    )
                })
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
        let mut current_name = match &self.base.value {
            Expression::Primary(primary_node) => match &primary_node.value {
                PrimaryExpression::Identifier(identifier)
                    if matches!(self.segments.first(), Some(CallChainSegment::Invoke { .. })) =>
                {
                    Some(
                        identifier
                            .value
                            .parts
                            .iter()
                            .map(|part| part.value.value.as_str())
                            .collect::<Vec<_>>()
                            .join("."),
                    )
                }
                _ => None,
            },
            _ => None,
        };

        if current_name.is_none() {
            let base_builder = self.base.compile(state)?;
            builder.extend(base_builder);
        }

        for segment in &self.segments {
            match segment {
                CallChainSegment::Invoke { args } => {
                    let function_name = current_name.take().ok_or_else(|| {
                        AlthreadError::new(
                            ErrorType::ExpressionError,
                            Some(pos.clone()),
                            "calling arbitrary expression values is not supported".to_string(),
                        )
                    })?;
                    let temp_call = temp_fn_call_node(&function_name, args.clone(), pos);
                    builder.extend(temp_call.compile(state)?);
                }
                CallChainSegment::Field { name } => {
                    if let Some(current_name) = current_name.as_mut() {
                        current_name.push('.');
                        current_name.push_str(&name.value.value);
                    } else {
                        return Err(AlthreadError::new(
                            ErrorType::InstructionNotAllowed,
                            Some(pos.clone()),
                            format!(
                                "field access '.{}' is not supported in compilation yet",
                                name.value.value
                            ),
                        ));
                    }
                }
                CallChainSegment::TupleIndex { index } => {
                    return Err(AlthreadError::new(
                        ErrorType::InstructionNotAllowed,
                        Some(pos.clone()),
                        format!(
                            "tuple access '.{}' is not supported in compilation yet",
                            index
                        ),
                    ));
                }
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
                    let method = resolve_interface_method(
                        &state.stdlib(),
                        &receiver_var.datatype,
                        &name.value.value,
                    )
                    .map_err(|message| {
                        AlthreadError::new(ErrorType::UndefinedFunction, Some(pos.clone()), message)
                    })?;

                    let args_var = state.program_stack.last().unwrap();
                    let arg_types = tuple_arg_types(&args_var.datatype).map_err(|message| {
                        AlthreadError::new(
                            ErrorType::FunctionArgumentTypeMismatch,
                            Some(pos.clone()),
                            message,
                        )
                    })?;
                    validate_interface_call(&method, arg_types).map_err(|message| {
                        AlthreadError::new(
                            ErrorType::FunctionArgumentTypeMismatch,
                            Some(pos.clone()),
                            message,
                        )
                    })?;

                    builder.instructions.push(Instruction {
                        pos: Some(pos.clone()),
                        control: InstructionType::MethodCall {
                            name: name.value.value.clone(),
                            receiver_idx: 1,
                            unstack_len: 1,
                            drop_receiver: true,
                            arguments: None,
                            global_receiver: None,
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
        if !state.in_condition_block {
            let lowered = lower_runtime_expression(self, state)?;
            let result_type = lowered.result_type.clone();
            let builder = materialize_lowered_expression(lowered, 0, &self.pos);
            state.program_stack.push(Variable {
                name: "".to_string(),
                depth: state.current_stack_depth,
                mutable: false,
                datatype: result_type,
                declare_pos: None,
            });
            return Ok(builder);
        }
        match &self.value {
            Expression::RunCall(node) => return node.compile(state),
            Expression::Bracket(node) => return node.compile(state),
            _ => {}
        }
        if let Expression::CallChain(node) = &self.value {
            if !state.in_condition_block {
                return node.value.compile_chain(state, &self.pos);
            }
        }
        let mut instructions = Vec::new();
        let mut vars = HashSet::new();
        if state.in_condition_block {
            let mut dependencies = WaitDependency::new();
            self.value.add_dependencies(&mut dependencies);
            vars.extend(dependencies.variables);
        } else {
            self.value.get_vars(&mut vars);
        }

        if !state.in_condition_block && vars.iter().any(|var| var.starts_with("$.procs.")) {
            return Err(AlthreadError::new(
                ErrorType::InstructionNotAllowed,
                Some(self.pos.clone()),
                "$.procs.* is only available inside always/check blocks".to_string(),
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

        if state.in_condition_block {
            instructions.push(Instruction {
                pos: Some(self.pos.clone()),
                control: InstructionType::Expression(local_expr),
            });
        } else if !local_expr.contains_call() {
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

            fn shift_non_temp_var_indices(
                expr: &LocalExpressionNode,
                shift: usize,
                temp_count: usize,
            ) -> LocalExpressionNode {
                match expr {
                    LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Var(var)) => {
                        let index = if var.index >= temp_count {
                            var.index + shift
                        } else {
                            var.index
                        };

                        LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Var(
                            LocalVarNode { index },
                        ))
                    }
                    LocalExpressionNode::Binary(node) => {
                        LocalExpressionNode::Binary(LocalBinaryExpressionNode {
                            left: Box::new(shift_non_temp_var_indices(
                                &node.left, shift, temp_count,
                            )),
                            operator: node.operator.clone(),
                            right: Box::new(shift_non_temp_var_indices(
                                &node.right,
                                shift,
                                temp_count,
                            )),
                        })
                    }
                    LocalExpressionNode::Unary(node) => {
                        LocalExpressionNode::Unary(LocalUnaryExpressionNode {
                            operand: Box::new(shift_non_temp_var_indices(
                                &node.operand,
                                shift,
                                temp_count,
                            )),
                            operator: node.operator.clone(),
                        })
                    }
                    LocalExpressionNode::Tuple(node) => {
                        LocalExpressionNode::Tuple(LocalTupleExpressionNode {
                            values: node
                                .values
                                .iter()
                                .map(|value| shift_non_temp_var_indices(value, shift, temp_count))
                                .collect(),
                        })
                    }
                    LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(expr)) => {
                        LocalExpressionNode::Primary(LocalPrimaryExpressionNode::Expression(
                            Box::new(shift_non_temp_var_indices(expr, shift, temp_count)),
                        ))
                    }
                    LocalExpressionNode::Range(node) => {
                        LocalExpressionNode::Range(LocalRangeListExpressionNode {
                            expression_start: Box::new(shift_non_temp_var_indices(
                                &node.expression_start,
                                shift,
                                temp_count,
                            )),
                            expression_end: Box::new(shift_non_temp_var_indices(
                                &node.expression_end,
                                shift,
                                temp_count,
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
                    LocalExpressionNode::RunCall(node) => {
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

                        let shifted_right = if left_calls > 0 {
                            shift_non_temp_var_indices(&right_expr, left_calls, right_calls)
                        } else {
                            right_expr
                        };

                        let new_expr = LocalExpressionNode::Binary(LocalBinaryExpressionNode {
                            left: Box::new(shifted_left),
                            right: Box::new(shifted_right),
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
                if matches!(self.value, Expression::FnCall(_) | Expression::RunCall(_)) {
                    // Direct call expressions handle their own stack effect.
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
            Self::RunCall(_) => {}
            Self::Tuple(node) => node.value.add_dependencies(dependencies),
            Self::Range(node) => node.value.add_dependencies(dependencies),
            Self::Bracket(node) => match &node.value.content {
                BracketContent::Range(range) => range.value.add_dependencies(dependencies),
                BracketContent::ListLiteral(values) => {
                    for value in values {
                        value.value.add_dependencies(dependencies);
                    }
                }
            },
            Self::CallChain(node) => {
                let skip_base = matches!(
                    (&node.value.base.value, node.value.segments.first()),
                    (
                        Expression::Primary(primary_node),
                        Some(CallChainSegment::Invoke { .. })
                    ) if matches!(primary_node.value, PrimaryExpression::Identifier(_))
                );
                if !skip_base {
                    node.value.base.value.add_dependencies(dependencies);
                }
                for segment in node.value.segments.iter() {
                    if let CallChainSegment::Invoke { args } | CallChainSegment::Call { args, .. } =
                        segment
                    {
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
            Self::RunCall(node) => node.value.args.value.get_vars(vars),
            Self::Bracket(node) => match &node.value.content {
                BracketContent::Range(range) => range.value.get_vars(vars),
                BracketContent::ListLiteral(values) => {
                    for value in values {
                        value.value.get_vars(vars);
                    }
                }
            },
            Self::CallChain(node) => {
                let skip_base = matches!(
                    (&node.value.base.value, node.value.segments.first()),
                    (
                        Expression::Primary(primary_node),
                        Some(CallChainSegment::Invoke { .. })
                    ) if matches!(primary_node.value, PrimaryExpression::Identifier(_))
                );
                if !skip_base {
                    node.value.base.value.get_vars(vars);
                }
                for segment in node.value.segments.iter() {
                    if let CallChainSegment::Invoke { args } | CallChainSegment::Call { args, .. } =
                        segment
                    {
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
            Self::RunCall(node) => node.ast_fmt(f, prefix),
            Self::Bracket(node) => node.ast_fmt(f, prefix),
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
                CallChainSegment::Invoke { .. } => {
                    writeln!(f, "{}call", seg_prefix)?;
                }
                CallChainSegment::Field { name } => {
                    writeln!(f, "{}field: {}", seg_prefix, name.value.value)?;
                }
                CallChainSegment::TupleIndex { index } => {
                    writeln!(f, "{}tuple_index: {}", seg_prefix, index)?;
                }
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
    use crate::{
        ast::token::{binary_operator::BinaryOperator, literal::Literal},
        compiler::{CompilationContext, CompilerState, Variable},
        vm::instruction::InstructionType,
    };
    use std::{cell::RefCell, rc::Rc};

    fn test_pos() -> Pos {
        Pos {
            line: 0,
            col: 0,
            start: 0,
            end: 0,
            file_path: "test".to_string(),
        }
    }

    fn test_state() -> CompilerState {
        CompilerState::new_with_context(Rc::new(RefCell::new(CompilationContext::new())))
    }

    fn identifier_expression(name: &str) -> Node<Expression> {
        Node {
            pos: test_pos(),
            value: Expression::Primary(Node {
                pos: test_pos(),
                value: PrimaryExpression::Identifier(Node {
                    pos: test_pos(),
                    value: ObjectIdentifier {
                        parts: vec![Node {
                            pos: test_pos(),
                            value: Identifier {
                                value: name.to_string(),
                            },
                        }],
                    },
                }),
            }),
        }
    }

    fn int_expression(value: i64) -> Node<Expression> {
        Node {
            pos: test_pos(),
            value: Expression::Primary(Node {
                pos: test_pos(),
                value: PrimaryExpression::Literal(Node {
                    pos: test_pos(),
                    value: Literal::Int(value),
                }),
            }),
        }
    }

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

    #[test]
    fn test_shift_left_expression() {
        let literal_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: Literal::Int(8),
        };
        let shift_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: Literal::Int(2),
        };

        let left_expr = Expression::Primary(Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: PrimaryExpression::Literal(literal_node),
        });

        let right_expr = Expression::Primary(Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: PrimaryExpression::Literal(shift_node),
        });

        let expr = Expression::Binary(Node {
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
                    value: left_expr,
                }),
                right: Box::new(Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                        file_path: "test".to_string(),
                    },
                    value: right_expr,
                }),
                operator: Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                        file_path: "test".to_string(),
                    },
                    value: BinaryOperator::ShiftLeft,
                },
            },
        });

        let local_expr = LocalExpressionNode::from_expression(&expr, &vec![]).unwrap();
        assert_eq!(local_expr.eval(&Memory::new()).unwrap(), Literal::Int(32));
    }

    #[test]
    fn test_shift_right_expression() {
        let literal_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: Literal::Int(32),
        };
        let shift_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: Literal::Int(3),
        };

        let left_expr = Expression::Primary(Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: PrimaryExpression::Literal(literal_node),
        });

        let right_expr = Expression::Primary(Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: PrimaryExpression::Literal(shift_node),
        });

        let expr = Expression::Binary(Node {
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
                    value: left_expr,
                }),
                right: Box::new(Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                        file_path: "test".to_string(),
                    },
                    value: right_expr,
                }),
                operator: Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                        file_path: "test".to_string(),
                    },
                    value: BinaryOperator::ShiftRight,
                },
            },
        });

        let local_expr = LocalExpressionNode::from_expression(&expr, &vec![]).unwrap();
        assert_eq!(local_expr.eval(&Memory::new()).unwrap(), Literal::Int(4));
    }

    #[test]
    fn test_shift_out_of_range_fails() {
        let left = Literal::Int(1);
        let right = Literal::Int(64); // i64::BITS
        let err = left.shift_left(&right).unwrap_err();
        assert!(err.contains("Shift count out of range"));
    }

    #[test]
    fn test_bitwise_and_expression() {
        let literal_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: Literal::Int(6), // 110 in binary
        };
        let and_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: Literal::Int(3), // 011 in binary
        };

        let left_expr = Expression::Primary(Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: PrimaryExpression::Literal(literal_node),
        });

        let right_expr = Expression::Primary(Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: PrimaryExpression::Literal(and_node),
        });

        let expr = Expression::Binary(Node {
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
                    value: left_expr,
                }),
                right: Box::new(Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                        file_path: "test".to_string(),
                    },
                    value: right_expr,
                }),
                operator: Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                        file_path: "test".to_string(),
                    },
                    value: BinaryOperator::BitAnd,
                },
            },
        });

        let local_expr = LocalExpressionNode::from_expression(&expr, &vec![]).unwrap();
        assert_eq!(local_expr.eval(&Memory::new()).unwrap(), Literal::Int(2)); // 010 in binary
    }

    #[test]
    fn test_bitwise_or_expression() {
        let literal_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: Literal::Int(6), // 110 in binary
        };
        let or_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: Literal::Int(3), // 011 in binary
        };
        let left_expr = Expression::Primary(Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: PrimaryExpression::Literal(literal_node),
        });
        let right_expr = Expression::Primary(Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: PrimaryExpression::Literal(or_node),
        });

        let expression = Expression::Binary(Node {
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
                    value: left_expr,
                }),
                right: Box::new(Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                        file_path: "test".to_string(),
                    },
                    value: right_expr,
                }),
                operator: Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                        file_path: "test".to_string(),
                    },
                    value: BinaryOperator::BitOr,
                },
            },
        });
        let local_expr = LocalExpressionNode::from_expression(&expression, &vec![]).unwrap();
        assert_eq!(local_expr.eval(&Memory::new()).unwrap(), Literal::Int(7)); // 111 in binary
    }

    #[test]
    fn test_bitwise_operation_type_error() {
        let literal_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: Literal::Int(6),
        };
        let float_node = Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: Literal::Float(ordered_float::OrderedFloat(0.33)),
        };

        let left_expr = Expression::Primary(Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: PrimaryExpression::Literal(literal_node),
        });

        let right_expr = Expression::Primary(Node {
            pos: Pos {
                line: 0,
                col: 0,
                start: 0,
                end: 0,
                file_path: "test".to_string(),
            },
            value: PrimaryExpression::Literal(float_node),
        });

        let expr = Expression::Binary(Node {
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
                    value: left_expr,
                }),
                right: Box::new(Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                        file_path: "test".to_string(),
                    },
                    value: right_expr,
                }),
                operator: Node {
                    pos: Pos {
                        line: 0,
                        col: 0,
                        start: 0,
                        end: 0,
                        file_path: "test".to_string(),
                    },
                    value: BinaryOperator::BitAnd,
                },
            },
        });

        let local_expr = LocalExpressionNode::from_expression(&expr, &vec![]).unwrap();
        let err = local_expr.eval(&Memory::new()).unwrap_err();
        assert!(err.contains("Cannot perform bitwise AND between int and float"));
    }

    #[test]
    fn compile_extracts_globals_as_separate_reads() {
        let mut state = test_state();
        state.global_table.insert(
            "B".to_string(),
            Variable {
                mutable: false,
                name: "B".to_string(),
                datatype: DataType::Integer,
                depth: 0,
                declare_pos: None,
            },
        );
        state.global_table.insert(
            "C".to_string(),
            Variable {
                mutable: false,
                name: "C".to_string(),
                datatype: DataType::Integer,
                depth: 0,
                declare_pos: None,
            },
        );

        let expr = Node {
            pos: test_pos(),
            value: Expression::Binary(Node {
                pos: test_pos(),
                value: BinaryExpression {
                    left: Box::new(identifier_expression("B")),
                    right: Box::new(identifier_expression("C")),
                    operator: Node {
                        pos: test_pos(),
                        value: BinaryOperator::Add,
                    },
                },
            }),
        };

        let compiled = expr.compile(&mut state).unwrap();
        assert_eq!(compiled.instructions.len(), 3);
        assert!(matches!(
            &compiled.instructions[0].control,
            InstructionType::GlobalReads { variables, .. } if variables == &vec!["B".to_string()]
        ));
        assert!(matches!(
            &compiled.instructions[1].control,
            InstructionType::GlobalReads { variables, .. } if variables == &vec!["C".to_string()]
        ));
        assert!(matches!(
            &compiled.instructions[2].control,
            InstructionType::ExpressionAndCleanup { unstack_len, .. } if *unstack_len == 2
        ));
    }

    #[test]
    fn compile_tuple_with_run_extracts_run_before_final_eval() {
        let mut state = test_state();
        state
            .program_arguments
            .insert("A".to_string(), (Vec::new(), false));

        let expr = Node {
            pos: test_pos(),
            value: Expression::Tuple(Node {
                pos: test_pos(),
                value: TupleExpression {
                    values: vec![
                        int_expression(1),
                        Node {
                            pos: test_pos(),
                            value: Expression::RunCall(Box::new(Node {
                                pos: test_pos(),
                                value: RunCall {
                                    identifier: Node {
                                        pos: test_pos(),
                                        value: ObjectIdentifier {
                                            parts: vec![Node {
                                                pos: test_pos(),
                                                value: Identifier {
                                                    value: "A".to_string(),
                                                },
                                            }],
                                        },
                                    },
                                    args: Node {
                                        pos: test_pos(),
                                        value: Expression::Tuple(Node {
                                            pos: test_pos(),
                                            value: TupleExpression { values: vec![] },
                                        }),
                                    },
                                },
                            })),
                        },
                    ],
                },
            }),
        };

        let compiled = expr.compile(&mut state).unwrap();
        assert!(matches!(
            &compiled.instructions[0].control,
            InstructionType::Expression(LocalExpressionNode::Tuple(_))
        ));
        assert!(matches!(
            &compiled.instructions[1].control,
            InstructionType::RunCall { name, unstack_len }
            if name == "A" && *unstack_len == 1
        ));
        assert!(matches!(
            compiled.instructions.last().map(|i| &i.control),
            Some(InstructionType::MakeTupleAndCleanup { unstack_len, .. }) if *unstack_len == 1
        ));
    }
}
