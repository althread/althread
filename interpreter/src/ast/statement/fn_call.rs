use std::{collections::HashSet, fmt};

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::{datatype::DataType, object_identifier::ObjectIdentifier},
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType},
    parser::Rule,
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
    
    pub fn add_dependencies(&self, dependencies: &mut WaitDependency) {
        let full_name = self.fn_name_to_string();
        dependencies.variables.insert(full_name);
        self.values.value.add_dependencies(dependencies);
    }

    pub fn get_vars(&self, vars: &mut HashSet<String>) {
        let full_name = self.fn_name_to_string();
        vars.insert(full_name);
        self.values.value.get_vars(vars);
    }
}

impl NodeBuilder for FnCall {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let fn_name = Node::<ObjectIdentifier>::build(pairs.next().unwrap())?;
        let values = Box::new(Expression::build_top_level(pairs.next().unwrap())?);

        Ok(Self { fn_name, values })
    }
}

impl InstructionBuilder for Node<FnCall> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();
        let full_name = self.value.fn_name_to_string();
        state.current_stack_depth += 1;

        builder.extend(self.value.values.compile(state)?);

        let args_on_stack_var = 
            state.program_stack
            .last()
            .cloned()
            .expect("Stack should not be empty");

        // For single function names or fully qualified names
        let basename = if state.user_functions.contains_key(&full_name) {
            &full_name
        } else {
            // Get the first part for method calls or the full name for simple calls
            if self.value.fn_name.value.parts.len() == 1 {
                &self.value.fn_name.value.parts[0].value.value
            } else {
                &full_name
            }
        };

        if state.user_functions.contains_key(&full_name) || self.value.fn_name.value.parts.len() == 1 {
            // Handle user-defined functions and built-in functions
            if let Some(func_def) = state.user_functions.get(basename).cloned() {

                let expected_args = &func_def.arguments;
                let expected_arg_count = expected_args.len();
                let provided_arg_types = args_on_stack_var.datatype.tuple_unwrap();

                // check if the number of arguments is correct
                if expected_arg_count != provided_arg_types.len() {

                    state.unstack_current_depth();

                    return Err(AlthreadError::new(
                        ErrorType::FunctionArgumentCountError,
                        Some(self.pos),
                        format!(
                            "Function '{}' expects {} arguments, but {} were provided.",
                            basename,
                            expected_arg_count,
                            provided_arg_types.len()
                        ),
                    ));
                }

                // check if the types of the arguments are correct
                for (i, ((_arg_name, expected_type), provided_type)) in expected_args.iter().zip(provided_arg_types.iter()).enumerate() {
                    if expected_type != provided_type {

                        state.unstack_current_depth(); 

                        return Err(AlthreadError::new(
                            ErrorType::FunctionArgumentTypeMismatch,
                            Some(self.pos),
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
                    declare_pos: Some(self.pos),
                });


                builder.instructions.push(Instruction {
                    control: InstructionType::FnCall { 
                        name: basename.to_string(), 
                        unstack_len, 
                        variable_idx: None, 
                        arguments: None 
                    },
                    pos: Some(self.pos),
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
                                    Some(self.pos),
                                    format!("Function 'print' can't accept argument {} of type Void.", idx + 1),
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
                                Some(self.pos),
                                "Function 'assert' expects exactly 2 arguments.".to_string(),
                            ));
                        }

                        if provided_arg_types[0] != DataType::Boolean {
                            state.unstack_current_depth();
                            return Err(AlthreadError::new(
                                ErrorType::FunctionArgumentTypeMismatch,
                                Some(self.pos),
                                format!("Function 'assert' expects the first argument to be of type bool, but got {}.", provided_arg_types[0]),
                            ));
                        }

                        if provided_arg_types[1] != DataType::String {
                            state.unstack_current_depth();
                            return Err(AlthreadError::new(
                                ErrorType::FunctionArgumentTypeMismatch,
                                Some(self.pos),
                                format!("Function 'assert' expects the second argument to be of type string, but got {}.", provided_arg_types[1]),
                            ));
                        }
                        DataType::Void
                    }
                    _ => {
                        return Err(AlthreadError::new(
                            ErrorType::UndefinedFunction,
                            Some(self.pos),
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
                    declare_pos: Some(self.pos),
                });

                builder.instructions.push(Instruction {
                    control: InstructionType::FnCall {
                        name: basename.to_string(),
                        unstack_len,
                        variable_idx: None,
                        arguments: None,
                    },
                    pos: Some(self.pos),
                });

            }

        } else {
            // Handle method calls (e.g., obj.method())
            let receiver_name = &self.value.fn_name.value.parts[0].value.value;
            let method_name = &self.value.fn_name.value.parts.last().unwrap().value.value;

            let raw_var_id = state
                .program_stack
                .iter()
                .rev()
                .position(|var| var.name.eq(receiver_name))
                .ok_or(AlthreadError::new(
                    ErrorType::VariableError,
                    Some(self.pos),
                    format!("Variable '{}' not found", receiver_name),
                ))?;

            let final_var_id = raw_var_id + state.method_call_stack_offset;
            let var = &state.program_stack[state.program_stack.len() - 1 - raw_var_id];
            let interfaces = state.stdlib.interfaces(&var.datatype);

            let fn_idx = interfaces.iter().position(|i| i.name == *method_name);
            if fn_idx.is_none() {
                return Err(AlthreadError::new(
                    ErrorType::UndefinedFunction,
                    Some(self.pos),
                    format!("undefined function {}", method_name),
                ));
            }
            let fn_idx = fn_idx.unwrap();
            let fn_info = &interfaces[fn_idx];
            let ret_type = fn_info.ret.clone();

            let unstack_len = state.unstack_current_depth();

            state.program_stack.push(Variable {
                mutable: true,
                name: "".to_string(),
                datatype: ret_type,
                depth: state.current_stack_depth,
                declare_pos: Some(self.pos),
            });

            builder.instructions.push(Instruction {
                control: InstructionType::FnCall {
                    name: method_name.clone(),
                    unstack_len: unstack_len,
                    variable_idx: Some(final_var_id),
                    arguments: None,
                },
                pos: Some(self.pos),
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
