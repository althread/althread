use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
    rc::Rc,
};

use crate::{
    ast::{
        node::{InstructionBuilder, Node},
        statement::{
            expression::{
                BracketContent, BracketExpression, Expression, LocalExpressionNode,
                SideEffectExpression,
            },
            Statement,
        },
        token::{
            condition_keyword::ConditionKeyword, datatype::DataType, identifier::Identifier,
            literal::Literal,
            tuple_identifier::Lvalue,
        },
        Ast,
    },
    compiler::{
        stdlib::{self},
        CompilationContext, CompiledProject, CompilerState, FunctionDefinition, Variable,
    },
    error::{AlthreadError, AlthreadResult, ErrorType},
    module_resolver::{module_resolver::ModuleResolver, FileSystem},
    vm::instruction::{Instruction, InstructionType, ProgramCode},
};

use super::ltl;

impl Ast {
    fn module_prefix(name: &str) -> &str {
        match name.rfind('.') {
            Some(idx) => &name[..idx],
            None => "",
        }
    }

    fn ast_empty(&self) -> bool {
        if self.process_blocks.is_empty()
            && self
                .global_block
                .as_ref()
                .map_or(true, |block| block.value.children.is_empty())
            && self.function_blocks.is_empty()
            && self.import_block.is_none()
            && self.condition_blocks.is_empty()
        {
            return true;
        }
        false
    }

    pub fn build_qualified_name(&self, name: &str, module_prefix: &str) -> String {
        if module_prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", module_prefix, name)
        }
    }

    pub fn check_privacy_violations(&self, state: &CompilerState) -> AlthreadResult<()> {
        // check function and program calls inside programs
        for (prog_name, prog_code) in state.programs_code().iter() {
            let caller_module = Ast::module_prefix(prog_name);

            for instruction in &prog_code.instructions {
                match &instruction.control {
                    InstructionType::FnCall {
                        name: call_name, ..
                    } => {
                        if let Some(func_def) = state.user_functions().get(call_name) {
                            let callee_module = Ast::module_prefix(call_name);

                            if func_def.is_private && caller_module != callee_module {
                                return Err(AlthreadError::new(
                                    ErrorType::PrivateFunctionCall,
                                    instruction.pos.clone(),
                                    format!(
                                        "Program '{}' cannot call private function '{}' from module '{}'",
                                        prog_name, call_name, callee_module
                                    )
                                ));
                            }
                        }
                    }
                    InstructionType::RunCall {
                        name: call_name, ..
                    } => {
                        if let Some((_, is_private)) = state.program_arguments().get(call_name) {
                            let callee_module = Ast::module_prefix(call_name);

                            if *is_private && caller_module != callee_module {
                                return Err(AlthreadError::new(
                                    ErrorType::PrivateFunctionCall,
                                    instruction.pos.clone(),
                                    format!(
                                        "Program '{}' cannot call private program '{}' from module '{}'",
                                        prog_name, call_name, callee_module
                                    )
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        for (func_name, func_def) in state.user_functions().iter() {
            let caller_module = Ast::module_prefix(func_name);

            for instruction in &func_def.body {
                match &instruction.control {
                    InstructionType::FnCall {
                        name: call_name, ..
                    } => {
                        if let Some(callee_def) = state.user_functions().get(call_name) {
                            let callee_module = Ast::module_prefix(call_name);

                            if callee_def.is_private && caller_module != callee_module {
                                return Err(AlthreadError::new(
                                    ErrorType::PrivateFunctionCall,
                                    instruction.pos.clone(),
                                    format!(
                                        "Function '{}' cannot call private function '{}' from module '{}'",
                                        func_name, call_name, callee_module
                                    )
                                ));
                            }
                        }
                    }
                    InstructionType::RunCall {
                        name: call_name, ..
                    } => {
                        if let Some((_, is_private)) = state.program_arguments().get(call_name) {
                            let callee_module = Ast::module_prefix(call_name);

                            if *is_private && caller_module != callee_module {
                                return Err(AlthreadError::new(
                                    ErrorType::PrivateFunctionCall,
                                    instruction.pos.clone(),
                                    format!(
                                        "Function '{}' cannot call private program '{}' from module '{}'",
                                        func_name, call_name, callee_module
                                    )
                                ));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn build_shared_const_scope(
        global_table: &HashMap<String, Variable>,
        global_memory: &BTreeMap<String, Literal>,
    ) -> AlthreadResult<(Vec<Variable>, Vec<Literal>)> {
        let mut names: Vec<_> = global_table.keys().cloned().collect();
        names.sort();

        let mut scope = Vec::with_capacity(names.len());
        let mut memory = Vec::with_capacity(names.len());

        for name in names {
            let variable = global_table
                .get(&name)
                .cloned()
                .expect("shared const scope key should exist in global_table");
            let literal = global_memory.get(&name).cloned().ok_or_else(|| {
                AlthreadError::new(
                    ErrorType::VariableError,
                    variable.declare_pos.clone(),
                    format!(
                        "Shared variable '{}' is registered but has no compile-time value",
                        name
                    ),
                )
            })?;
            scope.push(variable);
            memory.push(literal);
        }

        Ok((scope, memory))
    }

    fn validate_shared_const_expression(
        expression: &LocalExpressionNode,
        pos: &crate::error::Pos,
    ) -> AlthreadResult<()> {
        match expression {
            LocalExpressionNode::Binary(node) => {
                Self::validate_shared_const_expression(&node.left, pos)?;
                Self::validate_shared_const_expression(&node.right, pos)
            }
            LocalExpressionNode::Unary(node) => {
                Self::validate_shared_const_expression(&node.operand, pos)
            }
            LocalExpressionNode::Primary(node) => match node {
                crate::ast::statement::expression::primary_expression::LocalPrimaryExpressionNode::Literal(_) => {
                    Ok(())
                }
                crate::ast::statement::expression::primary_expression::LocalPrimaryExpressionNode::Var(_) => {
                    Ok(())
                }
                crate::ast::statement::expression::primary_expression::LocalPrimaryExpressionNode::Expression(expr) => {
                    Self::validate_shared_const_expression(expr, pos)
                }
            },
            LocalExpressionNode::Tuple(node) => {
                for value in &node.values {
                    Self::validate_shared_const_expression(value, pos)?;
                }
                Ok(())
            }
            LocalExpressionNode::Range(node) => {
                Self::validate_shared_const_expression(&node.expression_start, pos)?;
                Self::validate_shared_const_expression(&node.expression_end, pos)
            }
            LocalExpressionNode::FnCall(_) | LocalExpressionNode::CallChain(_) => Err(
                AlthreadError::new(
                    ErrorType::InstructionNotAllowed,
                    Some(pos.clone()),
                    "Shared initializers do not allow function or method calls".to_string(),
                ),
            ),
            LocalExpressionNode::Reaches(_) => Err(AlthreadError::new(
                ErrorType::InstructionNotAllowed,
                Some(pos.clone()),
                "Shared initializers do not allow 'reaches' predicates".to_string(),
            )),
            LocalExpressionNode::IfExpr(_)
            | LocalExpressionNode::ForAll(_)
            | LocalExpressionNode::Exists(_) => Err(AlthreadError::new(
                ErrorType::InstructionNotAllowed,
                Some(pos.clone()),
                "Shared initializers only allow pure compile-time expressions".to_string(),
            )),
        }
    }

    fn evaluate_shared_expression(
        expression: &Node<Expression>,
        scope: &[Variable],
        memory: &[Literal],
    ) -> AlthreadResult<Literal> {
        let scope = scope.to_vec();
        let memory = memory.to_vec();
        let local = LocalExpressionNode::from_expression(&expression.value, &scope)?;

        Self::validate_shared_const_expression(&local, &expression.pos)?;

        local.eval(&memory).map_err(|err| {
            AlthreadError::new(
                ErrorType::ExpressionError,
                Some(expression.pos.clone()),
                err,
            )
        })
    }

    fn evaluate_shared_bracket_expression(
        bracket: &Node<BracketExpression>,
        scope: &[Variable],
        memory: &[Literal],
    ) -> AlthreadResult<Literal> {
        match &bracket.value.content {
            BracketContent::Range(range) => {
                let expression = Node {
                    pos: range.pos.clone(),
                    value: Expression::Range(range.clone()),
                };
                Self::evaluate_shared_expression(&expression, scope, memory)
            }
            BracketContent::ListLiteral(elements) => {
                let mut evaluated = Vec::with_capacity(elements.len());
                let mut element_type = DataType::Void;

                for (index, element) in elements.iter().enumerate() {
                    let literal = Self::evaluate_shared_side_effect_expression(
                        element,
                        scope,
                        memory,
                    )?;
                    let literal_type = literal.get_datatype();

                    if index == 0 {
                        element_type = literal_type.clone();
                    } else if literal_type != element_type {
                        return Err(AlthreadError::new(
                            ErrorType::TypeError,
                            Some(element.pos.clone()),
                            format!(
                                "List element {} has type {}, expected {}",
                                index, literal_type, element_type
                            ),
                        ));
                    }

                    evaluated.push(literal);
                }

                Ok(Literal::List(element_type, evaluated))
            }
        }
    }

    fn evaluate_shared_side_effect_expression(
        expression: &Node<SideEffectExpression>,
        scope: &[Variable],
        memory: &[Literal],
    ) -> AlthreadResult<Literal> {
        match &expression.value {
            SideEffectExpression::Expression(node) => {
                Self::evaluate_shared_expression(node, scope, memory)
            }
            SideEffectExpression::Bracket(node) => {
                Self::evaluate_shared_bracket_expression(node, scope, memory)
            }
            SideEffectExpression::FnCall(_) => Err(AlthreadError::new(
                ErrorType::InstructionNotAllowed,
                Some(expression.pos.clone()),
                "Shared initializers do not allow function or method calls".to_string(),
            )),
            SideEffectExpression::RunCall(_) => Err(AlthreadError::new(
                ErrorType::InstructionNotAllowed,
                Some(expression.pos.clone()),
                "Shared initializers do not allow run calls".to_string(),
            )),
        }
    }

    fn evaluate_shared_initializer(
        value: Option<&Node<SideEffectExpression>>,
        datatype: &DataType,
        global_table: &HashMap<String, Variable>,
        global_memory: &BTreeMap<String, Literal>,
    ) -> AlthreadResult<Literal> {
        let literal = if let Some(value) = value {
            let (scope, memory) = Self::build_shared_const_scope(global_table, global_memory)?;
            Self::evaluate_shared_side_effect_expression(value, &scope, &memory)?
        } else {
            datatype.default()
        };

        match (literal, datatype) {
            (Literal::List(_, elements), DataType::List(element_type)) if elements.is_empty() => {
                Ok(Literal::List((**element_type).clone(), elements))
            }
            (literal, _) => Ok(literal),
        }
    }

    pub fn compile<F: FileSystem + Clone>(
        &self,
        current_file_path: &Path,
        filesystem: F,
        input_map: &mut HashMap<String, String>,
    ) -> AlthreadResult<CompiledProject> {
        // Create shared compilation context
        let context = Rc::new(RefCell::new(CompilationContext::new()));

        self.compile_with_context(
            current_file_path,
            filesystem,
            context,
            &"".to_string(),
            Vec::new(),
            input_map,
        )
    }

    pub fn compile_with_context<F: FileSystem + Clone>(
        &self,
        current_file_path: &Path,
        filesystem: F,
        context: Rc<RefCell<CompilationContext>>,
        module_prefix: &str,
        same_level_module_names: Vec<String>,
        input_map: &mut HashMap<String, String>,
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
                ltl_formulas: Vec::new(),
                compiled_ltl_formulas: Vec::new(),
                stdlib: Rc::new(stdlib::Stdlib::new()),
                program_debug_info: HashMap::new(),
            });
        }

        let mut state = CompilerState::new_with_context(context.clone());

        log::debug!("Current module prefix: {:?}", module_prefix);
        log::debug!(
            "[{}] Same level module names: {:?}",
            module_prefix,
            same_level_module_names
        );

        // scan everything for channel declarations in the current file
        if let Err(e) = self.prescan_channel_declarations(&mut state, module_prefix) {
            return Err(e);
        }

        let mut next_level_module_names = Vec::<String>::new();

        // if there's an import block, resolve the imports
        if let Some(import_block) = &self.import_block {
            let mut module_resolver = ModuleResolver::new(current_file_path, filesystem.clone());
            let mut import_stack = Vec::new();

            module_resolver.resolve_imports(&import_block.value, &mut import_stack, input_map)?;

            for (name, _) in module_resolver.resolved_modules.iter() {
                next_level_module_names.push(name.clone());
            }

            for (name, resolved_module) in module_resolver.resolved_modules {
                let compiled_module = resolved_module
                    .module_ast
                    .compile_with_context(
                        &resolved_module.resolved_path.path,
                        filesystem.clone(),
                        context.clone(),
                        &name,
                        next_level_module_names.clone(),
                        input_map,
                    )
                    .map_err(|mut e| {
                        e.push_stack(import_block.pos.clone());
                        e
                    })?;

                // display what was imported at this point
                log::debug!("----------------------------------------------------");
                log::debug!("[{}] Imported module '{}'", module_prefix, name);
                log::debug!(
                    "[{}] Imported shared variables: {:?}",
                    module_prefix,
                    compiled_module.global_memory.keys()
                );
                log::debug!(
                    "[{}] Imported functions: {:?}",
                    module_prefix,
                    compiled_module.user_functions.keys()
                );
                log::debug!(
                    "[{}] Imported programs: {:?}",
                    module_prefix,
                    compiled_module.programs_code.keys()
                );
                log::debug!(
                    "[{}] Imported always conditions: {:?}",
                    module_prefix,
                    compiled_module.always_conditions
                );
                log::debug!("----------------------------------------------------");

                state
                    .always_conditions_mut()
                    .extend(compiled_module.always_conditions);
                state
                    .global_memory_mut()
                    .extend(compiled_module.global_memory);
                state
                    .program_arguments_mut()
                    .extend(compiled_module.program_arguments);
                state
                    .user_functions_mut()
                    .extend(compiled_module.user_functions);
                state
                    .programs_code_mut()
                    .extend(compiled_module.programs_code);
                state
                    .global_table_mut()
                    .extend(compiled_module.global_table);
            }
        }

        // "compile" the "shared" block to retrieve the set of shared variables
        state.current_stack_depth = 1;
        state.is_shared = true;
        if let Some(global) = self.global_block.as_ref() {
            for node in global.value.children.iter() {
                match &node.value {
                    Statement::Declaration(decl) => {
                        let available_globals = state.global_memory().clone();
                        let available_table = state.global_table().clone();

                        node.compile(&mut state)?;
                        match &decl.value.identifier {
                            Lvalue::Identifier(node) =>{
                                let var_name = &node.value.value;
                                // Use context instead of local global_table
                                let last_program_stack = state.program_stack.last().unwrap().clone();
                                let literal = Self::evaluate_shared_initializer(
                                    decl.value.value.as_ref(),
                                    &last_program_stack.datatype,
                                    &available_table,
                                    &available_globals,
                                )?;

                                state
                                    .global_table_mut()
                                    .insert(var_name.clone(), last_program_stack);
                                state
                                    .global_memory_mut()
                                    .insert(var_name.clone(), literal);
                            },
                            Lvalue::TupleIdentifier(node) => 
                            {
                                print!("-----------COMPILER 2-----------\n");
                            },
                            Lvalue::NullIdentifier(node) => {
                                print!("-----------COMPILER 3-----------\n");
                            },
                        }
                    }
                    _ => {
                        return Err(AlthreadError::new(
                            ErrorType::InstructionNotAllowed,
                            Some(node.pos.clone()),
                            "The 'shared' block can only contain declarations".to_string(),
                        ))
                    }
                }
            }
        }

        log::debug!(
            "[{}] Shared variables: {:?}",
            module_prefix,
            state.global_memory().keys()
        );

        state.unstack_current_depth();
        assert!(state.current_stack_depth == 0);

        // functions baby ??
        // allow cross-function calls, recursive calls
        // this creates FunctionDefinitions without the compiled body, so that
        // compilation can be done no matter the order of the functions
        // or if they are recursive
        for (func_name, (args_list, return_datatype, func_block, is_private)) in
            &self.function_blocks
        {
            // check if the function is already defined
            if state.user_functions().contains_key(func_name) {
                return Err(AlthreadError::new(
                    ErrorType::FunctionAlreadyDefined,
                    Some(func_block.pos.clone()),
                    format!("Function '{}' is already defined", func_name),
                ));
            }
            // add the function to the user functions
            let arguments: Vec<(Identifier, DataType)> = args_list
                .value
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
                pos: func_block.pos.clone(),
                is_private: *is_private,
            };

            if let Err(e) = Ast::check_function_returns(&func_name, func_block, return_datatype) {
                return Err(e);
            }

            state
                .user_functions_mut()
                .insert(func_name.clone(), func_def);
        }

        // before compiling the programs, get the list of program names and their arguments
        let program_args: HashMap<String, (Vec<DataType>, bool)> = self
            .process_blocks
            .iter()
            .map(|(name, (args, _, is_private))| {
                (
                    name.clone(),
                    (
                        args.value
                            .datatypes
                            .iter()
                            .map(|d| d.value.clone())
                            .collect::<Vec<_>>(),
                        *is_private,
                    ),
                )
            })
            .collect();

        // Update context instead of state
        state.program_arguments_mut().extend(program_args);

        // Compile all the programs
        state.is_shared = false;

        // start with the main program

        if self.process_blocks.contains_key("main") {
            let is_private = state
                .program_arguments()
                .get("main")
                .is_some_and(|(_, is_private)| *is_private);

            if !module_prefix.is_empty() && !is_private {
                return Err(AlthreadError::new(
                        ErrorType::ImportMainConflict,
                        Some(self.process_blocks.get("main").unwrap().1.pos.clone()),
                        format!("'{}' defines a 'main' block.\nImported modules cannot define a 'main' block that is not set to private.\nUse '@private' to mark it as private.", module_prefix),
                ));
            }

            if module_prefix.is_empty() {
                let code = self.compile_program("main", &mut state, module_prefix)?;
                state.programs_code_mut().insert("main".to_string(), code);
                assert!(state.current_stack_depth == 0);
            }
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

        let program_names: Vec<String> = state.program_arguments().keys().cloned().collect();
        {
            let global_table = state.global_table_mut();
            for prog_name in program_names {
                let gs_name = format!("GS.procs.{}", prog_name);
                let dollar_name = format!("$.procs.{}", prog_name);
                if !global_table.contains_key(&gs_name) {
                    global_table.insert(
                        gs_name.clone(),
                        Variable {
                            mutable: false,
                            name: gs_name,
                            datatype: DataType::List(Box::new(DataType::Process(
                                prog_name.clone(),
                            ))),
                            depth: 0,
                            declare_pos: None,
                        },
                    );
                }
                if !global_table.contains_key(&dollar_name) {
                    global_table.insert(
                        dollar_name.clone(),
                        Variable {
                            mutable: false,
                            name: dollar_name,
                            datatype: DataType::List(Box::new(DataType::Process(
                                prog_name.clone(),
                            ))),
                            depth: 0,
                            declare_pos: None,
                        },
                    );
                }
            }
        }

        state.in_condition_block = true;
        for (name, condition_block) in self.condition_blocks.iter() {
            match name {
                ConditionKeyword::Always => {
                    for condition in condition_block.value.children.iter() {
                        let compiled = condition.compile(&mut state)?.instructions;
                        if compiled.len() == 1 {
                            return Err(AlthreadError::new(
                                ErrorType::InstructionNotAllowed,
                                Some(condition.pos.clone()),
                                "The condition must depend on shared variable(s)".to_string(),
                            ));
                        }
                        if compiled.len() != 2 {
                            return Err(AlthreadError::new(
                                ErrorType::InstructionNotAllowed,
                                Some(condition.pos.clone()),
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
                                    condition.pos.clone(),
                                ));
                            } else {
                                return Err(AlthreadError::new(
                                    ErrorType::InstructionNotAllowed,
                                    Some(condition.pos.clone()),
                                    "The condition must be a single expression".to_string(),
                                ));
                            }
                        } else {
                            return Err(AlthreadError::new(
                                ErrorType::InstructionNotAllowed,
                                Some(condition.pos.clone()),
                                "The condition must depend on shared variable(s)".to_string(),
                            ));
                        }
                    }
                }
                ConditionKeyword::Never => {
                    // 'never' blocks are not yet implemented, but reserved for future use.
                }
            }
        }
        state.in_condition_block = false;

        for check_block in self.check_blocks.iter() {
            for formula in &check_block.value.formulas {
                state.ltl_formulas_mut().push(formula.clone());
            }
        }

        // now compile the function bodies
        for (func_name, (args_list, return_datatype, func_block, is_private)) in
            &self.function_blocks
        {
            state.in_function = true;
            state.current_stack_depth += 1;
            let initial_stack_len = state.program_stack.len();

            let arguments: Vec<(Identifier, DataType)> = args_list
                .value
                .identifiers
                .iter()
                .zip(args_list.value.datatypes.iter())
                .map(|(id, dt)| {
                    state.program_stack.push(Variable {
                        name: id.value.value.clone(),
                        depth: state.current_stack_depth,
                        mutable: true,
                        datatype: dt.value.clone(),
                        declare_pos: Some(id.pos.clone()),
                    });
                    (id.value.clone(), dt.value.clone())
                })
                .collect();

            // compile the function body
            let mut compiled_body = func_block.compile(&mut state).map_err(|mut e| {
                e.push_stack(func_block.pos.clone());
                e
            })?;

            // if the function's return datatype is Void
            if *return_datatype == DataType::Void {
                let mut has_return = false;
                // check if it has a return instruction as the last instruction
                match compiled_body.instructions.last() {
                    Some(last_instruction) => {
                        if let InstructionType::Return { has_value: false } =
                            &last_instruction.control
                        {
                            has_return = true;
                        }
                    }
                    None => {}
                }
                // if it does not have a return instruction, add one
                if !has_return {
                    compiled_body.instructions.push(Instruction {
                        control: InstructionType::Return { has_value: false },
                        pos: Some(func_block.pos.clone()),
                    });
                }
            }

            state.user_functions_mut().insert(
                func_name.clone(),
                FunctionDefinition {
                    name: func_name.clone(),
                    arguments,
                    return_type: return_datatype.clone(),
                    body: compiled_body.instructions,
                    pos: func_block.pos.clone(),
                    is_private: *is_private,
                },
            );

            // clean up compiler state
            state.program_stack.truncate(initial_stack_len);
            state.current_stack_depth -= 1;
            state.in_function = false;
        }

        // if not the main file then we need to qualify everything
        // if !module_prefix.is_empty() {

        let user_functions: Vec<(String, FunctionDefinition)> = state
            .user_functions()
            .iter()
            .map(|(name, def)| (name.clone(), def.clone()))
            .collect();

        for (func_name, func_def) in user_functions {
            if same_level_module_names
                .iter()
                .any(|mod_name| func_name.starts_with(&format!("{}.", mod_name)))
            {
                log::debug!(
                    "[{}] Skipping function '{}' as it is already qualified",
                    module_prefix,
                    func_name
                );
                continue;
            }

            let old_func_def_opt = state.user_functions_mut().remove(&func_name);

            if old_func_def_opt.is_none() {
                return Err(AlthreadError::new(
                    ErrorType::FunctionNotFound,
                    Some(func_def.pos.clone()),
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
                    format!(
                        "Function '{}' from module '{}' is already defined",
                        func_name, module_prefix
                    ),
                ));
            }

            // replace all function calls in the body with the new function name
            for instruction in &mut old_func_def.body {
                match &mut instruction.control {
                    InstructionType::FnCall {
                        name: call_name, ..
                    } => {
                        if call_name == "print" || call_name == "assert" {
                            // do not qualify standard library function calls
                            continue;
                        }

                        // we don't care about the datatype, we just want to avoid qualifying list methods
                        if state
                            .stdlib()
                            .is_interface(&DataType::List(Box::new(DataType::Integer)), call_name)
                        {
                            // do not qualify standard library function calls
                            continue;
                        }
                        *call_name = self.build_qualified_name(call_name, module_prefix);
                    }
                    InstructionType::GlobalReads { variables, .. } => {
                        for var in variables.iter_mut() {
                            *var = self.build_qualified_name(var, module_prefix);
                        }
                    }
                    InstructionType::GlobalAssignment { identifier, .. } => {
                        *identifier = self.build_qualified_name(identifier, module_prefix);
                    }
                    InstructionType::MethodCall {
                        global_receiver, ..
                    } => {
                        if let Some(identifier) = global_receiver {
                            *identifier = self.build_qualified_name(identifier, module_prefix);
                        }
                    }
                    InstructionType::RunCall {
                        name: call_name, ..
                    } => {
                        *call_name = self.build_qualified_name(call_name, module_prefix);
                    }
                    _ => {}
                }
            }

            let func_def = FunctionDefinition {
                name: new_func_name.clone(),
                arguments: old_func_def.arguments.clone(),
                return_type: old_func_def.return_type.clone(),
                body: old_func_def.body.clone(),
                pos: old_func_def.pos.clone(),
                is_private: old_func_def.is_private,
            };

            // println!("[{}] New function definition: {:?}", module_prefix, func_def);

            state
                .user_functions_mut()
                .insert(new_func_name.clone(), func_def);
        }

        log::debug!("[{}] Module importing shared variables", module_prefix);
        log::debug!("Global memory: {:?}", state.global_memory_mut().keys());

        // Collect all variable data first to avoid borrow conflicts
        let shared_vars: Vec<(String, Literal, Option<Variable>)> = {
            let global_memory = state.global_memory();
            let global_table = state.global_table();

            global_memory
                .iter()
                .map(|(var_name, value)| {
                    let var_meta = global_table.get(var_name).cloned();
                    (var_name.clone(), value.clone(), var_meta)
                })
                .collect()
        };

        log::debug!(
            "[{}] Shared variables: {:?}",
            module_prefix,
            shared_vars
                .iter()
                .map(|(name, _, _)| name)
                .collect::<Vec<_>>()
        );

        for (var_name, value, var_meta) in shared_vars {
            let qualified_var_name = self.build_qualified_name(&var_name, module_prefix);

            if qualified_var_name == var_name
                || var_name.contains(module_prefix)
                || same_level_module_names
                    .iter()
                    .any(|mod_name| var_name.starts_with(&format!("{}.", mod_name)))
            {
                log::debug!(
                    "[{}] Skipping variable '{}' as it is already qualified",
                    module_prefix,
                    var_name
                );
                continue;
            }

            log::debug!(
                "[{}] Importing shared variable '{}'",
                module_prefix,
                qualified_var_name
            );

            if state.global_memory().contains_key(&qualified_var_name) {
                return Err(AlthreadError::new(
                    ErrorType::VariableAlreadyDefined,
                    None,
                    format!(
                        "Shared variable '{}' from module '{}' is already defined",
                        var_name, module_prefix
                    ),
                ));
            }

            state
                .global_memory_mut()
                .insert(qualified_var_name.clone(), value);
            state.global_memory_mut().remove(&var_name);

            if let Some(var_meta) = var_meta {
                let mut var_meta_cloned = var_meta.clone();
                var_meta_cloned.name = qualified_var_name.clone();
                state
                    .global_table_mut()
                    .insert(qualified_var_name.clone(), var_meta_cloned);
                state.global_table_mut().remove(&var_name);
            }
        }

        log::debug!(
            "[{}] Shared variables after import: {:?}",
            module_prefix,
            state.global_memory().keys()
        );

        log::debug!("[{}] Qualifying always conditions", module_prefix);
        // Collect conditions into a temporary vector to avoid borrow conflicts
        let always_conditions: Vec<_> = state.always_conditions().clone();
        let mut new_always_conditions = Vec::new();
        for condition in always_conditions.iter() {
            // skip if any dependency starts with any same-level module name
            if same_level_module_names.iter().any(|mod_name| {
                condition
                    .0
                    .iter()
                    .any(|dep| dep.starts_with(&format!("{}.", mod_name)))
            }) {
                log::debug!(
                    "[{}] Skipping condition as it starts with a same-level module name",
                    module_prefix
                );
                continue;
            }

            let (deps, read_vars, expr, pos) = condition;

            let updated_deps: HashSet<String> = deps
                .iter()
                .map(|dep| self.build_qualified_name(dep, module_prefix))
                .collect();
            let updated_read_vars: Vec<String> = read_vars
                .iter()
                .map(|var| self.build_qualified_name(var, module_prefix))
                .collect();

            new_always_conditions.push((
                updated_deps,
                updated_read_vars,
                expr.clone(),
                pos.clone(),
            ));
        }
        *state.always_conditions_mut() = new_always_conditions;

        // remove conditions that are not qualified
        if !module_prefix.is_empty() {
            log::debug!("[{}] Removing unqualified conditions", module_prefix);
            state
                .always_conditions_mut()
                .retain(|(deps, _read_vars, _expr, _pos)| deps.iter().any(|dep| dep.contains('.')));
        }

        log::debug!("[{}] Qualifying programs", module_prefix);
        // Collect programs to update first to avoid borrow conflicts
        let programs_to_update: Vec<(String, ProgramCode)> = state
            .programs_code()
            .iter()
            .map(|(prog_name, prog_code)| (prog_name.clone(), prog_code.clone()))
            .collect();
        log::debug!(
            "[{}] Programs to update: {:?}",
            module_prefix,
            programs_to_update
                .iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>()
        );

        for (prog_name, mut prog_code) in programs_to_update {
            log::debug!("[{}] Processing program '{}'", module_prefix, prog_name);

            if same_level_module_names
                .iter()
                .any(|mod_name| prog_name.starts_with(&format!("{}.", mod_name)))
            {
                log::debug!(
                    "[{}] Skipping program '{}' as it is already qualified",
                    module_prefix,
                    prog_name
                );
                continue;
            }

            let qualified_prog_name = self.build_qualified_name(&prog_name, module_prefix);

            if !(qualified_prog_name == prog_name) {
                log::debug!(
                    "[{}] Importing program '{}'",
                    module_prefix,
                    qualified_prog_name
                );
                if state.program_arguments().contains_key(&qualified_prog_name) {
                    return Err(AlthreadError::new(
                        ErrorType::ProgramAlreadyDefined,
                        Some(self.import_block.as_ref().unwrap().pos.clone()),
                        format!(
                            "Program '{}' from module '{}' is already defined",
                            prog_name, module_prefix
                        ),
                    ));
                }

                let prog_args = state
                    .program_arguments()
                    .get(&prog_name)
                    .cloned()
                    .unwrap_or_default();

                state
                    .program_arguments_mut()
                    .insert(qualified_prog_name.clone(), prog_args);
            }

            prog_code.name = qualified_prog_name.clone();

            for instruction in &mut prog_code.instructions {
                match &mut instruction.control {
                    InstructionType::FnCall {
                        name: call_name, ..
                    } => {
                        if call_name == "print" || call_name == "assert" {
                            // do not qualify standard library function calls
                            continue;
                        }

                        // we don't care about the datatype, we just want to avoid qualifying list methods
                        if state
                            .stdlib()
                            .is_interface(&DataType::List(Box::new(DataType::Integer)), call_name)
                        {
                            // do not qualify standard library function calls
                            continue;
                        }
                        *call_name = self.build_qualified_name(call_name, module_prefix);
                    }
                    InstructionType::GlobalReads { variables, .. } => {
                        for var in variables.iter_mut() {
                            *var = self.build_qualified_name(var, module_prefix);
                        }
                    }
                    InstructionType::GlobalAssignment { identifier, .. } => {
                        *identifier = self.build_qualified_name(identifier, module_prefix);
                    }
                    InstructionType::MethodCall {
                        global_receiver, ..
                    } => {
                        if let Some(identifier) = global_receiver {
                            *identifier = self.build_qualified_name(identifier, module_prefix);
                        }
                    }
                    InstructionType::RunCall {
                        name: call_name, ..
                    } => {
                        *call_name = self.build_qualified_name(call_name, module_prefix);
                    }
                    InstructionType::WaitStart { dependencies, .. } => {
                        let updated_vars: std::collections::HashSet<String> = dependencies
                            .variables
                            .iter()
                            .map(|dep| self.build_qualified_name(dep, module_prefix))
                            .collect();
                        dependencies.variables = updated_vars;
                    }
                    _ => {}
                }
            }
            state.programs_code_mut().remove(&prog_name);
            state
                .programs_code_mut()
                .insert(qualified_prog_name, prog_code.clone());
        }

        log::debug!("[{}] Compiled module with {} programs, {} functions, {} shared variables, {} always conditions", 
            module_prefix,
            state.programs_code().len(),
            state.user_functions().len(),
            state.global_memory().len(),
            state.always_conditions().len()
        );

        if module_prefix.is_empty() {
            log::debug!("-----------------------------------------------------");
            log::debug!("[Main module] Finished compiling main module");
            log::debug!(
                "[Main module] Shared variables: {:?}",
                state.global_memory().keys()
            );
            log::debug!(
                "[Main module] User functions: {:?}",
                state.user_functions().keys()
            );
            log::debug!(
                "[Main module] Programs code: {:?}",
                state.programs_code().keys()
            );
            log::debug!(
                "[Main module] Always conditions: {:?}",
                state.always_conditions()
            );
            log::debug!("-----------------------------------------------------");
        }

        self.check_privacy_violations(&state)?;

        // Return using context data instead of local variables
        Ok(CompiledProject {
            global_memory: state.global_memory().clone(),
            program_arguments: state.program_arguments().clone(),
            user_functions: state.user_functions().clone(),
            global_table: state.global_table().clone(),
            programs_code: state.programs_code().clone(),
            always_conditions: state.always_conditions().clone(),
            ltl_formulas: state.ltl_formulas().clone(),
            compiled_ltl_formulas: ltl::compile_ltl_formulas(state.ltl_formulas(), &state)?,
            stdlib: state.stdlib().clone(),
            program_debug_info: state.program_debug_info.clone(),
        })
    }

    fn compile_program(
        &self,
        name: &str,
        state: &mut CompilerState,
        module_prefix: &str,
    ) -> AlthreadResult<ProgramCode> {
        let mut process_code = ProgramCode {
            instructions: Vec::new(),
            name: name.to_string(),
            labels: HashMap::new(),
            argument_names: Vec::new(),
        };
        let (args, prog, _) = self
            .process_blocks
            .get(name)
            .expect("trying to compile a non-existant program");

        state.current_program_name = self.build_qualified_name(name, module_prefix);

        // Capture argument names for debug info
        let mut argument_names = Vec::new();
        let mut debug_variables = Vec::new();
        
        for (i, var) in args.value.identifiers.iter().enumerate() {
            let var_name = var.value.value.clone();
            argument_names.push(var_name.clone());
            
            state.program_stack.push(Variable {
                name: var_name.clone(),
                depth: state.current_stack_depth,
                mutable: true,
                datatype: args.value.datatypes[i].value.clone(),
                declare_pos: Some(var.pos.clone()),
            });
            
            // Add debug variable for program arguments (available from the start)
            debug_variables.push(crate::compiler::LocalVariableDebugInfo {
                name: var_name,
                datatype: args.value.datatypes[i].value.clone(),
                stack_index: i,
                scope_start_ip: 0,
                scope_end_ip: None,
                declare_pos: Some(var.pos.clone()),
            });
        }

        let compiled = prog.compile(state).map_err(|mut e| {
            e.push_stack(prog.pos.clone());
            e
        })?;
        if compiled.contains_jump() {
            unimplemented!("breaks or return statements in programs are not yet implemented");
        }
        
        // Collect debug variables from the compiled builder
        debug_variables.extend(compiled.debug_variables);
        
        if !args.value.identifiers.is_empty() {
            process_code.instructions.push(Instruction {
                control: InstructionType::Destruct(0),
                pos: Some(args.pos.clone()),
            });
        }
        process_code.instructions.extend(compiled.instructions);
        process_code.instructions.push(Instruction {
            control: InstructionType::EndProgram,
            pos: Some(prog.pos.clone()),
        });

        let mut label_map: HashMap<String, usize> = HashMap::new();
        for (idx, inst) in process_code.instructions.iter().enumerate() {
            if let InstructionType::Label { name } = &inst.control {
                if name == "end" {
                    return Err(AlthreadError::new(
                        ErrorType::SyntaxError,
                        inst.pos.clone(),
                        "Label name 'end' is reserved".to_string(),
                    ));
                }
                if label_map.contains_key(name) {
                    return Err(AlthreadError::new(
                        ErrorType::SyntaxError,
                        inst.pos.clone(),
                        format!("Label '{}' is already defined", name),
                    ));
                }
                label_map.insert(name.clone(), idx);
            }
        }

        let end_index = process_code.instructions.len() - 1;
        label_map.insert("end".to_string(), end_index);
        process_code.labels = label_map;
        process_code.argument_names = argument_names.clone();
        
        // Store debug info for this program
        state.program_debug_info.insert(
            state.current_program_name.clone(),
            crate::compiler::ProgramDebugInfo {
                argument_names,
                local_variables: debug_variables,
            },
        );
        
        // Clear debug variables for next program
        state.debug_variables.clear();
        
        Ok(process_code)
    }
}
