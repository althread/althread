pub mod block;
pub mod condition_block;
pub mod import_block;
pub mod display;
pub mod node;
pub mod statement;
pub mod token;

use core::panic;
use std::{
    cell::RefCell, collections::{BTreeMap, HashMap, HashSet}, fmt::{self, Formatter}, path::Path, rc::Rc
};

use block::Block;
use condition_block::ConditionBlock;
use import_block::ImportBlock;
use display::{AstDisplay, Prefix};
use node::{InstructionBuilder, Node};
use pest::{iterators::Pairs};
use statement::Statement;
use token::{args_list::ArgsList, condition_keyword::ConditionKeyword, datatype::DataType, identifier::Identifier};

use crate::{
    analysis::control_flow_graph::ControlFlowGraph, ast::statement::{assignment::{Assignment}, expression::{primary_expression::PrimaryExpression, Expression, SideEffectExpression}}, compiler::{stdlib, CompilationContext, CompiledProject, CompilerState, FunctionDefinition, Variable}, error::{AlthreadError, AlthreadResult, ErrorType}, module_resolver::{module_resolver::ModuleResolver, FileSystem, StandardFileSystem}, no_rule, parser::{self, Rule}, vm::{
        instruction::{Instruction, InstructionType, ProgramCode},
        VM,
    }
};

#[derive(Debug)]
pub struct Ast {
    pub process_blocks: HashMap<String, (Node<ArgsList>, Node<Block>)>,
    pub condition_blocks: HashMap<ConditionKeyword, Node<ConditionBlock>>,
    pub global_block: Option<Node<Block>>,
    pub function_blocks: HashMap<String, (Node<ArgsList>, DataType, Node<Block>)>,
    pub import_block: Option<Node<ImportBlock>>
}

pub fn check_function_returns(func_name: &str,  func_body: &Node<Block>, return_type: &DataType) -> AlthreadResult<()> {
    if matches!(return_type, DataType::Void) {
        return Ok(());
    }

    let cfg = ControlFlowGraph::from_function(func_body);
    
    // display the control flow graph for debugging
    // cfg.display();


    // we need to return the function at line does not return a value
    // and say on which line it does not return a value
    
    if let Some(missing_return_pos) = cfg.find_first_missing_return_point(func_body.pos) {
        return Err(AlthreadError::new(
            ErrorType::FunctionMissingReturnStatement,
            Some(missing_return_pos), // Use the specific Pos found by the CFG analysis
            format!(
                "Function '{}' does not return a value on all code paths. Problem detected in construct starting at line {}.",
                func_name, missing_return_pos.line
            ),
        ));
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct ProcessListInfo {
    program_name: String, 
    element_type: String
}

impl Ast {
    pub fn new() -> Self {
        Self {
            process_blocks: HashMap::new(),
            condition_blocks: HashMap::new(),
            global_block: None,
            function_blocks: HashMap::new(),
            import_block: None,
        }
    }
    /// 
    pub fn build(pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let mut ast = Self::new();
        for pair in pairs {
            match pair.as_rule() {
                Rule::import_block => {
                    if ast.import_block.is_some() {
                        return Err(AlthreadError::new(
                            ErrorType::SyntaxError,
                            Some(pair.as_span().into()),
                            "Only one import block is allowed per file.".to_string(),
                        ));
                    }

                    let import_block = Node::build(pair)?;
                    ast.import_block = Some(import_block);
                }
                Rule::main_block => {
                    let mut pairs = pair.into_inner();

                    let main_block = Node::build(pairs.next().unwrap())?;
                    ast.process_blocks
                        .insert("main".to_string(), (Node::<ArgsList>::new(), main_block));
                }
                Rule::global_block => {
                    let mut pairs = pair.into_inner();

                    let global_block = Node::build(pairs.next().unwrap())?;
                    ast.global_block = Some(global_block);
                }
                Rule::condition_block => {
                    let mut pairs = pair.into_inner();

                    let keyword_pair = pairs.next().unwrap();
                    let condition_keyword = match keyword_pair.as_rule() {
                        Rule::ALWAYS_KW => ConditionKeyword::Always,
                        Rule::NEVER_KW => ConditionKeyword::Never,
                        Rule::EVENTUALLY_KW => ConditionKeyword::Eventually,
                        _ => return Err(no_rule!(keyword_pair, "condition keyword")),
                    };
                    let condition_block = Node::build(pairs.next().unwrap())?;
                    ast.condition_blocks
                        .insert(condition_keyword, condition_block);
                }
                Rule::program_block => {
                    let mut pairs = pair.into_inner();

                    let process_identifier = pairs.next().unwrap().as_str().to_string();
                    let args_list: Node<token::args_list::ArgsList> =
                        Node::build(pairs.next().unwrap())?;
                    let program_block = Node::build(pairs.next().unwrap())?;
                    ast.process_blocks
                        .insert(process_identifier, (args_list, program_block));
                }
                Rule::function_block => {
                    let mut pairs  = pair.into_inner();

                    let function_identifier = pairs.next().unwrap().as_str().to_string();
                    
                    let args_list: Node<token::args_list::ArgsList> = Node::build(pairs.next().unwrap())?;
                    pairs.next(); // skip the "->" token
                    let return_datatype = DataType::from_str(pairs.next().unwrap().as_str());
                    
                    let function_block: Node<Block>  = Node::build(pairs.next().unwrap())?;
                    
                    // check if function definition is already defined
                    if ast.function_blocks.contains_key(&function_identifier) {
                        return Err(AlthreadError::new(
                            ErrorType::FunctionAlreadyDefined,
                            Some(function_block.pos),
                            format!("Function '{}' is already defined", function_identifier),
                        ));
                    }

                    ast.function_blocks
                        .insert(
                        function_identifier,
                        (args_list, return_datatype, function_block)
                    );

                }
                Rule::EOI => (),
                _ => return Err(no_rule!(pair, "root ast")),
            }
        }

        Ok(ast)
    }


    fn extract_channel_declarations_from_statement(
        &self,
        statement: &Statement,
        state: &mut CompilerState,
        module_prefix: &str,
        var_to_program: &HashMap<String, String>,
    ) -> AlthreadResult<()> {
        // println!("Extracting channel declarations from statement: {:?}", statement);
        match statement {
            Statement::ChannelDeclaration(channel_decl) => {
                self.register_channel_declaration(&channel_decl.value, state, module_prefix, var_to_program)?;
            }
            Statement::Atomic(atomic_statement) => {
                self.extract_channel_declarations_from_statement(&atomic_statement.value.statement.value, state, module_prefix, var_to_program)?;
            }
            Statement::If(if_statement) => {
                self.extract_channel_declarations_from_block(&if_statement.value.then_block.value, state, module_prefix, var_to_program)?;
                if let Some(else_block) = &if_statement.value.else_block {
                    self.extract_channel_declarations_from_block(&else_block.value, state, module_prefix, var_to_program)?;
                }
            }
            Statement::Block(block) => {
                self.extract_channel_declarations_from_block(&block.value, state, module_prefix, var_to_program)?;
            }
            Statement::Loop(loop_statement) => {
                self.extract_channel_declarations_from_statement(&loop_statement.value.statement.value, state, module_prefix, var_to_program)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn register_channel_declaration(
        &self,
        channel_decl: &statement::channel_declaration::ChannelDeclaration,
        state: &mut CompilerState,
        module_prefix: &str,
        var_to_program: &HashMap<String, String>,
    ) -> AlthreadResult<()> {
        // Resolve program names for both sides of the channel
        let left_prog = self.prescan_get_prog_name(&channel_decl.ch_left_prog, module_prefix, var_to_program)?;
        let right_prog = self.prescan_get_prog_name(&channel_decl.ch_right_prog, module_prefix, var_to_program)?;

        // Create channel keys for both sender and receiver
        let left_key = (left_prog, channel_decl.ch_left_name.clone());
        let right_key = (right_prog, channel_decl.ch_right_name.clone());

        // Register the channel types - both sides get the same datatype info
        let pos = crate::error::Pos::default(); // We don't have position info during prescan
        state.channels_mut().insert(left_key.clone(), (channel_decl.datatypes.clone(), pos.clone()));
        state.channels_mut().insert(right_key.clone(), (channel_decl.datatypes.clone(), pos));

        // Remove from undefined channels if they exist
        state.undefined_channels_mut().remove(&left_key);
        state.undefined_channels_mut().remove(&right_key);

        Ok(())
    }

    fn prescan_get_prog_name(
        &self,
        var_name: &str,
        module_prefix: &str,
        var_to_program: &HashMap<String, String>,
    ) -> AlthreadResult<String> {
        if var_name == "self" {
            return Ok("main".to_string());
        }
        
        // Look up the variable in our mapping
        if let Some(program_name) = var_to_program.get(var_name) {
            if module_prefix.is_empty() {
                Ok(program_name.clone())
            } else {
                Ok(format!("{}.{}", module_prefix, program_name))
            }
        } else {
            Err(AlthreadError::new(
                ErrorType::VariableError,
                None,
                format!("Variable '{}' not found in run statements during prescan", var_name),
            ))
            // let res = if module_prefix.is_empty() {
            //     var_name.to_string()
            // } else {
            //     format!("{}.{}", module_prefix, var_name)
            // };
            // Ok(res) // Fallback to module prefix if not found
        }
    }

    fn extract_channel_declarations_from_block(
        &self,
        block: &Block,
        state: &mut CompilerState,
        module_prefix: &str,
        var_to_program: &HashMap<String, String>
    ) -> AlthreadResult<()> {
        for statement in &block.children {
            // println!("Extracting channel declarations from statement: {:?}", statement);
            self.extract_channel_declarations_from_statement(&statement.value, state, module_prefix, var_to_program)?;
        }
        Ok(())
    }

    fn build_variable_program_mapping(&self, var_to_program: &mut HashMap<String, String>) -> AlthreadResult<()> {
        let mut process_lists: HashMap<String, ProcessListInfo> = HashMap::new();

        // Scan all process blocks, not just main
        for (program_name, (_, program_block)) in &self.process_blocks {
            self.scan_block_for_run_statements(
                &program_block.value,
                var_to_program,
                &mut process_lists,
                program_name
            )?;
        }

        Ok(())
    }

    fn scan_block_for_run_statements(
        &self,
        block: &Block,
        var_to_program: &mut HashMap<String, String>,
        process_lists: &mut HashMap<String, ProcessListInfo>,
        current_program: &str
    ) -> AlthreadResult<()> {
        for statement in &block.children {
            self.scan_statement_for_run_statements(
                &statement.value, 
                var_to_program,
                process_lists,
                current_program
            )?;
        }
        Ok(())
    }

    fn scan_statement_for_run_statements(
        &self,
        statement: &Statement,
        var_to_program: &mut HashMap<String, String>,
        process_lists: &mut HashMap<String, ProcessListInfo>,
        current_program: &str
    ) -> AlthreadResult<()> {
        match statement {
            Statement::Declaration(var_decl) => {
                let var_name = &var_decl.value.identifier.value.parts[0].value.value;

                // Check if this is a list
                if let Some(list_type) = var_decl.value.datatype.as_ref() {
                    println!("Found list declaration: {} with type {:?}", var_name, list_type.value);

                    let (is_process, element_type) = list_type.value.is_process();
                    if is_process {
                        println!("Found process list declaration: {} with type {:?}", var_name, element_type);
                        process_lists.insert(
                            var_name.clone(),
                            ProcessListInfo { 
                                program_name: current_program.to_string(),
                                element_type: element_type
                            }
                        );
                    }
                }
                println!("process_lists: {:?}", process_lists.clone());

                // check for run calls
                if let Some(side_effect_node) = var_decl.value.value.as_ref() {
                    if let Some(program_name) = self.extract_run_program_name(&side_effect_node.value) {
                        var_to_program.insert(var_decl.value.identifier.value.parts[0].value.value.clone(), program_name);
                    }

                    // check for reference assignments (let b = a;)
                    else if let Some(ref_var) = self.extract_variable_reference(&side_effect_node.value) {
                        if let Some(program_type) = var_to_program.get(&ref_var) {
                            var_to_program.insert(var_name.clone(), program_type.clone());
                        }

                        if let Some(list_info) = process_lists.get(&ref_var).cloned() {
                            process_lists.insert(
                                var_name.clone(),
                                list_info
                            );
                        }
                    }

                    // check for .at() calls
                    else if let Some((list_var, _index)) = self.extract_list_at_call(&side_effect_node.value) {
                        if let Some(list_info) = process_lists.get(&list_var) {
                            var_to_program.insert(var_name.clone(), list_info.element_type.clone());
                        }
                    }
                }
            }
            Statement::Assignment(assignment) => {
                // handle assignments like: p1 = a.at(i);
                let Assignment::Binary(binary) = &assignment.value;
                let var_name = &binary.value.identifier.value.parts[0].value.value;
                
                // Get the right-hand side expression
                let rhs = &binary.value.value;

                // Check for reference assignment (p1 = b;)
                if let Some(ref_var) = self.extract_variable_reference(&rhs.value) {
                    if let Some(program_type) = var_to_program.get(&ref_var) {
                        var_to_program.insert(var_name.clone(), program_type.clone());
                    }
                    if let Some(list_info) = process_lists.get(&ref_var).cloned() {
                        process_lists.insert(var_name.clone(), list_info);
                    }
                }
                // Check for .at() calls (p1 = a.at(i);)
                else if let Some((list_var, _index)) = self.extract_list_at_call(&rhs.value) {
                    if let Some(list_info) = process_lists.get(&list_var) {
                        var_to_program.insert(var_name.clone(), list_info.element_type.clone());
                    }
                }
            }
            Statement::Atomic(atomic_statement) => {
                self.scan_statement_for_run_statements(&atomic_statement.value.statement.value, var_to_program, process_lists, current_program)?;
            }
            Statement::If(if_statement) => {
                self.scan_block_for_run_statements(&if_statement.value.then_block.value, var_to_program, process_lists, current_program)?;
                if let Some(else_block) = &if_statement.value.else_block {
                    self.scan_block_for_run_statements(&else_block.value, var_to_program, process_lists, current_program)?;
                }
            }
            Statement::Block(block) => {
                self.scan_block_for_run_statements(&block.value, var_to_program, process_lists, current_program)?;
            }
            Statement::For(for_statement) => {
                self.scan_statement_for_run_statements(&for_statement.value.statement.value, var_to_program, process_lists, current_program)?;
            }
            Statement::Loop(loop_statement) => {
                self.scan_statement_for_run_statements(&loop_statement.value.statement.value, var_to_program, process_lists, current_program)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn extract_variable_reference(&self, side_effect_expr: &SideEffectExpression) -> Option<String> {
        match side_effect_expr {
            SideEffectExpression::Expression(expr_node) => {
                if let Expression::Primary(primary_expr) = &expr_node.value {
                    if let PrimaryExpression::Identifier(identifier) = &primary_expr.value {
                        if identifier.value.parts.len() == 1 {
                            return Some(identifier.value.parts[0].value.value.clone());
                        } 
                    }
                }
            }
            _ => {},
        }
        None
    }

    fn extract_list_at_call(&self, side_effect_expr: &SideEffectExpression) -> Option<(String, String)> {
        match side_effect_expr {
            SideEffectExpression::FnCall(fn_call_node) => {
                let fn_call = &fn_call_node.value;

                if fn_call.fn_name.value.parts.len() == 2 {
                    let receiver_name = &fn_call.fn_name.value.parts[0].value.value;
                    let method_name = &fn_call.fn_name.value.parts[1].value.value;

                    if method_name == "at" {
                        return Some((receiver_name.clone(), "index".to_string()));
                    }
                }
            }
            _ => {},
        }
        None
    }

    fn extract_run_program_name(&self, side_effect_expr: &SideEffectExpression) -> Option<String> {
        match side_effect_expr {
            SideEffectExpression::RunCall(run_call_node) => {
                Some(run_call_node.value.program_name_to_string())
            }
            _ => None
        }
    }

    fn prescan_channel_declarations<F: FileSystem + Clone>(
    &self, 
    state: &mut CompilerState,
    current_file_path: &Path,
    filesystem: Option<F>
) -> AlthreadResult<()> {
    // Build variable-to-program mapping first
    let mut var_to_program: HashMap<String, String> = HashMap::new();
    self.build_variable_program_mapping(&mut var_to_program)?;

    // Scan ALL process blocks for channel declarations, not just main
    for (program_name, (_, program_block)) in &self.process_blocks {
        println!("Scanning program '{}' for channel declarations", program_name);
        self.extract_channel_declarations_from_block(
            &program_block.value, 
            state,
            "",
            &var_to_program
        )?;
        // println!("Finished scanning program '{:?}'", var_to_program);
    }

    // Handle imported modules
    if let Some(import_block) = &self.import_block {
        let mut module_resolver = ModuleResolver::new(current_file_path, filesystem.unwrap());
        let mut import_stack = Vec::new();
        module_resolver.resolve_imports(&import_block.value, &mut import_stack)?;

        for (name, resolved_module) in &module_resolver.resolved_modules {
            let module_content = module_resolver.filesystem.read_file(&resolved_module.path)?;
            let pairs = parser::parse(&module_content)?;
            let module_ast = Ast::build(pairs)?;

            let mut module_var_to_program: HashMap<String, String> = HashMap::new();
            module_ast.build_variable_program_mapping(&mut module_var_to_program)?;

            // Scan ALL process blocks in imported modules too
            for (program_name, (_, program_block)) in &module_ast.process_blocks {
                module_ast.extract_channel_declarations_from_block(
                    &program_block.value, 
                    state, 
                    name, 
                    &module_var_to_program
                )?;
            }
        }
    }

    Ok(())
    }
    

    pub fn compile<F: FileSystem + Clone>(
        &self,
        current_file_path: &Path,
        filesystem: F
    ) -> AlthreadResult<CompiledProject> {
        self.compile_internal(current_file_path, Some(filesystem), None, None, None)
    }

    fn compile_internal<F: FileSystem + Clone>(
        &self, 
        current_file_path: &Path,
        filesystem: Option<F>,
        stdlib: Option<Rc<stdlib::Stdlib>>,
        module_prefix: Option<&str>,
        existing_context: Option<&Rc<RefCell<CompilationContext>>>
    ) -> AlthreadResult<CompiledProject> {
        if filesystem.is_none() {
            // Delegate to the standard filesystem version
            return self.compile_internal(current_file_path, Some(StandardFileSystem), stdlib, None, existing_context);
        }

        let filesystem = filesystem.unwrap();

        println!("Compiling AST for file: {}", current_file_path.display());
        let stdlib = stdlib.unwrap_or_else(|| Rc::new(stdlib::Stdlib::new()));

        // Create shared compilation context
        let context = existing_context.cloned().unwrap_or_else(|| {
            Rc::new(RefCell::new(CompilationContext::new(Rc::clone(&stdlib))))
        });
        let mut state = CompilerState::new_with_context(Rc::clone(&context));

        if module_prefix.is_none() {
            self.prescan_channel_declarations(&mut state, current_file_path, Some(filesystem.clone()))?;
            println!("Prescan complete. Channels: {:?}", state.channels().clone());
        }


        if  self.process_blocks.is_empty() &&
            self.global_block.as_ref().map_or(true, |block| block.value.children.is_empty()) &&
            self.function_blocks.is_empty() &&
            self.import_block.is_none() &&
            self.condition_blocks.is_empty() {
                return Ok(CompiledProject {
                    global_memory: BTreeMap::new(),
                    program_arguments: HashMap::new(),
                    global_table: HashMap::new(),
                    user_functions: HashMap::new(),
                    programs_code: HashMap::new(),
                    always_conditions: Vec::new(),
                    eventually_conditions: Vec::new(),
                    stdlib: Rc::clone(&stdlib),
                });
            }

        if let Some(import_block) = &self.import_block {
            let mut module_resolver = ModuleResolver::new(current_file_path, filesystem.clone());
            let mut import_stack = Vec::new();

            module_resolver.resolve_imports(&import_block.value, &mut import_stack)?;

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

                let compiled_module = module_ast.compile_internal(&resolved_module.path, Some(filesystem.clone()), Some(Rc::clone(&stdlib)), Some(&name), Some(&Rc::clone(&context))).map_err(|e| {
                    AlthreadError::new(
                        ErrorType::SyntaxError,
                        Some(import_block.pos),
                        format!("Failed to compile module '{}': {:?}", resolved_module.name, e),
                    )
                })?;

                // shared variables - use context instead of local variables
                for (var_name, value) in compiled_module.global_memory {
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

                println!("global memory after importing module '{}': {:?}", name, state.global_memory());
                println!("global table after importing module '{}': {:?}", name, state.global_table());


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

                    // println!("function body for {}: {:?}", new_func_name, new_func_def.body);

                    state.user_functions_mut().insert(new_func_name.clone(), new_func_def);
                }

                for (prog_name, mut prog_code) in compiled_module.programs_code {
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

                    // println!("Adding program '{}' with arguments {:?}", qualified_prog_name, compiled_module.program_arguments.get(&prog_name));
                    state.program_arguments_mut().insert(
                        qualified_prog_name.clone(),
                        compiled_module.program_arguments.get(&prog_name)
                            .cloned()
                            .unwrap_or_default(),
                    );

                    // println!("Adding program '{}' with code {:?}", qualified_prog_name, prog_code);
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

            if let Err(e) = check_function_returns(&func_name,func_block, return_datatype){
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

        println!("always conditions: {:?}", state.always_conditions());
        println!("eventually conditions: {:?}", state.eventually_conditions());

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

        // println!("program arguments: {:?}", state.program_arguments());
        // println!("programs code: {:?}", state.programs_code());

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
        module_prefix: Option<&str>
    ) -> AlthreadResult<ProgramCode> {
        let mut process_code = ProgramCode {
            instructions: Vec::new(),
            name: name.to_string(),
        };
        let (args, prog) = self
            .process_blocks
            .get(name)
            .expect("trying to compile a non-existant program");
        state.current_program_name = if let Some(prefix) = module_prefix {
            format!("{}.{}", prefix, name)
        } else {
            name.to_string()
        };

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

impl fmt::Display for Ast {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.ast_fmt(f, &Prefix::new())
    }
}

impl AstDisplay for Ast {
    fn ast_fmt(&self, f: &mut Formatter, prefix: &Prefix) -> fmt::Result {
        if let Some(import_block) = &self.import_block {
            import_block.ast_fmt(f, prefix)?;
            writeln!(f, "")?;
        }

        if let Some(global_node) = &self.global_block {
            writeln!(f, "{}shared", prefix)?;
            global_node.ast_fmt(f, &prefix.add_branch())?;
        }

        writeln!(f, "")?;

        for (condition_name, condition_node) in &self.condition_blocks {
            writeln!(f, "{}{}", prefix, condition_name)?;
            condition_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
        }

        for (process_name, (_args, process_node)) in &self.process_blocks {
            writeln!(f, "{}{}", prefix, process_name)?;
            process_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
        }

        for (function_name, (_args, return_type, function_node)) in &self.function_blocks {
            writeln!(f, "{}{} -> {}", prefix, function_name, return_type)?;
            function_node.ast_fmt(f, &prefix.add_branch())?;
            writeln!(f, "")?;
        }

        Ok(())
    }
}
