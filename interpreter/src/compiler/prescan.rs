use std::collections::HashMap;

use crate::{
    analysis::control_flow_graph::ControlFlowGraph,
    ast::{
        block::Block,
        node::Node,
        statement::{
            assignment::{Assignment},
            channel_declaration::ChannelDeclaration,
            expression::{primary_expression::PrimaryExpression, Expression, SideEffectExpression},
            Statement,
        },
        token::datatype::DataType,
        Ast,
    },
    compiler::CompilerState,
    error::{AlthreadError, AlthreadResult, ErrorType},
};

impl Ast {
    pub fn check_function_returns(
        func_name: &str,
        func_body: &Node<Block>,
        return_type: &DataType,
    ) -> AlthreadResult<()> {
        if matches!(return_type, DataType::Void) {
            return Ok(());
        }

        let cfg = ControlFlowGraph::from_function(func_body);

        // display the control flow graph for debugging
        // cfg.display();

        // cfg.display_ascii_flowchart();

        // cfg.display_dot();

        // we need to return the function at line does not return a value
        // and say on which line it does not return a value

        if let Some(missing_return_pos) = cfg.find_first_missing_return_point(func_body.pos.clone())
        {
            return Err(AlthreadError::new(
            ErrorType::FunctionMissingReturnStatement,
            Some(missing_return_pos.clone()), // Use the specific Pos found by the CFG analysis
            format!(
                "Function '{}' does not return a value on all code paths. Problem detected in construct starting at line {}.",
                func_name, missing_return_pos.line
            ),
        ));
        }

        Ok(())
    }

    fn extract_channel_declarations_from_statement(
        &self,
        statement: &Statement,
        state: &mut CompilerState,
        module_prefix: &str,
        var_to_program: &HashMap<String, String>,
    ) -> AlthreadResult<()> {
        match statement {
            Statement::ChannelDeclaration(channel_decl) => {
                self.register_channel_declaration(
                    &channel_decl.value,
                    state,
                    module_prefix,
                    var_to_program,
                )?;
            }
            Statement::Atomic(atomic_statement) => {
                self.extract_channel_declarations_from_statement(
                    &atomic_statement.value.statement.value,
                    state,
                    module_prefix,
                    var_to_program,
                )?;
            }
            Statement::If(if_statement) => {
                self.extract_channel_declarations_from_block(
                    &if_statement.value.then_block.value,
                    state,
                    module_prefix,
                    var_to_program,
                )?;
                if let Some(else_block) = &if_statement.value.else_block {
                    self.extract_channel_declarations_from_block(
                        &else_block.value,
                        state,
                        module_prefix,
                        var_to_program,
                    )?;
                }
            }
            Statement::Block(block) => {
                self.extract_channel_declarations_from_block(
                    &block.value,
                    state,
                    module_prefix,
                    var_to_program,
                )?;
            }
            Statement::Loop(loop_statement) => {
                self.extract_channel_declarations_from_statement(
                    &loop_statement.value.statement.value,
                    state,
                    module_prefix,
                    var_to_program,
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn register_channel_declaration(
        &self,
        channel_decl: &ChannelDeclaration,
        state: &mut CompilerState,
        module_prefix: &str,
        var_to_program: &HashMap<String, String>,
    ) -> AlthreadResult<()> {
        // Resolve program names for both sides of the channel
        let left_prog =
            self.prescan_get_prog_name(&channel_decl.ch_left_prog, module_prefix, var_to_program)?;
        let right_prog =
            self.prescan_get_prog_name(&channel_decl.ch_right_prog, module_prefix, var_to_program)?;

        // Create channel keys for both sender and receiver
        let left_key = (left_prog, channel_decl.ch_left_name.clone());
        let right_key = (right_prog, channel_decl.ch_right_name.clone());

        // Register the channel types - both sides get the same datatype info
        let pos = crate::error::Pos::default(); // We don't have position info during prescan
        state.channels_mut().insert(
            left_key.clone(),
            (channel_decl.datatypes.clone(), pos.clone()),
        );
        state
            .channels_mut()
            .insert(right_key.clone(), (channel_decl.datatypes.clone(), pos));

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
                format!(
                    "Variable '{}' not found in run statements during prescan",
                    var_name
                ),
            ))
        }
    }

    fn extract_channel_declarations_from_block(
        &self,
        block: &Block,
        state: &mut CompilerState,
        module_prefix: &str,
        var_to_program: &HashMap<String, String>,
    ) -> AlthreadResult<()> {
        for statement in &block.children {
            self.extract_channel_declarations_from_statement(
                &statement.value,
                state,
                module_prefix,
                var_to_program,
            )?;
        }
        Ok(())
    }

    fn build_variable_program_mapping(
        &self,
        var_to_program: &mut HashMap<String, String>,
    ) -> AlthreadResult<()> {
        let mut process_lists: HashMap<String, String> = HashMap::new();

        // Scan all process blocks, not just main
        for (program_name, (_, program_block, _)) in &self.process_blocks {
            self.scan_block_for_run_statements(
                &program_block.value,
                var_to_program,
                &mut process_lists,
                program_name,
            )?;
        }
        // Scan all function blocks
        for (function_name, (_, _, function_block, _)) in &self.function_blocks {
            self.scan_block_for_run_statements(
                &function_block.value,
                var_to_program,
                &mut process_lists,
                &format!("function_{}", function_name),
            )?;
        }

        Ok(())
    }

    fn scan_block_for_run_statements(
        &self,
        block: &Block,
        var_to_program: &mut HashMap<String, String>,
        process_lists: &mut HashMap<String, String>,
        current_program: &str,
    ) -> AlthreadResult<()> {
        for statement in &block.children {
            self.scan_statement_for_run_statements(
                &statement.value,
                var_to_program,
                process_lists,
                current_program,
            )?;
        }
        Ok(())
    }

    fn scan_statement_for_run_statements(
        &self,
        statement: &Statement,
        var_to_program: &mut HashMap<String, String>,
        process_lists: &mut HashMap<String, String>,
        current_program: &str,
    ) -> AlthreadResult<()> {
        match statement {
            Statement::Declaration(var_decl) => {
                let var_name = &var_decl.value.identifier.value.parts[0].value.value;

                // Check if this is a list
                if let Some(list_type) = var_decl.value.datatype.as_ref() {
                    let (is_process, element_type) = list_type.value.is_process();
                    if is_process {
                        process_lists.insert(var_name.clone(), element_type);
                    }
                }

                // check for run calls
                if let Some(side_effect_node) = var_decl.value.value.as_ref() {
                    if let Some(program_name) =
                        self.extract_run_program_name(&side_effect_node.value)
                    {
                        var_to_program.insert(
                            var_decl.value.identifier.value.parts[0].value.value.clone(),
                            program_name,
                        );
                    }
                    // check for reference assignments (let b = a;)
                    else if let Some(ref_var) =
                        self.extract_variable_reference(&side_effect_node.value)
                    {
                        if let Some(program_type) = var_to_program.get(&ref_var) {
                            var_to_program.insert(var_name.clone(), program_type.clone());
                        }

                        if let Some(element_type) = process_lists.get(&ref_var).cloned() {
                            process_lists.insert(var_name.clone(), element_type);
                        }
                    }
                    // check for .at() calls
                    else if let Some((list_var, _index)) =
                        self.extract_list_at_call(&side_effect_node.value)
                    {
                        if let Some(element_type) = process_lists.get(&list_var).cloned() {
                            var_to_program.insert(var_name.clone(), element_type);
                        }
                    }
                }
            }
            Statement::Assignment(assignment) => {
                // handle assignments like: p1 = a.at(i);
                let Assignment::Binary(binary) = &assignment.value;
                
                // Only handle identifier assignments for prescan
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
                    if let Some(element_type) = process_lists.get(&list_var).cloned() {
                        var_to_program.insert(var_name.clone(), element_type);
                    }
                }
            }
            Statement::Atomic(atomic_statement) => {
                self.scan_statement_for_run_statements(
                    &atomic_statement.value.statement.value,
                    var_to_program,
                    process_lists,
                    current_program,
                )?;
            }
            Statement::If(if_statement) => {
                self.scan_block_for_run_statements(
                    &if_statement.value.then_block.value,
                    var_to_program,
                    process_lists,
                    current_program,
                )?;
                if let Some(else_block) = &if_statement.value.else_block {
                    self.scan_block_for_run_statements(
                        &else_block.value,
                        var_to_program,
                        process_lists,
                        current_program,
                    )?;
                }
            }
            Statement::Block(block) => {
                self.scan_block_for_run_statements(
                    &block.value,
                    var_to_program,
                    process_lists,
                    current_program,
                )?;
            }
            Statement::For(for_statement) => {
                self.scan_statement_for_run_statements(
                    &for_statement.value.statement.value,
                    var_to_program,
                    process_lists,
                    current_program,
                )?;
            }
            Statement::Loop(loop_statement) => {
                self.scan_statement_for_run_statements(
                    &loop_statement.value.statement.value,
                    var_to_program,
                    process_lists,
                    current_program,
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn extract_variable_reference(
        &self,
        side_effect_expr: &SideEffectExpression,
    ) -> Option<String> {
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
            _ => {}
        }
        None
    }

    fn extract_list_at_call(
        &self,
        side_effect_expr: &SideEffectExpression,
    ) -> Option<(String, String)> {
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
            _ => {}
        }
        None
    }

    fn extract_run_program_name(&self, side_effect_expr: &SideEffectExpression) -> Option<String> {
        match side_effect_expr {
            SideEffectExpression::RunCall(run_call_node) => {
                Some(run_call_node.value.program_name_to_string())
            }
            _ => None,
        }
    }

    pub fn prescan_channel_declarations(
        &self,
        state: &mut CompilerState,
        module_prefix: &str,
    ) -> AlthreadResult<()> {
        // Build variable-to-program mapping first
        let mut var_to_program: HashMap<String, String> = HashMap::new();
        self.build_variable_program_mapping(&mut var_to_program)?;

        log::debug!("[{}] Prescanning for channel declarations", module_prefix);

        // Scan ALL process blocks for channel declarations, not just main
        for (program_name, (_, program_block, _)) in &self.process_blocks {
            log::debug!(
                "Scanning program '{}' for channel declarations",
                program_name
            );
            self.extract_channel_declarations_from_block(
                &program_block.value,
                state,
                module_prefix,
                &var_to_program,
            )?;
        }

        // Scan ALL function blocks for channel declarations
        for (function_name, (_, _, function_block, _)) in &self.function_blocks {
            log::debug!(
                "Scanning function '{}' for channel declarations",
                function_name
            );
            self.extract_channel_declarations_from_block(
                &function_block.value,
                state,
                module_prefix,
                &var_to_program,
            )?;
        }
        Ok(())
    }
}
