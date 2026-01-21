use std::collections::HashMap;

use crate::{
    ast::{statement::expression::LocalExpressionNode, token::{datatype::DataType, literal::Literal}},
    checker::ltl::{
        automaton::BuchiAutomaton, compiled::CompiledLtlExpression, monitor::MonitoringState,
    },
    error::{AlthreadError, AlthreadResult, ErrorType},
    vm::VM,
};

/// Analyzes LTL formulas to detect top-level quantifiers that require dynamic monitor instantiation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantifierKind {
    ForAll,
    Exists,
}

#[derive(Debug, Clone)]
pub struct TopLevelQuantifier {
    pub var_name: String,
    pub list_expression: LocalExpressionNode,
    pub list_read_variables: Vec<String>,
    pub kind: QuantifierKind,
}

/// Analyzes a formula to extract top-level quantifiers (for loops at the root level)
pub fn analyze_formula(formula: &CompiledLtlExpression) -> Option<TopLevelQuantifier> {
    match formula {
        CompiledLtlExpression::ForLoop {
            list_expression,
            list_read_variables,
            loop_var_name,
            ..
        } => Some(TopLevelQuantifier {
            var_name: loop_var_name.clone(),
            list_expression: list_expression.clone(),
            list_read_variables: list_read_variables.clone(),
            kind: QuantifierKind::ForAll,
        }),
        CompiledLtlExpression::Exists {
            list_expression,
            list_read_variables,
            loop_var_name,
            ..
        } => Some(TopLevelQuantifier {
            var_name: loop_var_name.clone(),
            list_expression: list_expression.clone(),
            list_read_variables: list_read_variables.clone(),
            kind: QuantifierKind::Exists,
        }),
        _ => None,
    }
}

/// Initializes monitoring state with proper monitor instances based on formula analysis
pub fn initialize_monitoring(
    formulas: &[CompiledLtlExpression],
    automatons: &[BuchiAutomaton],
    vm: &VM,
) -> AlthreadResult<MonitoringState> {
    let mut monitoring = MonitoringState::new(formulas.len());

    for (idx, formula) in formulas.iter().enumerate() {
        let automaton = &automatons[idx];

        if let Some(quantifier) = analyze_formula(formula) {
            // Top-level for loop: instantiate one monitor per list element

            // Evaluate the list expression
            let memory = prepare_list_memory(&quantifier.list_read_variables, vm)?;
            let list_value = quantifier
                .list_expression
                .eval_with_context(&memory, vm)
                .map_err(|msg| {
                    AlthreadError::new(
                        ErrorType::ExpressionError,
                        None,
                        format!("Error evaluating list for top-level quantifier: {}", msg),
                    )
                })?;

            let elements = match list_value {
                Literal::List(_, elems) => elems,
                _ => {
                    return Err(AlthreadError::new(
                        ErrorType::ExpressionError,
                        None,
                        format!("Top-level for loop expects a list, got {:?}", list_value),
                    ))
                }
            };

            // Create one monitor per element
            for element in elements {
                let mut bindings = HashMap::new();
                bindings.insert(quantifier.var_name.clone(), element);
                monitoring.add_monitors_for_enabled_initial_states(
                    idx,
                    automaton,
                    bindings,
                    vm,
                )?;
            }

            println!(
                "Initialized {} monitors for formula #{} (top-level for loop)",
                monitoring.monitors_per_formula[idx].len(),
                idx + 1
            );
        } else {
            // No top-level quantifier: single monitor with empty bindings
            monitoring.add_monitors_for_enabled_initial_states(
                idx,
                automaton,
                HashMap::new(),
                vm,
            )?;
            println!("Initialized 1 monitor for formula #{}", idx + 1);
        }
    }

    Ok(monitoring)
}

/// Prepares memory for evaluating list expressions
fn prepare_list_memory(
    read_variables: &[String],
    vm: &VM,
) -> AlthreadResult<crate::vm::Memory> {
    let mut memory = Vec::new();

    for var_name in read_variables {
        if let Some(proc_name) = var_name.strip_prefix("$.procs.") {
            let values = vm
                .running_programs
                .iter()
                .filter(|p| p.name == proc_name)
                .map(|p| Literal::Process(p.name.clone(), p.id))
                .collect::<Vec<_>>();
            memory.push(Literal::List(
                DataType::Process(proc_name.to_string()),
                values,
            ));
        } else if let Some(value) = vm.globals.get(var_name) {
            memory.push(value.clone());
        } else {
            return Err(AlthreadError::new(
                ErrorType::VariableError,
                None,
                format!("Variable '{}' not found in global memory", var_name),
            ));
        }
    }

    Ok(memory)
}

/// Checks if new processes were created and updates monitoring state accordingly
pub fn update_monitors_for_new_processes(
    formulas: &[CompiledLtlExpression],
    automatons: &[BuchiAutomaton],
    monitoring: &mut MonitoringState,
    vm: &VM,
) -> AlthreadResult<()> {
    for (idx, formula) in formulas.iter().enumerate() {
        if let Some(quantifier) = analyze_formula(formula) {
            // Check if the list has grown
            let memory = prepare_list_memory(&quantifier.list_read_variables, vm)?;
            let list_value = quantifier
                .list_expression
                .eval_with_context(&memory, vm)
                .map_err(|msg| {
                    AlthreadError::new(
                        ErrorType::ExpressionError,
                        None,
                        format!("Error evaluating list for monitor update: {}", msg),
                    )
                })?;

            let elements = match list_value {
                Literal::List(_, elems) => elems,
                _ => continue,
            };

            let current_monitor_count = monitoring.monitors_per_formula[idx].len();

            // Create monitors for new elements
            if elements.len() > current_monitor_count {
                for element in elements.into_iter().skip(current_monitor_count) {
                    let mut bindings = HashMap::new();
                    bindings.insert(quantifier.var_name.clone(), element);
                    monitoring.add_monitors_for_enabled_initial_states(
                        idx,
                        &automatons[idx],
                        bindings,
                        vm,
                    )?;
                }

                let new_count = monitoring.monitors_per_formula[idx].len();
                println!(
                    "Added {} new monitors for formula #{} (now {} total)",
                    new_count - current_monitor_count,
                    idx + 1,
                    new_count
                );
            }
        }
    }

    Ok(())
}
