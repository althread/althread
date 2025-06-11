use std::{collections::HashSet, fmt};

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::{datatype::DataType, identifier::Identifier},
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType},
    parser::Rule,
    vm::instruction::{Instruction, InstructionType},
};

use super::{expression::Expression, waiting_case::WaitDependency};

#[derive(Debug, Clone, PartialEq)]
pub struct FnCall {
    pub fn_name: Vec<Node<Identifier>>,
    pub values: Box<Node<Expression>>,
}

impl FnCall {
    pub fn add_dependencies(&self, dependencies: &mut WaitDependency) {
        for ident in &self.fn_name {
            dependencies.variables.insert(ident.value.value.clone());
        }

        self.values.value.add_dependencies(dependencies);
    }

    pub fn get_vars(&self, vars: &mut HashSet<String>) {
        for ident in &self.fn_name {
            vars.insert(ident.value.value.clone());
        }

        self.values.value.get_vars(vars);
    }
}

impl NodeBuilder for FnCall {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let mut object_identifier = pairs.next().unwrap();

        let mut fn_name = Vec::new();

        loop {
            let n: Node<Identifier> = Node::build(object_identifier.clone())?;
            fn_name.push(n);

            let mut pairs = object_identifier.into_inner();
            pairs.next().unwrap();
            if let Some(p) = pairs.next() {
                object_identifier = p;
            } else {
                break;
            }
        }

        let values = Box::new(Expression::build_top_level(pairs.next().unwrap())?);

        Ok(Self { fn_name, values })
    }
}

impl InstructionBuilder for Node<FnCall> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {

        let mut builder = InstructionBuilderOk::new();
        state.current_stack_depth += 1;


        builder.extend(self.value.values.compile(state)?);


        // normally it's always a tuple so it's always 1 argument
        // Tuple([]) when nothing is passed as argument
        let args_on_stack_var = 
            state.program_stack
            .last()
            .cloned()
            .expect("Stack should not be empty");

        // println!("args_on_stack_var: {:?}", args_on_stack_var);


        // get the function's basename (the last identifier in the fn_name)
        let basename = &self.value.fn_name[0].value.value;

        if self.value.fn_name.len() == 1 {

            if let Some(func_def) = state.user_functions.get(basename).cloned() {

                let expected_args = &func_def.arguments;
                let expected_arg_count = expected_args.len();

                // get the list of arguments (datatypes) from the tuple arg_list
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

            } else if basename == "print" {

                let unstack_len = state.unstack_current_depth();


                state.program_stack.push(Variable {
                    mutable: true,
                    name: "".to_string(),
                    datatype: DataType::Void,
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

            } else {

                return Err(AlthreadError::new(
                    ErrorType::UndefinedFunction,
                    Some(self.pos),
                    format!("undefined function {}", basename),
                ));
            }

        } else {
            // this is a method call

            //get the type of the variable in the stack with this name
            let var_id = state
                .program_stack
                .iter()
                .rev()
                .position(|var| var.name.eq(basename))
                .ok_or(AlthreadError::new(
                    ErrorType::VariableError,
                    Some(self.pos),
                    format!("Variable '{}' not found", basename),
                ))?;
            let var = &state.program_stack[state.program_stack.len() - var_id - 1];

            let interfaces = state.stdlib.interfaces(&var.datatype);

            // retreive the name of the function
            let fn_name = self.value.fn_name.last().unwrap().value.value.clone();

            let fn_idx = interfaces.iter().position(|i| i.name == fn_name);
            if fn_idx.is_none() {
                return Err(AlthreadError::new(
                    ErrorType::UndefinedFunction,
                    Some(self.pos),
                    format!("undefined function {}", fn_name),
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
                declare_pos: None,
            });

            builder.instructions.push(Instruction {
                control: InstructionType::FnCall {
                    name: fn_name,
                    unstack_len: unstack_len,
                    variable_idx: Some(var_id),
                    arguments: None, // use the top of the stack
                },
                pos: Some(self.pos),
            });
        }

        Ok(builder)
    }
}

impl AstDisplay for FnCall {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        let names: Vec<String> = self.fn_name
            .iter()
            .map(|n| n.value.value.clone())
            .collect();
        let fn_name = names.join(".");
        writeln!(f, "{}{}", prefix, fn_name)?;
        self.values.ast_fmt(f, &prefix.add_leaf())?;

        Ok(())
    }
}
