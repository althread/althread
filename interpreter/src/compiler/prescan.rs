use std::collections::HashMap;

use crate::{
    analysis::control_flow_graph::ControlFlowGraph,
    ast::{
        block::Block,
        node::Node,
        statement::{
            assignment::Assignment,
            channel_declaration::ChannelDeclaration,
            expression::{Expression, LocalExpressionNode},
            Statement,
        },
        token::datatype::DataType,
        Ast,
    },
    compiler::{CompilerState, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType},
};

fn known_channel_types_for_receive(
    receive: &crate::ast::statement::receive::ReceiveStatement,
    state: &CompilerState,
) -> Option<Vec<DataType>> {
    state
        .channels()
        .get(&(state.current_program_name.clone(), receive.channel.clone()))
        .map(|(types, _)| types.clone())
}

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
                func_name, missing_return_pos.line()
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
            Statement::For(for_statement) => {
                self.extract_channel_declarations_from_statement(
                    &for_statement.value.statement.value,
                    state,
                    module_prefix,
                    var_to_program,
                )?;
            }
            Statement::While(while_statement) => {
                self.extract_channel_declarations_from_block(
                    &while_statement.value.then_block.value,
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
            self.prescan_get_prog_name(&channel_decl.ch_left_prog.value.value, module_prefix, var_to_program)?;
        let right_prog =
            self.prescan_get_prog_name(&channel_decl.ch_right_prog.value.value, module_prefix, var_to_program)?;

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
        state: &mut CompilerState,
        var_to_program: &mut HashMap<String, String>,
    ) -> AlthreadResult<()> {
        for (_, (args, program_block, _)) in &self.process_blocks {
            let stack_len = state.program_stack.len();
            let depth = state.current_stack_depth;
            state.current_stack_depth += 1;

            for (identifier, datatype) in args
                .value
                .identifiers
                .iter()
                .zip(args.value.datatypes.iter())
            {
                state.program_stack.push(Variable {
                    mutable: true,
                    name: identifier.value.value.clone(),
                    datatype: datatype.value.clone(),
                    depth: state.current_stack_depth,
                    declare_pos: Some(identifier.pos.clone()),
                });
            }

            self.scan_block_for_typed_processes(&program_block.value, state, var_to_program)?;
            state.program_stack.truncate(stack_len);
            state.current_stack_depth = depth;
        }

        for (_, (args, _, function_block, _)) in &self.function_blocks {
            let stack_len = state.program_stack.len();
            let depth = state.current_stack_depth;
            state.current_stack_depth += 1;

            for (identifier, datatype) in args
                .value
                .identifiers
                .iter()
                .zip(args.value.datatypes.iter())
            {
                state.program_stack.push(Variable {
                    mutable: true,
                    name: identifier.value.value.clone(),
                    datatype: datatype.value.clone(),
                    depth: state.current_stack_depth,
                    declare_pos: Some(identifier.pos.clone()),
                });
            }

            self.scan_block_for_typed_processes(&function_block.value, state, var_to_program)?;
            state.program_stack.truncate(stack_len);
            state.current_stack_depth = depth;
        }

        Ok(())
    }

    fn scan_block_for_typed_processes(
        &self,
        block: &Block,
        state: &mut CompilerState,
        var_to_program: &mut HashMap<String, String>,
    ) -> AlthreadResult<()> {
        let stack_len = state.program_stack.len();
        let depth = state.current_stack_depth;
        state.current_stack_depth += 1;

        for statement in &block.children {
            self.scan_statement_for_typed_processes(&statement.value, state, var_to_program)?;
        }

        state.program_stack.truncate(stack_len);
        state.current_stack_depth = depth;
        Ok(())
    }

    fn scan_statement_for_typed_processes(
        &self,
        statement: &Statement,
        state: &mut CompilerState,
        var_to_program: &mut HashMap<String, String>,
    ) -> AlthreadResult<()> {
        match statement {
            Statement::Declaration(var_decl) => {
                let Some(identifier_node) = var_decl.value.identifier.value.parts.first() else {
                    return Ok(());
                };
                let var_name = identifier_node.value.value.clone();

                let datatype = if let Some(explicit) = &var_decl.value.datatype {
                    explicit.value.clone()
                } else if let Some(value) = &var_decl.value.value {
                    match Self::prescan_expression_datatype(&value.value, state) {
                        Ok(datatype) => datatype,
                        Err(message) => {
                            log::debug!(
                                "Skipping declaration type inference during prescan for '{}': {}",
                                var_name,
                                message
                            );
                            return Ok(());
                        }
                    }
                } else {
                    return Ok(());
                };

                self.record_process_type(&var_name, &datatype, var_to_program);

                state.program_stack.push(Variable {
                    mutable: true,
                    name: var_name,
                    datatype,
                    depth: state.current_stack_depth,
                    declare_pos: Some(var_decl.pos.clone()),
                });
            }
            Statement::Assignment(assignment) => {
                let Assignment::Binary(binary) = &assignment.value;
                let Some(identifier_node) = binary.value.identifier.value.parts.first() else {
                    return Ok(());
                };
                let datatype =
                    match Self::prescan_expression_datatype(&binary.value.value.value, state) {
                        Ok(datatype) => datatype,
                        Err(message) => {
                            log::debug!(
                                "Skipping assignment type inference during prescan for '{}': {}",
                                identifier_node.value.value,
                                message
                            );
                            return Ok(());
                        }
                    };
                self.record_process_type(&identifier_node.value.value, &datatype, var_to_program);
            }
            Statement::Atomic(atomic_statement) => {
                self.scan_statement_for_typed_processes(
                    &atomic_statement.value.statement.value,
                    state,
                    var_to_program,
                )?;
            }
            Statement::If(if_statement) => {
                self.scan_block_for_typed_processes(
                    &if_statement.value.then_block.value,
                    state,
                    var_to_program,
                )?;
                if let Some(else_block) = &if_statement.value.else_block {
                    self.scan_block_for_typed_processes(&else_block.value, state, var_to_program)?;
                }
            }
            Statement::Block(block) => {
                self.scan_block_for_typed_processes(&block.value, state, var_to_program)?;
            }
            Statement::For(for_statement) => {
                let stack_len = state.program_stack.len();
                let depth = state.current_stack_depth;
                state.current_stack_depth += 1;

                let item_type =
                    Self::prescan_expression_datatype(&for_statement.value.expression.value, state)
                        .ok()
                        .and_then(|dtype| match dtype {
                            DataType::List(inner) => Some(*inner),
                            _ => None,
                        })
                        .unwrap_or(DataType::Integer);

                state.program_stack.push(Variable {
                    mutable: true,
                    name: for_statement.value.identifier.value.value.clone(),
                    datatype: item_type,
                    depth: state.current_stack_depth,
                    declare_pos: Some(for_statement.value.identifier.pos.clone()),
                });

                self.scan_statement_for_typed_processes(
                    &for_statement.value.statement.value,
                    state,
                    var_to_program,
                )?;

                state.program_stack.truncate(stack_len);
                state.current_stack_depth = depth;
            }
            Statement::Loop(loop_statement) => {
                let stack_len = state.program_stack.len();
                let depth = state.current_stack_depth;
                state.current_stack_depth += 1;
                self.scan_statement_for_typed_processes(
                    &loop_statement.value.statement.value,
                    state,
                    var_to_program,
                )?;
                state.program_stack.truncate(stack_len);
                state.current_stack_depth = depth;
            }
            Statement::While(while_statement) => {
                self.scan_block_for_typed_processes(
                    &while_statement.value.then_block.value,
                    state,
                    var_to_program,
                )?;
            }
            Statement::Wait(wait_statement) => {
                for case in &wait_statement.value.waiting_cases {
                    let stack_len = state.program_stack.len();
                    let depth = state.current_stack_depth;
                    state.current_stack_depth += 1;

                    if let crate::ast::statement::waiting_case::WaitingBlockCaseRule::Receive(
                        receive,
                    ) = &case.value.rule
                    {
                        if let Some(channel_types) =
                            known_channel_types_for_receive(&receive.value, state)
                        {
                            for (idx, variable) in receive.value.variables.iter().enumerate() {
                                if let Some(datatype) = channel_types.get(idx) {
                                    state.program_stack.push(Variable {
                                        mutable: true,
                                        name: variable.clone(),
                                        datatype: datatype.clone(),
                                        depth: state.current_stack_depth,
                                        declare_pos: Some(receive.pos.clone()),
                                    });
                                }
                            }
                        }
                    }

                    if let Some(statement) = &case.value.statement {
                        self.scan_statement_for_typed_processes(
                            &statement.value,
                            state,
                            var_to_program,
                        )?;
                    }

                    state.program_stack.truncate(stack_len);
                    state.current_stack_depth = depth;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn record_process_type(
        &self,
        var_name: &str,
        datatype: &DataType,
        var_to_program: &mut HashMap<String, String>,
    ) {
        if let DataType::Process(program_name) = datatype {
            var_to_program.insert(var_name.to_string(), program_name.clone());
        }
    }

    fn prescan_expression_datatype(
        expression: &Expression,
        state: &mut CompilerState,
    ) -> Result<DataType, String> {
        let original_stack = state.program_stack.clone();
        let mut scope: Vec<Variable> = state.global_table().values().cloned().collect();
        scope.extend(original_stack.clone());
        state.program_stack = scope;

        let result = LocalExpressionNode::from_expression(expression, &state.program_stack)
            .map_err(|e| e.message)
            .and_then(|expr| expr.datatype(state));

        state.program_stack = original_stack;
        result
    }

    pub fn prescan_channel_declarations(
        &self,
        state: &mut CompilerState,
        module_prefix: &str,
    ) -> AlthreadResult<()> {
        // Build variable-to-program mapping first
        let mut var_to_program: HashMap<String, String> = HashMap::new();
        self.build_variable_program_mapping(state, &mut var_to_program)?;

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
