use std::fmt::{self};

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        statement::{
            expression::{
                SideEffectExpression, 
                Expression,
                tuple_expression::TupleExpression
            },
            fn_call::FnCall
        },
        token::{
            binary_assignment_operator::BinaryAssignmentOperator,
            identifier::Identifier,
            object_identifier::ObjectIdentifier,
        },
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::{AlthreadError, AlthreadResult, ErrorType},
    parser::Rule,
    vm::instruction::{Instruction, InstructionType},
};

#[derive(Debug, Clone)]
pub enum AssignmentTarget {
    Identifier(Node<ObjectIdentifier>),
    IndexAccess {
        object: Node<ObjectIdentifier>,
        index: Node<Expression>,
    },
}

#[derive(Debug, Clone)]
pub struct BinaryAssignment {
    pub target: AssignmentTarget,
    pub operator: Node<BinaryAssignmentOperator>,
    pub value: Node<SideEffectExpression>,
}

impl NodeBuilder for BinaryAssignment {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let target_pair = pairs.next().unwrap();
        let operator = Node::build(pairs.next().unwrap(), filepath)?;
        let value = Node::build(pairs.next().unwrap(), filepath)?;

        let target = match target_pair.as_rule() {
            Rule::object_identifier => {
                AssignmentTarget::Identifier(Node::build(target_pair, filepath)?)
            }
            Rule::index_access => {
                let mut inner = target_pair.into_inner();
                let object: Node<ObjectIdentifier> = Node::build(inner.next().unwrap(), filepath)?;
                let index: Node<Expression> = Node::build(inner.next().unwrap(), filepath)?;
                AssignmentTarget::IndexAccess { object, index }
            }
            _ => return Err(crate::no_rule!(target_pair, "AssignmentTarget", filepath)),
        };

        Ok(Self {
            target,
            operator,
            value,
        })
    }
}

impl InstructionBuilder for Node<BinaryAssignment> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();

        // Get the full variable name (e.g., "fibo.N")
        match &self.value.target {
            AssignmentTarget::Identifier(identifier) => {
                let full_var_name = identifier
                    .value
                    .parts
                    .iter()
                    .map(|p| p.value.value.as_str())
                    .collect::<Vec<_>>()
                    .join(".");

                state.current_stack_depth += 1;
                builder.extend(self.value.value.compile(state)?);
                let rdatatype = state
                    .program_stack
                    .last()
                    .expect("empty stack after expression")
                    .datatype
                    .clone();
                let unstack_len = state.unstack_current_depth();

                if let Some(g_val) = state.global_table().get(&full_var_name) {
                    // Global variable assignment
                    if g_val.datatype != rdatatype {
                        return Err(AlthreadError::new(
                            ErrorType::TypeError,
                            Some(self.pos.clone()),
                            format!(
                                "Cannot assign value of type {} to variable of type {}",
                                rdatatype, g_val.datatype
                            ),
                        ));
                    }
                    if !g_val.mutable {
                        return Err(AlthreadError::new(
                            ErrorType::VariableError,
                            Some(self.pos.clone()),
                            format!(
                                "Cannot assign value to the immutable global variable {}",
                                full_var_name
                            ),
                        ));
                    }
                    builder.instructions.push(Instruction {
                        pos: Some(identifier.pos.clone()),
                        control: InstructionType::GlobalAssignment {
                            identifier: full_var_name,
                            operator: self.value.operator.value.clone(),
                            unstack_len,
                        },
                    });
                } else {
                    // Local variable assignment
                    let mut var_idx = 0;
                    let mut l_var = None;
                    for var in state.program_stack.iter().rev() {
                        if var.name == full_var_name {
                            l_var = Some(var);
                            break;
                        }
                        var_idx += 1;
                    }
                    if l_var.is_none() {
                        return Err(AlthreadError::new(
                            ErrorType::VariableError,
                            Some(self.pos.clone()),
                            format!("Variable '{}' is undefined", full_var_name),
                        ));
                    }
                    let l_var = l_var.unwrap();
                    if l_var.datatype != rdatatype {
                        return Err(AlthreadError::new(
                            ErrorType::TypeError,
                            Some(self.pos.clone()),
                            format!(
                                "Cannot assign value of type {} to variable of type {}",
                                rdatatype, l_var.datatype
                            ),
                        ));
                    }
                    if !l_var.mutable {
                        return Err(AlthreadError::new(
                            ErrorType::VariableError,
                            Some(self.pos.clone()),
                            format!(
                                "Cannot assign value to the immutable local variable {}",
                                full_var_name
                            ),
                        ));
                    }

                    builder.instructions.push(Instruction {
                        pos: Some(identifier.pos.clone()),
                        control: InstructionType::LocalAssignment {
                            index: var_idx,
                            operator: self.value.operator.value.clone(),
                            unstack_len,
                        },
                    });
                }
            }
            AssignmentTarget::IndexAccess { object, index } => {
                // Handle index assignment: obj[index] = value
                // Convert to: obj.set(index, value)

                // Build object.set as the function name
                let mut parts = object.value.parts.clone();
                parts.push(Node {
                    pos: self.pos.clone(),
                    value: Identifier {
                        value: "set".to_string(),
                    },
                });

                let fn_name = Node {
                    pos: self.pos.clone(),
                    value: ObjectIdentifier { parts },
                };

                // Create tuple expression (index, value)
                let index_expr = index.clone();

                let value_expr = match &self.value.value.value {
                    SideEffectExpression::Expression(expr) => expr.clone(),
                    _ => return Err(AlthreadError::new(
                        ErrorType::ExpressionError,
                        Some(self.value.value.pos.clone()),
                        "Value must be an expression".to_string(),
                    )),
                };

                let tuple_expr = Node {
                    pos: self.pos.clone(),
                    value: Expression::Tuple(Node {
                        pos: self.pos.clone(),
                        value: TupleExpression {
                            values: vec![index_expr, value_expr],
                        },
                    }),
                };

                let fn_call = Node {
                    pos: self.pos.clone(),
                    value: FnCall {
                        fn_name,
                        values: Box::new(tuple_expr),
                    },
                };

                // Compile the function call
                return fn_call.compile(state);
            }
        }

        Ok(builder)
    }
}

impl AstDisplay for BinaryAssignment {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{}binary_assign", prefix)?;

        let prefix = prefix.add_branch();
        match &self.target {
            AssignmentTarget::Identifier(identifier) => {
                let full_name = identifier
                    .value
                    .parts
                    .iter()
                    .map(|p| p.value.value.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                writeln!(f, "{}ident: {}", &prefix, full_name)?;
            }
            AssignmentTarget::IndexAccess { object, index: _ } => {
                let full_name = object
                    .value
                    .parts
                    .iter()
                    .map(|p| p.value.value.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                writeln!(f, "{}index_access: {}[...]", &prefix, full_name)?;
            }
        }
        writeln!(f, "{}op: {}", &prefix, self.operator)?;

        let prefix = prefix.switch();
        writeln!(f, "{}value:", &prefix)?;
        let prefix = prefix.add_leaf();
        self.value.ast_fmt(f, &prefix)?;
        Ok(())
    }
}
