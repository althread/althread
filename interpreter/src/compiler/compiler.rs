use std::{cell::RefCell, collections::{BTreeMap, HashMap, HashSet}, path::Path, rc::Rc};

use crate::{ast::{statement::Statement, token::{condition_keyword::ConditionKeyword, datatype::DataType, identifier::Identifier}, Ast, node::InstructionBuilder}, compiler::{stdlib::{self, Stdlib}, CompilationContext, CompiledProject, CompilerState, FunctionDefinition, Variable}, error::{AlthreadError, AlthreadResult, ErrorType}, module_resolver::{self, module_resolver::{ModuleResolver, ResolvedModule}, FileSystem}, parser, vm::{instruction::{Instruction, InstructionType, ProgramCode}, VM}};


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

    pub fn compile<F: FileSystem + Clone>(
        &self,
        current_file_path: &Path,
        filesystem: F
    ) -> AlthreadResult<CompiledProject> {
        
        // Create shared compilation context
        let context = Rc::new(RefCell::new(CompilationContext::new()));

        self.compile_with_context(current_file_path, filesystem, context, None)
    }

    pub fn compile_with_context<F: FileSystem + Clone>(
        &self,
        current_file_path: &Path,
        filesystem: F,
        context: Rc<RefCell<CompilationContext>>,
        module_prefix: Option<&String>,
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

        if let Some(prefix) = &module_prefix {
            state.module_prefix = Some(prefix.to_string());
        }

        // scan everything for channel declarations (in the main file and in all imports)
        if let Err(e) = self.prescan_channel_declarations(&mut state) {
            return Err(e);
        }

        self.compile_imports(current_file_path, filesystem.clone(), &mut state, context.clone())?;

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
            let code = self.compile_program("main", &mut state)?;
            state.programs_code_mut().insert("main".to_string(), code);
            assert!(state.current_stack_depth == 0);
        }

        for name in self.process_blocks.keys() {
            if name == "main" {
                continue;
            }
            let code = self.compile_program(name, &mut state)?;
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

            // clean up compiler state
            state.program_stack.truncate(initial_stack_len);
            state.current_stack_depth -= 1;
            state.in_function = false;


            let func_def = FunctionDefinition {
                name: func_name.clone(),
                arguments,
                return_type: return_datatype.clone(),
                body: compiled_body.instructions,
                pos: func_block.pos,
            };

            state.user_functions_mut().insert(func_name.clone(), func_def);

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

    fn compile_imports<F: FileSystem + Clone>(
        &self,
        current_file_path: &Path,
        filesystem: F,
        state: &mut CompilerState,
        context: Rc<RefCell<CompilationContext>>,
    ) -> AlthreadResult<()> {

        if let Some(import_block) = &self.import_block {
            let mut module_resolver = ModuleResolver::new(current_file_path, filesystem.clone());
            let mut import_stack = Vec::new();
            let mut same_level_module_names = Vec::<String>::new();

            module_resolver.resolve_imports(&import_block.value, &mut import_stack)?;

            for (name, _) in module_resolver.resolved_modules.clone() {
                same_level_module_names.push(name);
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

                let next_module_prefix = if let Some(prefix) = state.module_prefix.as_ref() {
                    format!("{}.{}", prefix, name)
                } else {
                    name.clone()
                };

                let compiled_module = module_ast.compile_with_context(&resolved_module.path, filesystem.clone(), context.clone(), Some(&next_module_prefix)).map_err(|e| {
                    AlthreadError::new(
                        ErrorType::SyntaxError,
                        Some(import_block.pos),
                        format!("Failed to compile module '{}': {:?}", resolved_module.name, e),
                    )
                })?;

                // shared variables - use context instead of local variables
                for (var_name, value) in compiled_module.global_memory {
                    // Skip if var_name is already qualified with a same-level module name
                    if same_level_module_names.iter().any(|mod_name| var_name.starts_with(&format!("{}.", mod_name))) {
                        continue;
                    }
                    let qualified_var_name = format!("{}.{}", name, var_name);
                    if state.global_memory().contains_key(&qualified_var_name) {
                        return Err(AlthreadError::new(
                            ErrorType::VariableAlreadyDefined,
                            Some(import_block.pos),
                            format!("Shared variable '{}' from module '{}' is already defined", var_name, name),
                        ));
                    }
                    state.global_memory_mut().insert(qualified_var_name.clone(), value.clone());
                    state.global_memory_mut().remove(&var_name);
                    if let Some(var_meta) = compiled_module.global_table.get(&var_name) {
                        let mut var_meta_cloned = var_meta.clone();
                        var_meta_cloned.name = qualified_var_name.clone();
                        state.global_table_mut().insert(qualified_var_name.clone(), var_meta_cloned);
                        state.global_table_mut().remove(&var_name);
                    }
                }

                for condition in compiled_module.always_conditions {
                    let (deps, read_vars, expr, pos) = condition;

                    let updated_deps: HashSet<String> = deps.iter()
                        .map(|dep| format!("{}.{}", name, dep))
                        .collect();
                    let updated_read_vars: Vec<String> = read_vars.iter()
                        .map(|var| format!("{}.{}", name, var))
                        .collect();

                    state.always_conditions_mut().push((
                        updated_deps, 
                        updated_read_vars,
                        expr,
                        pos
                    ))
                }

                for condition in compiled_module.eventually_conditions {
                    let (deps, read_vars, expr, pos) = condition;

                    let updated_deps: HashSet<String> = deps.iter()
                        .map(|dep| format!("{}.{}", name, dep))
                        .collect();
                    let updated_read_vars: Vec<String> = read_vars.iter()
                        .map(|var| format!("{}.{}", name, var))
                        .collect();

                    state.eventually_conditions_mut().push((
                        updated_deps, 
                        updated_read_vars,
                        expr,
                        pos
                    ))
                }

                // functions
                let imported_fn_names: std::collections::HashSet<String> = 
                    compiled_module.user_functions.keys().cloned().collect();

                for (func_name, func_def) in compiled_module.user_functions {
                    // Skip if func_name is already qualified with a same-level module name
                    if same_level_module_names.iter().any(|mod_name| func_name.starts_with(&format!("{}.", mod_name))) {
                        continue;
                    }
                    let new_func_name = format!("{}.{}", name, func_name);
                    if state.user_functions().contains_key(&new_func_name) {
                        return Err(AlthreadError::new(
                            ErrorType::FunctionAlreadyDefined,
                            Some(func_def.pos),
                            format!("Function '{}' from module '{}' is already defined", func_name, name),
                        ));
                    }
                    
                    let mut new_func_def = func_def.clone();
                    new_func_def.name = new_func_name.clone();

                    // replace all function calls in the body with the new function name
                    for instruction in &mut new_func_def.body {
                        match &mut instruction.control {
                            InstructionType::FnCall { name: call_name, ..} => {
                                if imported_fn_names.contains(call_name) {
                                    *call_name = format!("{}.{}", name, call_name);
                            }
                        }
                        InstructionType::GlobalReads { variables, ..} => {
                            for var in variables.iter_mut() {
                                *var = format!("{}.{}", name, var);
                            }
                        }
                        InstructionType::GlobalAssignment { identifier, ..} => {
                            *identifier = format!("{}.{}", name, identifier);
                        }
                        InstructionType::RunCall { name: call_name, ..} => {
                            *call_name = format!("{}.{}", name, call_name);
                        }
                        _ => {}
                        }
                    }

                    state.user_functions_mut().remove(&func_name);
                    state.user_functions_mut().insert(new_func_name.clone(), new_func_def);
                }

                for (prog_name, mut prog_code) in compiled_module.programs_code {
                    // Skip if prog_name is already qualified with a same-level module name
                    if same_level_module_names.iter().any(|mod_name| prog_name.starts_with(&format!("{}.", mod_name))) {
                        continue;
                    }

                    let qualified_prog_name = format!("{}.{}", name, prog_name);
                    if state.program_arguments().contains_key(&qualified_prog_name) {
                        return Err(AlthreadError::new(
                            ErrorType::ProgramAlreadyDefined,
                            Some(import_block.pos),
                            format!("Program '{}' from module '{}' is already defined", prog_name, name),
                        ));
                    }

                    // remove the unqualified program name from the arguments if it exists
                    state.program_arguments_mut().remove(&prog_name);

                    state.program_arguments_mut().insert(
                        qualified_prog_name.clone(),
                        compiled_module.program_arguments.get(&prog_name)
                            .cloned()
                            .unwrap_or_default(),
                    );

                    prog_code.name = qualified_prog_name.clone();
                    for instruction in &mut prog_code.instructions {
                        match &mut instruction.control {
                            InstructionType::FnCall { name: call_name, ..} => {
                                if imported_fn_names.contains(call_name) {
                                    *call_name = format!("{}.{}", name, call_name);
                                }
                            }
                            InstructionType::GlobalReads { variables, ..} => {
                                for var in variables.iter_mut() {
                                    *var = format!("{}.{}", name, var);
                                }
                            }
                            InstructionType::GlobalAssignment { identifier, ..} => {
                                *identifier = format!("{}.{}", name, identifier);
                            }
                            InstructionType::RunCall { name: call_name, ..} => {
                                *call_name = format!("{}.{}", name, call_name);
                            }
                            InstructionType::WaitStart { dependencies, ..} => {
                                let updated_vars: std::collections::HashSet<String> = dependencies.variables.iter()
                                    .map(|dep| format!("{}.{}", name, dep))
                                    .collect();
                                dependencies.variables = updated_vars;
                            }
                            _ => {}
                        }
                    }
                    state.programs_code_mut().remove(&prog_name);
                    state.programs_code_mut().insert(qualified_prog_name, prog_code);
                }
            }

            state.always_conditions_mut().retain(|(deps, _read_vars, _expr, _pos)| {
                deps.iter().all(|dep| {
                    let parts: Vec<&str> = dep.split('.').collect();
                    parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty()
                })
            });

            state.eventually_conditions_mut().retain(|(deps, _read_vars, _expr, _pos)| {
                deps.iter().all(|dep| {
                    let parts: Vec<&str> = dep.split('.').collect();
                    parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty()
                })
            });
        }
        Ok(())
    }

    fn compile_program(
        &self,
        name: &str,
        state: &mut CompilerState,
    ) -> AlthreadResult<ProgramCode> {
        let mut process_code = ProgramCode {
            instructions: Vec::new(),
            name: name.to_string(),
        };
        let (args, prog) = self
            .process_blocks
            .get(name)
            .expect("trying to compile a non-existant program");
        if let Some(prefix) = &state.module_prefix {
            state.current_program_name = format!("{}.{}", prefix, name);
        } else {
            state.current_program_name = name.to_string();
        }

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