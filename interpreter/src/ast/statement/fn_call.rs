use std::{collections::HashSet, fmt};

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node},
        token::{datatype::DataType, object_identifier::ObjectIdentifier},
    },
    compiler::{
        stdlib::{resolve_interface_method, validate_interface_call},
        CompilerState, InstructionBuilderOk, Variable,
    },
    error::{AlthreadError, AlthreadResult, ErrorType},
    vm::instruction::{Instruction, InstructionType},
};

use super::{expression::Expression, waiting_case::WaitDependency};

#[derive(Debug, Clone, PartialEq)]
pub struct FnCall {
    pub fn_name: Node<ObjectIdentifier>, // Changed from Vec<Node<ObjectIdentifier>>
    pub values: Box<Node<Expression>>,
}

impl FnCall {
    pub fn fn_name_to_string(&self) -> String {
        self.fn_name
            .value
            .parts
            .iter()
            .map(|p| p.value.value.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn receiver_name(&self) -> Option<String> {
        if self.fn_name.value.parts.len() > 1 {
            Some(
                self.fn_name.value.parts[..self.fn_name.value.parts.len() - 1]
                    .iter()
                    .map(|part| part.value.value.as_str())
                    .collect::<Vec<_>>()
                    .join("."),
            )
        } else {
            None
        }
    }

    pub fn method_name(&self) -> Option<String> {
        self.fn_name
            .value
            .parts
            .last()
            .map(|part| part.value.value.clone())
    }

    pub fn add_dependencies(&self, dependencies: &mut WaitDependency) {
        if let Some(receiver_name) = self.receiver_name() {
            dependencies.variables.insert(receiver_name);
        }
        self.values.value.add_dependencies(dependencies);
    }

    pub fn get_vars(&self, vars: &mut HashSet<String>) {
        self.values.value.get_vars(vars);
    }
}

impl InstructionBuilder for Node<FnCall> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();
        let full_name = self.value.fn_name_to_string();
        state.current_stack_depth += 1;

        builder.extend(self.value.values.compile(state).map_err(|mut e| {
            e.push_stack(self.pos.clone());
            e
        })?);

        let args_on_stack_var = state
            .program_stack
            .last()
            .cloned()
            .expect("Stack should not be empty");

        // For single function names or fully qualified names
        let basename = if state.user_functions().contains_key(&full_name) {
            &full_name
        } else {
            // Get the first part for method calls or the full name for simple calls
            if self.value.fn_name.value.parts.len() == 1 {
                &self.value.fn_name.value.parts[0].value.value
            } else {
                &full_name
            }
        };

        if state.user_functions().contains_key(&full_name)
            || self.value.fn_name.value.parts.len() == 1
        {
            let func_def_opt = state.user_functions().get(basename).cloned();

            if let Some(func_def) = func_def_opt {
                let expected_args = &func_def.arguments;
                let expected_arg_count = expected_args.len();
                let provided_arg_types = args_on_stack_var.datatype.tuple_unwrap();

                // check if the number of arguments is correct
                if expected_arg_count != provided_arg_types.len() {
                    state.unstack_current_depth();
                    return Err(AlthreadError::new(
                        ErrorType::FunctionArgumentCountError,
                        Some(self.pos.clone()),
                        format!(
                            "Function '{}' expects {} arguments, but {} were provided.",
                            basename,
                            expected_arg_count,
                            provided_arg_types.len()
                        ),
                    ));
                }

                // check if the types of the arguments are correct
                for (i, ((_arg_name, expected_type), provided_type)) in expected_args
                    .iter()
                    .zip(provided_arg_types.iter())
                    .enumerate()
                {
                    if expected_type != provided_type {
                        state.unstack_current_depth();
                        return Err(AlthreadError::new(
                            ErrorType::FunctionArgumentTypeMismatch,
                            Some(self.pos.clone()),
                            format!(
                                "Function '{}' expects argument {} ('{}') to be of type {}, but got {}.",
                                basename,
                                i + 1,
                                expected_args[i].0.value, // argument name
                                expected_type,
                                provided_type
                            ),
                        ));
                    }
                }

                let unstack_len = state.unstack_current_depth();

                state.program_stack.push(Variable {
                    mutable: true,
                    name: "".to_string(),
                    datatype: func_def.return_type.clone(),
                    depth: state.current_stack_depth,
                    declare_pos: Some(self.pos.clone()),
                });

                builder.instructions.push(Instruction {
                    control: InstructionType::FnCall {
                        name: basename.to_string(),
                        unstack_len,
                        arguments: None,
                    },
                    pos: Some(self.pos.clone()),
                });
            } else {
                // Handle built-in functions like print, assert
                let return_type = match basename.as_str() {
                    "print" => {
                        let provided_arg_types = args_on_stack_var.datatype.tuple_unwrap();

                        for (idx, arg_type) in provided_arg_types.iter().enumerate() {
                            if *arg_type == DataType::Void {
                                state.unstack_current_depth();
                                return Err(AlthreadError::new(
                                    ErrorType::FunctionArgumentTypeMismatch,
                                    Some(self.pos.clone()),
                                    format!(
                                        "Function 'print' can't accept argument {} of type Void.",
                                        idx + 1
                                    ),
                                ));
                            }
                        }
                        DataType::Void
                    }
                    "assert" => {
                        let provided_arg_types = args_on_stack_var.datatype.tuple_unwrap();

                        if provided_arg_types.len() != 2 {
                            state.unstack_current_depth();
                            return Err(AlthreadError::new(
                                ErrorType::FunctionArgumentCountError,
                                Some(self.pos.clone()),
                                "Function 'assert' expects exactly 2 arguments.".to_string(),
                            ));
                        }

                        if provided_arg_types[0] != DataType::Boolean {
                            state.unstack_current_depth();
                            return Err(AlthreadError::new(
                                ErrorType::FunctionArgumentTypeMismatch,
                                Some(self.pos.clone()),
                                format!("Function 'assert' expects the first argument to be of type bool, but got {}.", provided_arg_types[0]),
                            ));
                        }

                        if provided_arg_types[1] != DataType::String {
                            state.unstack_current_depth();
                            return Err(AlthreadError::new(
                                ErrorType::FunctionArgumentTypeMismatch,
                                Some(self.pos.clone()),
                                format!("Function 'assert' expects the second argument to be of type string, but got {}.", provided_arg_types[1]),
                            ));
                        }
                        DataType::Void
                    }
                    _ => {
                        return Err(AlthreadError::new(
                            ErrorType::UndefinedFunction,
                            Some(self.pos.clone()),
                            format!("undefined function {}", basename),
                        ));
                    }
                };

                let unstack_len = state.unstack_current_depth();

                state.program_stack.push(Variable {
                    mutable: true,
                    name: "".to_string(),
                    datatype: return_type,
                    depth: state.current_stack_depth,
                    declare_pos: Some(self.pos.clone()),
                });

                builder.instructions.push(Instruction {
                    control: InstructionType::FnCall {
                        name: basename.to_string(),
                        unstack_len,
                        arguments: None,
                    },
                    pos: Some(self.pos.clone()),
                });
            }
        } else {
            // Handle method calls (e.g., obj.method())
            let receiver_name = self.value.receiver_name().ok_or(AlthreadError::new(
                ErrorType::UndefinedFunction,
                Some(self.pos.clone()),
                format!(
                    "Method call receiver is missing in {}",
                    self.value.fn_name_to_string()
                ),
            ))?;
            let method_name = self.value.method_name().ok_or(AlthreadError::new(
                ErrorType::UndefinedFunction,
                Some(self.pos.clone()),
                format!(
                    "Method name is missing in {}",
                    self.value.fn_name_to_string()
                ),
            ))?;

            let local_var_id = state
                .program_stack
                .iter()
                .rev()
                .position(|var| var.name == receiver_name);

            let (receiver_idx, global_receiver, receiver_type, receiver_is_mutable) =
                if let Some(raw_var_id) = local_var_id {
                    let var = &state.program_stack[state.program_stack.len() - 1 - raw_var_id];
                    (
                        raw_var_id + state.method_call_stack_offset,
                        None,
                        var.datatype.clone(),
                        var.mutable,
                    )
                } else if let Some(global_var) = state.global_table().get(&receiver_name) {
                    (
                        0,
                        Some(receiver_name.clone()),
                        global_var.datatype.clone(),
                        global_var.mutable,
                    )
                } else {
                    return Err(AlthreadError::new(
                        ErrorType::VariableError,
                        Some(self.pos.clone()),
                        format!("Variable '{}' not found", receiver_name),
                    ));
                };

            let fn_info = resolve_interface_method(&state.stdlib(), &receiver_type, &method_name)
                .map_err(|message| {
                AlthreadError::new(
                    ErrorType::UndefinedFunction,
                    Some(self.pos.clone()),
                    message,
                )
            })?;
            let ret_type = fn_info.ret.clone();

            let provided_arg_types = args_on_stack_var.datatype.tuple_unwrap();
            validate_interface_call(&fn_info, &provided_arg_types).map_err(|message| {
                AlthreadError::new(
                    ErrorType::FunctionArgumentTypeMismatch,
                    Some(self.pos.clone()),
                    message,
                )
            })?;

            if fn_info.mutates_receiver && !receiver_is_mutable {
                return Err(AlthreadError::new(
                    ErrorType::VariableError,
                    Some(self.pos.clone()),
                    format!(
                        "Cannot call mutating method '{}' on immutable global variable {}",
                        method_name, receiver_name
                    ),
                ));
            }

            let unstack_len = state.unstack_current_depth();

            state.program_stack.push(Variable {
                mutable: true,
                name: "".to_string(),
                datatype: ret_type,
                depth: state.current_stack_depth,
                declare_pos: Some(self.pos.clone()),
            });

            builder.instructions.push(Instruction {
                control: InstructionType::MethodCall {
                    name: method_name,
                    receiver_idx,
                    unstack_len,
                    drop_receiver: false,
                    arguments: None,
                    global_receiver,
                },
                pos: Some(self.pos.clone()),
            });
        }

        Ok(builder)
    }
}

impl AstDisplay for FnCall {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        let fn_name = self.fn_name_to_string();
        writeln!(f, "{}{}", prefix, fn_name)?;
        self.values.ast_fmt(f, &prefix.add_leaf())?;
        Ok(())
    }
}
