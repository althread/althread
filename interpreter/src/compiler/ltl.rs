use crate::ast::{
    statement::{
        expression::{Expression, LocalExpressionNode},
        waiting_case::WaitDependency,
    },
};
use crate::ast::token::datatype::DataType;
use crate::checker::ltl::ast::LtlExpression;
use crate::checker::ltl::compiled::CompiledLtlExpression;
use crate::compiler::{CompilerState, Variable};
use crate::error::{AlthreadError, AlthreadResult, ErrorType};

pub fn compile_ltl_formulas(
    formulas: &Vec<LtlExpression>,
    state: &CompilerState,
) -> AlthreadResult<Vec<CompiledLtlExpression>> {
    let mut compiled_formulas = Vec::new();
    for formula in formulas {
        compiled_formulas.push(compile_ltl_node(formula, state, &mut Vec::new())?);
    }
    Ok(compiled_formulas)
}

fn compile_ltl_node(
    node: &LtlExpression,
    state: &CompilerState,
    loop_vars: &mut Vec<Variable>,
) -> AlthreadResult<CompiledLtlExpression> {
    match node {
        LtlExpression::Eventually(inner) => Ok(CompiledLtlExpression::Eventually(Box::new(
            compile_ltl_node(inner, state, loop_vars)?,
        ))),
        LtlExpression::Always(inner) => Ok(CompiledLtlExpression::Always(Box::new(
            compile_ltl_node(inner, state, loop_vars)?,
        ))),
        LtlExpression::Next(inner) => Ok(CompiledLtlExpression::Next(Box::new(compile_ltl_node(
            inner, state, loop_vars,
        )?))),
        LtlExpression::Not(inner) => Ok(CompiledLtlExpression::Not(Box::new(compile_ltl_node(
            inner, state, loop_vars,
        )?))),
        LtlExpression::Until(lhs, rhs) => Ok(CompiledLtlExpression::Until(
            Box::new(compile_ltl_node(lhs, state, loop_vars)?),
            Box::new(compile_ltl_node(rhs, state, loop_vars)?),
        )),
        LtlExpression::And(lhs, rhs) => Ok(CompiledLtlExpression::And(
            Box::new(compile_ltl_node(lhs, state, loop_vars)?),
            Box::new(compile_ltl_node(rhs, state, loop_vars)?),
        )),
        LtlExpression::Or(lhs, rhs) => Ok(CompiledLtlExpression::Or(
            Box::new(compile_ltl_node(lhs, state, loop_vars)?),
            Box::new(compile_ltl_node(rhs, state, loop_vars)?),
        )),
        LtlExpression::Implies(lhs, rhs) => Ok(CompiledLtlExpression::Implies(
            Box::new(compile_ltl_node(lhs, state, loop_vars)?),
            Box::new(compile_ltl_node(rhs, state, loop_vars)?),
        )),
        LtlExpression::Predicate(expr_node) => {
            compile_predicate(&expr_node.value, state, loop_vars)
        }
        LtlExpression::ForLoop {
            var_name,
            list,
            body,
        } => {
            // Compile list expression
            // The list expression itself acts like a predicate/expression: it can depend on variables
            let (compiled_list, list_globals) =
                compile_expression_with_context(&list.value, state, loop_vars)?;

            // Determine type of the list to push the correct variable to loop_vars
            // We need a temporary state to evaluate datatype
            let mut temp_program_stack = Vec::new();
            for global_name in &list_globals {
                let global_var = state.global_table().get(global_name).ok_or_else(|| {
                    AlthreadError::new(
                        ErrorType::VariableError,
                        Some(list.pos.clone()),
                        format!("Variable '{}' not found", global_name),
                    )
                })?;
                temp_program_stack.push(global_var.clone());
            }
            temp_program_stack.extend(loop_vars.clone());

            let mut temp_state = CompilerState::new_with_context(state.context.clone());
            temp_state.program_stack = temp_program_stack;
            temp_state.global_table = state.global_table.clone();

            let list_type = compiled_list.datatype(&temp_state).map_err(|e| {
                AlthreadError::new(
                    ErrorType::ExpressionError,
                    Some(list.pos.clone()),
                    format!("Cannot determine type of list expression: {}", e),
                )
            })?;

            let element_type = match list_type {
                DataType::List(inner) => *inner,
                _ => {
                    return Err(AlthreadError::new(
                        ErrorType::ExpressionError,
                        Some(list.pos.clone()),
                        format!("For loop expects a list, got {:?}", list_type),
                    ))
                }
            };

            // Push loop variable
            loop_vars.push(Variable {
                name: var_name.clone(),
                mutable: false,
                datatype: element_type,
                depth: 0,
                declare_pos: None,
            });

            // Compile body
            let compiled_body = compile_ltl_node(body, state, loop_vars)?;

            // Pop loop variable
            loop_vars.pop();

            Ok(CompiledLtlExpression::ForLoop {
                list_expression: compiled_list,
                list_read_variables: list_globals,
                loop_var_name: var_name.clone(),
                body: Box::new(compiled_body),
            })
        }
    }
}

fn compile_predicate(
    expr: &Expression,
    state: &CompilerState,
    loop_vars: &Vec<Variable>,
) -> AlthreadResult<CompiledLtlExpression> {
    // Compile the expression first, which includes validation
    let (expression, read_variables) = compile_expression_with_context(expr, state, loop_vars)?;
    
    // Type-check: ensure the expression returns a boolean
    let mut temp_stack = Vec::new();
    
    // Build temp_stack with globals + loop_vars (same order as compile_expression_with_context)
    for var_name in &read_variables {
        if let Some(loop_var) = loop_vars.iter().find(|v| &v.name == var_name) {
            temp_stack.push(loop_var.clone());
        } else if let Some(global_var) = state.global_table().get(var_name) {
            temp_stack.push(global_var.clone());
        }
    }
    
    let mut temp_state = CompilerState::new_with_context(state.context.clone());
    temp_state.program_stack = temp_stack;
    temp_state.global_table = state.global_table.clone();
    temp_state.in_condition_block = true;
    temp_state.programs_code = state.programs_code.clone();
    
    let expr_type = expression.datatype(&temp_state).map_err(|e| {
        AlthreadError::new(
            ErrorType::TypeError,
            None,
            format!("Invalid LTL predicate: {}", e),
        )
    })?;
    
    if expr_type != DataType::Boolean {
        return Err(AlthreadError::new(
            ErrorType::TypeError,
            None,
            format!("LTL predicate must be boolean, got {:?}", expr_type),
        ));
    }

    Ok(CompiledLtlExpression::Predicate {
        expression,
        read_variables,
        scope_mapping: None, // TODO
    })
}

fn compile_expression_with_context(
    expr: &Expression,
    state: &CompilerState,
    loop_vars: &Vec<Variable>,
) -> AlthreadResult<(LocalExpressionNode, Vec<String>)> {
    let mut dependencies = WaitDependency::new();
    expr.add_dependencies(&mut dependencies);
    let used_vars = dependencies.variables;

    let mut globals = Vec::new();
    let mut temp_stack = Vec::new();

    // 1. Identify globals vs loop vars
    for var_name in used_vars {
        if loop_vars.iter().any(|v| v.name == var_name) {
            // It's a loop variable, covered by loop_vars
            continue;
        }

        if state.global_table().contains_key(&var_name) {
            globals.push(var_name);
        } else {
            // Cannot find variable.
            // Note: Expression compilation will fail later with precise error,
            // but we can error here too.
            return Err(AlthreadError::new(
                ErrorType::VariableError,
                None, // We don't have pos here easily without passing Node
                format!("Variable '{}' not found", var_name),
            ));
        }
    }

    // Sort globals for deterministic stack order
    globals.sort();

    // 2. Build temporary stack: [Globals..., LoopVars...]
    // Note: The stack order matters for index resolution.
    // read_variables will be the list of globals + loop vars to push at bottom of stack.
    // loop_vars are already on top of them in the recurrence.

    for global_name in &globals {
        let global_var = state.global_table().get(global_name).unwrap();
        temp_stack.push(global_var.clone());
    }

    temp_stack.extend(loop_vars.clone());

    let compiled_expr = LocalExpressionNode::from_expression(expr, &temp_stack)?;

    // read_variables must match the stack layout: globals then loop vars
    let mut read_variables = globals;
    for v in loop_vars {
        read_variables.push(v.name.clone());
    }

    Ok((compiled_expr, read_variables))
}
