pub mod binary_expression;
pub mod list_expression;
pub mod primary_expression;
pub mod tuple_expression;
pub mod unary_expression;

use std::{collections::HashSet, fmt};

use binary_expression::{BinaryExpression, LocalBinaryExpressionNode};
use list_expression::{LocalRangeListExpressionNode, RangeListExpression};
use primary_expression::{LocalPrimaryExpressionNode, LocalVarNode, PrimaryExpression};
use tuple_expression::{LocalTupleExpressionNode, TupleExpression};
use unary_expression::{LocalUnaryExpressionNode, UnaryExpression};

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node},
        token::{datatype::DataType, identifier::Identifier, literal::Literal},
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

impl Expression {}

fn tuple_arg_types(datatype: &DataType) -> Result<&[DataType], String> {
    match datatype {
        DataType::Tuple(types) => Ok(types),
        _ => Err("Method call expects tuple arguments".to_string()),
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
                let mut current_type = node.base.datatype(state)?;
                for (idx, segment) in node.segments.iter().enumerate() {
                    match segment {
                        LocalCallChainSegment::Call { name, args } => {
                            let method =
                                resolve_interface_method(&state.stdlib(), &current_type, name)?;
                            let args_type = args.datatype(state)?;
                            validate_interface_call(&method, tuple_arg_types(&args_type)?)?;
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

                            current_type = DataType::Boolean;
                        }
                    }
                }
                Ok(current_type)
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
                let mut current = node.base.eval_with_scope(mem, scope, vm)?;
                for segment in node.segments.iter() {
                    match segment {
                        LocalCallChainSegment::Call { name, args } => {
                            let mut args_value = args.eval_with_scope(mem, scope, vm)?;
                            current = LocalExpressionNode::evaluate_method_call(
                                vm,
                                &mut current,
                                name,
                                &mut args_value,
                                None,
                            )?;
                        }
                        LocalCallChainSegment::Reaches { label } => {
                            log::debug!(
                                "CallChain Reaches evaluation started for label '{}'",
                                label
                            );
                            let (program_name, pid) = match &current {
                                Literal::Process(name, pid) => {
                                    log::debug!("  Current is Process({}, {})", name, pid);
                                    (name.clone(), *pid)
                                }
                                _ => {
                                    log::debug!("  Current is not a Process: {:?}", current);
                                    current = Literal::Bool(false);
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
                                        current = Literal::Bool(true);
                                    } else {
                                        current = Literal::Bool(false);
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
                                current = Literal::Bool(false);
                                continue;
                            }

                            let program_code = match vm.programs_code.get(&program_name) {
                                Some(code) => code,
                                None => {
                                    log::debug!("  Program code for {} not found", program_name);
                                    current = Literal::Bool(false);
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
                                    current = Literal::Bool(false);
                                    continue;
                                }
                            };

                            let (_, pc, _) = prog_state.current_state();
                            let reached = pc == *label_pc;
                            log::debug!("  pc={}, label_pc={}, reached={}", pc, label_pc, reached);
                            current = Literal::Bool(reached);
                        }
                    }
                }
                Ok(current)
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
        } else if !local_expr.contains_fn_call() {
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
}
