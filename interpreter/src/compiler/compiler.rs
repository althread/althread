use std::{cell::RefCell, collections::{BTreeMap, HashMap, HashSet}, path::Path, rc::Rc};

use crate::{ast::{node::{InstructionBuilder}, statement::Statement, token::{condition_keyword::ConditionKeyword, datatype::DataType, identifier::Identifier, literal::Literal}, Ast}, compiler::{stdlib::{self}, CompilationContext, CompiledProject, CompilerState, FunctionDefinition, Variable}, error::{AlthreadError, AlthreadResult, ErrorType}, module_resolver::{module_resolver::{ModuleResolver}, FileSystem}, parser, vm::{instruction::{Instruction, InstructionType, ProgramCode}, VM}};


impl Ast {

    fn ast_empty(&self) -> bool {
        if  self.process_blocks.is_empty() &&
            self.global_block.as_ref().map_or(true, |block| block.value.children.is_empty()) &&
            self.function_blocks.is_empty() &&
            self.import_block.is_none() &&
            self.condition_blocks.is_empty() {
                return true;
            }
        false
    }

    pub fn build_qualified_name(
        &self, 
        name: &str,
        module_prefix: &str
    ) -> String {
        if module_prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", module_prefix, name)
        }
    }

    pub fn compile<F: FileSystem + Clone>(
        &self,
        current_file_path: &Path,
        filesystem: F
    ) -> AlthreadResult<CompiledProject> {
        
        // Create shared compilation context
        let context = Rc::new(RefCell::new(CompilationContext::new()));

        self.compile_with_context(current_file_path, filesystem, context, &"".to_string(), Vec::new())
    }

    pub fn compile_with_context<F: FileSystem + Clone>(
        &self,
        current_file_path: &Path,
        filesystem: F,
        context: Rc<RefCell<CompilationContext>>,
        module_prefix: &str,
        same_level_module_names: Vec<String>
    ) -> AlthreadResult<CompiledProject> {

        // if the main file is empty (no executable stuff) we have nothing to do
        if self.ast_empty() {
            return Ok(CompiledProject {
                global_memory: BTreeMap::new(),
                program_arguments: HashMap::new(),
                global_table: HashMap::new(),
                user_functions: HashMap::new(),
                programs_code: HashMap::new(),
                always_conditions: Vec::new(),
                eventually_conditions: Vec::new(),
                stdlib: Rc::new(stdlib::Stdlib::new()),
            });
        }

        let mut state= CompilerState::new_with_context(context.clone());

        log::debug!("Current module prefix: {:?}", module_prefix);
        log::debug!("[{}] Same level module names: {:?}", module_prefix, same_level_module_names);

        // scan everything for channel declarations in the current file
        if let Err(e) = self.prescan_channel_declarations(&mut state, module_prefix) {
            return Err(e);
        }

        let mut next_level_module_names = Vec::<String>::new();

        // if there's an import block, resolve the imports
        if let Some(import_block) = &self.import_block {
            
            let mut module_resolver = ModuleResolver::new(current_file_path, filesystem.clone());
            let mut import_stack = Vec::new();

            module_resolver.resolve_imports(&import_block.value, &mut import_stack)?;

            for (name, _) in module_resolver.resolved_modules.clone() {
                next_level_module_names.push(name);
            }

            for (name, resolved_module) in module_resolver.resolved_modules {
                let module_content = module_resolver.filesystem.read_file(&resolved_module.path)?;

                let pairs = parser::parse(&module_content).map_err(|e| {
                    AlthreadError::new(
                        ErrorType::SyntaxError,
                        Some(import_block.pos),
                        format!("Failed to parse module '{}': {:?}", resolved_module.name, e),
                    )
                })?;

                let module_ast = Ast::build(pairs).map_err(|e| {
                    AlthreadError::new(
                        ErrorType::SyntaxError,
                        Some(import_block.pos),
                        format!("Failed to build AST for module '{}': {:?}", resolved_module.name, e),
                    )
                })?;

                if module_ast.process_blocks.contains_key("main") {
                    return Err(AlthreadError::new(
                        ErrorType::ImportMainConflict,
                        Some(import_block.pos),
                        format!("'{}' defines a 'main' block. Imported modules cannot define a 'main' block.", resolved_module.name),
                    ));
                }

                let next_module_prefix: String = self.build_qualified_name(&name, module_prefix);

                let compiled_module = module_ast.compile_with_context(&resolved_module.path, filesystem.clone(), context.clone(), &next_module_prefix, next_level_module_names.clone()).map_err(|e| {
                    AlthreadError::new(
                        ErrorType::SyntaxError,
                        Some(import_block.pos),
                        format!("Failed to compile module '{}': {:?}", resolved_module.name, e),
                    )
                })?;

                // display what was imported at this point
                log::debug!("----------------------------------------------------");
                log::debug!("[{}] Imported module '{}'", module_prefix, name);
                log::debug!("[{}] Imported shared variables: {:?}", module_prefix, compiled_module.global_memory.keys());
                log::debug!("[{}] Imported functions: {:?}", module_prefix, compiled_module.user_functions.keys());
                log::debug!("[{}] Imported programs: {:?}", module_prefix, compiled_module.programs_code.keys());
                log::debug!("[{}] Imported always conditions: {:?}", module_prefix, compiled_module.always_conditions);
                log::debug!("[{}] Imported eventually conditions: {:?}", module_prefix, compiled_module.eventually_conditions);
                log::debug!("----------------------------------------------------");


                state.always_conditions_mut().extend(compiled_module.always_conditions);
                state.eventually_conditions_mut().extend(compiled_module.eventually_conditions);
                state.global_memory_mut().extend(compiled_module.global_memory);
                state.program_arguments_mut().extend(compiled_module.program_arguments);
                state.user_functions_mut().extend(compiled_module.user_functions);
                state.programs_code_mut().extend(compiled_module.programs_code);
                state.global_table_mut().extend(compiled_module.global_table);

            }

        }


         // "compile" the "shared" block to retrieve the set of shared variables
        state.current_stack_depth = 1;
        state.is_shared = true;
        if let Some(global) = self.global_block.as_ref() {
            let mut memory = VM::new_memory();
            for node in global.value.children.iter() {
                match &node.value {
                    Statement::Declaration(decl) => {
                        let mut literal = None;
                        let node_compiled = node.compile(&mut state)?;
                        for gi in node_compiled.instructions {
                            match gi.control {
                                InstructionType::Expression(exp) => {
                                    literal = Some(exp.eval(&memory).or_else(|err| {
                                        Err(AlthreadError::new(
                                            ErrorType::ExpressionError,
                                            gi.pos,
                                            err,
                                        ))
                                    })?);
                                }
                                InstructionType::Declaration { unstack_len } => {
                                    // do nothing
                                    assert!(unstack_len == 1)
                                }
                                InstructionType::Push(pushed_literal) => {
                                    literal = Some(pushed_literal)
                                }
                                _ => {
                                    panic!("unexpected instruction in compiled declaration statement")
                                }
                            }
                          }
                            let literal = literal
                                .expect("declaration did not compile to expression nor PushNull");
                            memory.push(literal);

                            let var_name = &decl.value.identifier.value.parts[0].value.value;
                            // Use context instead of local global_table
                            state.global_table_mut().insert(
                                var_name.clone(),
                                state.program_stack.last().unwrap().clone(),
                            );
                            state.global_memory_mut().insert(var_name.clone(), memory.last().unwrap().clone());
                    }
                    _ => {
                        return Err(AlthreadError::new(
                            ErrorType::InstructionNotAllowed,
                            Some(node.pos),
                            "The 'shared' block can only contains assignment from an expression"
                                .to_string(),
                        ))
                    }
                }
            }
        }

        log::debug!("[{}] Shared variables: {:?}", module_prefix, state.global_memory().keys());

        state.unstack_current_depth();
        assert!(state.current_stack_depth == 0);


        // functions baby ??
        // allow cross-function calls, recursive calls
        // this creates FunctionDefinitions without the compiled body, so that
        // compilation can be done no matter the order of the functions
        // or if they are recursive
        for (func_name, (args_list, return_datatype, func_block)) in &self.function_blocks {
            // check if the function is already defined
            if state.user_functions().contains_key(func_name) {
                return Err(AlthreadError::new(
                    ErrorType::FunctionAlreadyDefined,
                    Some(func_block.pos),
                    format!("Function '{}' is already defined", func_name),
                ));
            }
            // add the function to the user functions
            let arguments: Vec<(Identifier, DataType)> = args_list.value
                .identifiers
                .iter()
                .zip(args_list.value.datatypes.iter())
                .map(|(id, dt)| (id.value.clone(), dt.value.clone()))
                .collect();

            let func_def = FunctionDefinition {
                name: func_name.clone(),
                arguments: arguments.clone(),
                return_type: return_datatype.clone(),
                body: Vec::new(),
                pos: func_block.pos,
            };

            if let Err(e) = Ast::check_function_returns(&func_name,func_block, return_datatype){
                return Err(e);
            }

            state.user_functions_mut().insert(func_name.clone(), func_def);
        }

        // before compiling the programs, get the list of program names and their arguments
        let program_args: HashMap<String, Vec<DataType>> = self
            .process_blocks
            .iter()
            .map(|(name, (args, _))| {
                (
                    name.clone(),
                    args.value
                        .datatypes
                        .iter()
                        .map(|d| d.value.clone())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        
        // Update context instead of state
        state.program_arguments_mut().extend(program_args);

        // Compile all the programs
        state.is_shared = false;
        
        // start with the main program
        if self.process_blocks.contains_key("main") {
            let code = self.compile_program("main", &mut state, module_prefix)?;
            state.programs_code_mut().insert("main".to_string(), code);
            assert!(state.current_stack_depth == 0);
        }

        for name in self.process_blocks.keys() {
            if name == "main" {
                continue;
            }
            let code = self.compile_program(name, &mut state, module_prefix)?;
            state.programs_code_mut().insert(name.clone(), code);
            assert!(state.current_stack_depth == 0);
        }

        // check if all the channels used have been declared
        for (channel_name, (_, pos)) in state.undefined_channels().iter() {
            return Err(AlthreadError::new(
                ErrorType::UndefinedChannel,
                Some(pos.clone()),
                format!(
                    "Channel '{}' used in program '{}' at line {} has not been declared",
                    channel_name.1, channel_name.0, pos.line
                ),
            ));
        }

        for (name, condition_block) in self.condition_blocks.iter() {
            match name {
                ConditionKeyword::Always => {
                    for condition in condition_block.value.children.iter() {
                        let compiled = condition.compile(&mut state)?.instructions;
                        if compiled.len() == 1 {
                            return Err(AlthreadError::new(
                                ErrorType::InstructionNotAllowed,
                                Some(condition.pos),
                                "The condition must depend on shared variable(s)".to_string(),
                            ));
                        }
                        if compiled.len() != 2 {
                            return Err(AlthreadError::new(
                                ErrorType::InstructionNotAllowed,
                                Some(condition.pos),
                                "The condition must be a single expression".to_string(),
                            ));
                        }
                        if let InstructionType::GlobalReads { variables, .. } = &compiled[0].control
                        {
                            if let InstructionType::Expression(exp) = &compiled[1].control {
                                state.always_conditions_mut().push((
                                    variables.iter().map(|s| s.clone()).collect(),
                                    variables.clone(),
                                    exp.clone(),
                                    condition.pos,
                                ));
                            } else {
                                return Err(AlthreadError::new(
                                    ErrorType::InstructionNotAllowed,
                                    Some(condition.pos),
                                    "The condition must be a single expression".to_string(),
                                ));
                            }
                        } else {
                            return Err(AlthreadError::new(
                                ErrorType::InstructionNotAllowed,
                                Some(condition.pos),
                                "The condition must depend on shared variable(s)".to_string(),
                            ));
                        }
                    }
                }
                // TODO  since the content is sensitively similar to always block find a way to combine both to avoid code duplication
                ConditionKeyword::Eventually => {
                    for condition in condition_block.value.children.iter() {
                        let compiled = condition.compile(&mut state)?.instructions;
                        if compiled.len() == 1 {
                            return Err(AlthreadError::new(
                                ErrorType::InstructionNotAllowed,
                                Some(condition.pos),
                                "The condition must depend on shared variable(s)".to_string(),
                            ));
                        }
                        if compiled.len() != 2 {
                            return Err(AlthreadError::new(
                                ErrorType::InstructionNotAllowed,
                                Some(condition.pos),
                                "The condition must be a single expression".to_string(),
                            ));
                        }
                        if let InstructionType::GlobalReads { variables, .. } = &compiled[0].control
                        {
                            if let InstructionType::Expression(exp) = &compiled[1].control {
                                state.eventually_conditions_mut().push((
                                    variables.iter().map(|s| s.clone()).collect(),
                                    variables.clone(),
                                    exp.clone(),
                                    condition.pos,
                                ));
                            } else {
                                return Err(AlthreadError::new(
                                    ErrorType::InstructionNotAllowed,
                                    Some(condition.pos),
                                    "The condition must be a single expression".to_string(),
                                ));
                            }
                        } else {
                            return Err(AlthreadError::new(
                                ErrorType::InstructionNotAllowed,
                                Some(condition.pos),
                                "The condition must depend on shared variable(s)".to_string(),
                            ));
                        }
                    }
                }
                _ => {}
            }
        
        }

        // now compile the function bodies
        for (func_name, (args_list, return_datatype, func_block)) in &self.function_blocks {

            state.in_function = true;
            state.current_stack_depth += 1;
            let initial_stack_len = state.program_stack.len();

            let arguments: Vec<(Identifier, DataType)> = args_list.value
                .identifiers
                .iter()
                .zip(args_list.value.datatypes.iter())
                .map(|(id, dt)| {
                    // add the arguments to the stack
                    state.program_stack.push(Variable {
                        name: id.value.value.clone(),
                        depth: state.current_stack_depth,
                        mutable: true,
                        datatype: dt.value.clone(),
                        declare_pos: Some(id.pos),
                    });
                    (id.value.clone(), dt.value.clone())
                })
                .collect();


            // compile the function body
            let mut compiled_body = func_block.compile(&mut state)?;
            
            // if the function's return datatype is Void
            if *return_datatype == DataType::Void {
                let mut has_return = false;
                // check if it has a return instruction as the last instruction
                match compiled_body.instructions.last() {
                    Some(last_instruction) => {
                        if let InstructionType::Return { has_value: false } = &last_instruction.control {
                            has_return = true;
                        }
                    }
                    None => {}
                }
                // if it does not have a return instruction, add one
                if !has_return {
                    compiled_body.instructions.push(
                        Instruction {
                            control: InstructionType::Return {
                                has_value: false,
                            },
                            pos: Some(func_block.pos),
                        },
                    );
                }
            }

            state.user_functions_mut().insert(
                func_name.clone(),
                FunctionDefinition {
                    name: func_name.clone(),
                    arguments,
                    return_type: return_datatype.clone(),
                    body: compiled_body.instructions,
                    pos: func_block.pos,
                },
            );

            // clean up compiler state
            state.program_stack.truncate(initial_stack_len);
            state.current_stack_depth -= 1;
            state.in_function = false;

        }

        // if not the main file then we need to qualify everything
        if !module_prefix.is_empty() {
            for (func_name, (_, return_datatype, func_block )) in &self.function_blocks {
                
                if same_level_module_names.iter().any(|mod_name| func_name.starts_with(&format!("{}.", mod_name))) {
                        log::debug!("[{}] Skipping function '{}' as it is already qualified", module_prefix, func_name);
                        continue;
                }

                let old_func_def_opt = state.user_functions_mut().remove(func_name);

                if old_func_def_opt.is_none() {
                    return Err(AlthreadError::new(
                        ErrorType::FunctionNotFound,
                        Some(func_block.pos),
                        format!("Function '{}' not found", func_name),
                    ));
                }

                let mut old_func_def = old_func_def_opt.unwrap();

                let new_func_name = self.build_qualified_name(&func_name, module_prefix);

                log::debug!("[{}] Importing function '{}'", module_prefix, func_name);
                if state.user_functions().contains_key(&new_func_name) {
                    return Err(AlthreadError::new(
                        ErrorType::FunctionAlreadyDefined,
                        Some(old_func_def.pos),
                        format!("Function '{}' from module '{}' is already defined", func_name, module_prefix),
                    ));
                }

                // replace all function calls in the body with the new function name
                for instruction in &mut old_func_def.body {
                    match &mut instruction.control {
                    InstructionType::FnCall { name: call_name, ..} => {
                        if call_name == "print" || call_name == "assert" {
                            // do not qualify standard library function calls
                            continue;
                        }
                        *call_name = format!("{}.{}", module_prefix, call_name);
                    }
                    InstructionType::GlobalReads { variables, ..} => {
                        for var in variables.iter_mut() {
                            *var = format!("{}.{}", module_prefix, var);
                        }
                    }
                    InstructionType::GlobalAssignment { identifier, ..} => {
                        *identifier = format!("{}.{}", module_prefix, identifier);
                    }
                    InstructionType::RunCall { name: call_name, ..} => {
                        *call_name = format!("{}.{}", module_prefix, call_name);
                    }
                    _ => {}
                    }
                }

                let func_def = FunctionDefinition {
                    name: new_func_name.clone(),
                    arguments: old_func_def.arguments.clone(),
                    return_type: return_datatype.clone(),
                    body: old_func_def.body.clone(),
                    pos: func_block.pos,
                };

                state.user_functions_mut().insert(new_func_name.clone(), func_def);
            }


            log::debug!("[{}] Module importing shared variables", module_prefix);
            log::debug!("Global memory: {:?}", state.global_memory_mut().keys());

            // Collect all variable data first to avoid borrow conflicts
            let shared_vars: Vec<(String, Literal, Option<Variable>)> = {
                let global_memory = state.global_memory();
                let global_table = state.global_table();
                
                global_memory.iter().map(|(var_name, value)| {
                    let var_meta = global_table.get(var_name).cloned();
                    (var_name.clone(), value.clone(), var_meta)
                }).collect()
            };

            log::debug!("[{}] Shared variables: {:?}", module_prefix, shared_vars.iter().map(|(name, _, _)| name).collect::<Vec<_>>());

            for (var_name, value, var_meta) in shared_vars {
                let qualified_var_name = self.build_qualified_name(&var_name, module_prefix);
                
                if qualified_var_name == var_name || var_name.contains(module_prefix) || same_level_module_names.iter().any(|mod_name| var_name.starts_with(&format!("{}.", mod_name))) {
                        log::debug!("[{}] Skipping variable '{}' as it is already qualified", module_prefix, var_name);
                        continue;
                }
                
                log::debug!("[{}] Importing shared variable '{}'", module_prefix, qualified_var_name);

                if state.global_memory().contains_key(&qualified_var_name) {
                    return Err(AlthreadError::new(
                        ErrorType::VariableAlreadyDefined,
                        None,
                        format!("Shared variable '{}' from module '{}' is already defined", var_name, module_prefix),
                    ));
                }
                
                state.global_memory_mut().insert(qualified_var_name.clone(), value);
                state.global_memory_mut().remove(&var_name);
                
                if let Some(var_meta) = var_meta {
                    let mut var_meta_cloned = var_meta.clone();
                    var_meta_cloned.name = qualified_var_name.clone();
                    state.global_table_mut().insert(qualified_var_name.clone(), var_meta_cloned);
                    state.global_table_mut().remove(&var_name);
                }
            }

            log::debug!("[{}] Shared variables after import: {:?}", module_prefix, state.global_memory().keys());

            log::debug!("[{}] Qualifying always conditions", module_prefix);
            // Collect conditions into a temporary vector to avoid borrow conflicts
            let always_conditions: Vec<_> = state.always_conditions().clone();
            let mut new_always_conditions = Vec::new();
            for condition in always_conditions.iter() {
                // if the condition is already qualified, skip it
                if condition.0.iter().any(|dep| dep.contains(module_prefix)) {
                    log::debug!("[{}] Skipping condition as it is already qualified", module_prefix);
                    continue;
                }

                // skip if any dependency starts with any same-level module name
                if same_level_module_names.iter().any(|mod_name| {
                    condition.0.iter().any(|dep| dep.starts_with(&format!("{}.", mod_name)))
                }) {
                    log::debug!("[{}] Skipping condition as it starts with a same-level module name", module_prefix);
                    continue;
                }
                
                let (deps, read_vars, expr, pos) = condition;

                let updated_deps: HashSet<String> = deps.iter()
                    .map(|dep| format!("{}.{}", module_prefix, dep))
                    .collect();
                let updated_read_vars: Vec<String> = read_vars.iter()
                    .map(|var| format!("{}.{}", module_prefix, var))
                    .collect();

                new_always_conditions.push((
                    updated_deps,
                    updated_read_vars,
                    expr.clone(),
                    pos.clone()
                ));
            }
            *state.always_conditions_mut() = new_always_conditions;

            log::debug!("[{}] Qualifying eventually conditions", module_prefix);
            // Do the same for eventually_conditions
            let eventually_conditions: Vec<_> = state.eventually_conditions().clone();
            let mut new_eventually_conditions = Vec::new();
            for condition in eventually_conditions.iter() {
                // if the condition is already qualified, skip it
                if condition.0.iter().any(|dep| dep.contains(module_prefix)) {
                    continue;
                }

                // skip if any dependency starts with any same-level module name
                if same_level_module_names.iter().any(|mod_name| {
                    condition.0.iter().any(|dep| dep.starts_with(&format!("{}.", mod_name)))
                }) {
                    continue;
                }
                let (deps, read_vars, expr, pos) = condition;

                let updated_deps: HashSet<String> = deps.iter()
                    .map(|dep| format!("{}.{}", module_prefix, dep))
                    .collect();
                let updated_read_vars: Vec<String> = read_vars.iter()
                    .map(|var| format!("{}.{}", module_prefix, var))
                    .collect();

                new_eventually_conditions.push((
                    updated_deps,
                    updated_read_vars,
                    expr.clone(),
                    pos.clone()
                ));
            }
            *state.eventually_conditions_mut() = new_eventually_conditions;

            state.always_conditions_mut().retain(|(deps, _read_vars, _expr, _pos)| {
                deps.iter().any(|dep| dep.contains('.'))
            });

            state.eventually_conditions_mut().retain(|(deps, _read_vars, _expr, _pos)| {
                deps.iter().any(|dep| dep.contains('.'))
            });

            log::debug!("[{}] Qualifying programs", module_prefix);
            // Collect programs to update first to avoid borrow conflicts
            let programs_to_update: Vec<(String, ProgramCode)> = state.programs_code()
            .iter()
            .map(|(prog_name, prog_code)| (prog_name.clone(), prog_code.clone()))
            .collect();

            for (prog_name, mut prog_code) in programs_to_update {
                if prog_name.contains(module_prefix) {
                    // This program is already qualified, skip it
                    continue;
                }

                if same_level_module_names.iter().any(|mod_name| prog_name.starts_with(&format!("{}.", mod_name))) {
                        log::debug!("[{}] Skipping program '{}' as it is already qualified", module_prefix, prog_name);
                        continue;
                }
                
                let qualified_prog_name = self.build_qualified_name(&prog_name, module_prefix);

                if qualified_prog_name == prog_name {
                    // No need to qualify, it's already qualified
                    continue;
                }

                log::debug!("[{}] Importing program '{}'", module_prefix, qualified_prog_name);
                if state.program_arguments().contains_key(&qualified_prog_name) {
                    return Err(AlthreadError::new(
                        ErrorType::ProgramAlreadyDefined,
                        Some(self.import_block.as_ref().unwrap().pos),
                        format!("Program '{}' from module '{}' is already defined", prog_name, module_prefix),
                    ));
                }

                let prog_args = state.program_arguments().get(&prog_name)
                .cloned()
                .unwrap_or_default();

                state.program_arguments_mut().insert(
                    qualified_prog_name.clone(),
                    prog_args,
                );

                prog_code.name = qualified_prog_name.clone();
                for instruction in &mut prog_code.instructions {
                    match &mut instruction.control {
                        InstructionType::FnCall { name: call_name, ..} => {
                                if call_name == "print" || call_name == "assert" {
                                    // do not qualify standard library function calls
                                    continue;
                                }
                            *call_name = format!("{}.{}", module_prefix, call_name);
                        }
                        InstructionType::GlobalReads { variables, ..} => {
                            for var in variables.iter_mut() {
                                *var = format!("{}.{}", module_prefix, var);
                            }
                        }
                        InstructionType::GlobalAssignment { identifier, ..} => {
                            *identifier = format!("{}.{}", module_prefix, identifier);
                        }
                        InstructionType::RunCall { name: call_name, ..} => {
                            *call_name = format!("{}.{}", module_prefix, call_name);
                        }
                        InstructionType::WaitStart { dependencies, ..} => {
                            let updated_vars: std::collections::HashSet<String> = dependencies.variables.iter()
                                .map(|dep| format!("{}.{}", module_prefix, dep))
                                .collect();
                            dependencies.variables = updated_vars;
                        }
                        _ => {}
                    }
                }
                state.programs_code_mut().remove(&prog_name);
                state.programs_code_mut().insert(qualified_prog_name, prog_code.clone());
            }
        }

        state.context.borrow_mut().global_memory.extend(state.global_memory().clone());
        state.context.borrow_mut().program_arguments.extend(state.program_arguments().clone());
        state.context.borrow_mut().user_functions.extend(state.user_functions().clone());
        state.context.borrow_mut().global_table.extend(state.global_table().clone());
        state.context.borrow_mut().programs_code.extend(state.programs_code().clone());
        state.context.borrow_mut().always_conditions.extend(state.always_conditions().clone());
        state.context.borrow_mut().eventually_conditions.extend(state.eventually_conditions().clone());

        log::debug!("[{}] Compiled module with {} programs, {} functions, {} shared variables, {} always conditions, {} eventually conditions", 
            module_prefix, 
            state.programs_code().len(), 
            state.user_functions().len(), 
            state.global_memory().len(),
            state.always_conditions().len(),
            state.eventually_conditions().len()
        );

        if module_prefix.is_empty() {
            log::debug!("-----------------------------------------------------");
            log::debug!("[Main module] Finished compiling main module");
            log::debug!("[Main module] Shared variables: {:?}", state.global_memory().keys());
            log::debug!("[Main module] User functions: {:?}", state.user_functions().keys());
            log::debug!("[Main module] Programs code: {:?}", state.programs_code().keys());
            log::debug!("[Main module] Always conditions: {:?}", state.always_conditions());
            log::debug!("[Main module] Eventually conditions: {:?}", state.eventually_conditions());
            log::debug!("-----------------------------------------------------");
        }

        // Return using context data instead of local variables
        let context_borrow = context.borrow();
        Ok(CompiledProject {
            global_memory: context_borrow.global_memory.clone(),
            program_arguments: context_borrow.program_arguments.clone(),
            user_functions: context_borrow.user_functions.clone(),
            global_table: context_borrow.global_table.clone(),
            programs_code: context_borrow.programs_code.clone(),
            always_conditions: context_borrow.always_conditions.clone(),
            eventually_conditions: context_borrow.eventually_conditions.clone(),
            stdlib: context_borrow.stdlib.clone(),
        })

    }


    fn compile_program(
        &self,
        name: &str,
        state: &mut CompilerState,
        module_prefix: &str
    ) -> AlthreadResult<ProgramCode> {
        let mut process_code = ProgramCode {
            instructions: Vec::new(),
            name: name.to_string(),
        };
        let (args, prog) = self
            .process_blocks
            .get(name)
            .expect("trying to compile a non-existant program");

        state.current_program_name = self.build_qualified_name(name, module_prefix);

        for (i, var) in args.value.identifiers.iter().enumerate() {
            state.program_stack.push(Variable {
                name: var.value.value.clone(),
                depth: state.current_stack_depth,
                mutable: true,
                datatype: args.value.datatypes[i].value.clone(),
                declare_pos: Some(var.pos),
            });
        }

        let compiled = prog.compile(state)?;
        if compiled.contains_jump() {
            unimplemented!("breaks or return statements in programs are not yet implemented");
        }
        if !args.value.identifiers.is_empty() {
            process_code.instructions.push(Instruction {
                control: InstructionType::Destruct,
                pos: Some(args.pos),
            });
        }
        process_code.instructions.extend(compiled.instructions);
        process_code.instructions.push(Instruction {
            control: InstructionType::EndProgram,
            pos: Some(prog.pos),
        });
        Ok(process_code)
    }

}