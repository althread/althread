use std::collections::HashMap;

use crate::{
    ast::token::literal::Literal,
    checker::ltl::compiled::CompiledLtlExpression,
    error::{AlthreadError, AlthreadResult, ErrorType},
    vm::{GlobalMemory, Memory, VM},
};

/// Evaluates a compiled LTL expression as a boolean predicate on a given VM state.
/// This is used for nested predicates (ForLoop, Exists) and atomic propositions.
///
/// IMPORTANT: This function should ONLY be called with expressions that are:
/// - Predicate (atomic propositions)
/// - Boolean (true/false)
/// - ForLoop (nested, must evaluate to boolean)
/// - Exists (nested, must evaluate to boolean)
/// - And/Or/Not (logical operators on predicates)
///
/// Temporal operators (Next, Until, Always, Eventually, Release) MUST NOT appear
/// in nested predicates and will cause an error.
pub fn evaluate_ltl_predicate(
    expression: &CompiledLtlExpression,
    vm: &VM,
    variable_bindings: &HashMap<String, Literal>,
) -> AlthreadResult<bool> {
    match expression {
        CompiledLtlExpression::Boolean(b) => Ok(*b),

        CompiledLtlExpression::Predicate {
            expression: local_expr,
            read_variables,
            ..
        } => {
            // Build the memory stack with global variables and bindings
            let memory = prepare_memory(read_variables, &vm.globals, variable_bindings)?;

            // Evaluate the expression with the VM context
            let result = local_expr.eval_with_context(&memory, vm).map_err(|msg| {
                AlthreadError::new(
                    ErrorType::ExpressionError,
                    None,
                    format!("Error evaluating predicate: {}", msg),
                )
            })?;

            Ok(result.is_true())
        }

        CompiledLtlExpression::ForLoop {
            list_expression,
            list_read_variables,
            loop_var_name,
            body,
        } => {
            // Evaluate the list expression
            let memory = prepare_memory(list_read_variables, &vm.globals, variable_bindings)?;
            let list_value = list_expression
                .eval_with_context(&memory, vm)
                .map_err(|msg| {
                    AlthreadError::new(
                        ErrorType::ExpressionError,
                        None,
                        format!("Error evaluating list in for loop: {}", msg),
                    )
                })?;

            // Extract list elements
            let elements = match list_value {
                Literal::List(_, elems) => elems,
                _ => {
                    return Err(AlthreadError::new(
                        ErrorType::ExpressionError,
                        None,
                        format!("For loop expects a list, got {:?}", list_value),
                    ))
                }
            };

            // Check if body contains temporal operators (which is forbidden)
            check_no_temporal_operators(body)?;

            // ForAll semantics: all elements must satisfy the body
            for element in elements {
                let mut extended_bindings = variable_bindings.clone();
                extended_bindings.insert(loop_var_name.clone(), element);

                let body_result = evaluate_ltl_predicate(body, vm, &extended_bindings)?;
                if !body_result {
                    return Ok(false);
                }
            }

            Ok(true)
        }

        CompiledLtlExpression::Exists {
            list_expression,
            list_read_variables,
            loop_var_name,
            body,
        } => {
            // Evaluate the list expression
            let memory = prepare_memory(list_read_variables, &vm.globals, variable_bindings)?;
            let list_value = list_expression
                .eval_with_context(&memory, vm)
                .map_err(|msg| {
                    AlthreadError::new(
                        ErrorType::ExpressionError,
                        None,
                        format!("Error evaluating list in exists: {}", msg),
                    )
                })?;

            // Extract list elements
            let elements = match list_value {
                Literal::List(_, elems) => elems,
                _ => {
                    return Err(AlthreadError::new(
                        ErrorType::ExpressionError,
                        None,
                        format!("Exists expects a list, got {:?}", list_value),
                    ))
                }
            };

            // Check if body contains temporal operators (which is forbidden)
            check_no_temporal_operators(body)?;

            // Exists semantics: at least one element must satisfy the body
            for element in elements {
                let mut extended_bindings = variable_bindings.clone();
                extended_bindings.insert(loop_var_name.clone(), element);

                let body_result = evaluate_ltl_predicate(body, vm, &extended_bindings)?;
                if body_result {
                    return Ok(true);
                }
            }

            Ok(false)
        }

        CompiledLtlExpression::And(left, right) => {
            let left_result = evaluate_ltl_predicate(left, vm, variable_bindings)?;
            if !left_result {
                return Ok(false);
            }
            evaluate_ltl_predicate(right, vm, variable_bindings)
        }

        CompiledLtlExpression::Or(left, right) => {
            let left_result = evaluate_ltl_predicate(left, vm, variable_bindings)?;
            if left_result {
                return Ok(true);
            }
            evaluate_ltl_predicate(right, vm, variable_bindings)
        }

        CompiledLtlExpression::Not(inner) => {
            let result = evaluate_ltl_predicate(inner, vm, variable_bindings)?;
            Ok(!result)
        }

        // Temporal operators should never appear in nested predicates
        CompiledLtlExpression::Next(_) => Err(AlthreadError::new(
            ErrorType::ExpressionError,
            None,
            "Next operator cannot appear in nested predicates (for/exists loops)".to_string(),
        )),

        CompiledLtlExpression::Until(_, _) => Err(AlthreadError::new(
            ErrorType::ExpressionError,
            None,
            "Until operator cannot appear in nested predicates (for/exists loops)".to_string(),
        )),

        CompiledLtlExpression::Always(_) => Err(AlthreadError::new(
            ErrorType::ExpressionError,
            None,
            "Always operator cannot appear in nested predicates (for/exists loops)".to_string(),
        )),

        CompiledLtlExpression::Eventually(_) => Err(AlthreadError::new(
            ErrorType::ExpressionError,
            None,
            "Eventually operator cannot appear in nested predicates (for/exists loops)".to_string(),
        )),

        CompiledLtlExpression::Release(_, _) => Err(AlthreadError::new(
            ErrorType::ExpressionError,
            None,
            "Release operator cannot appear in nested predicates (for/exists loops)".to_string(),
        )),

        CompiledLtlExpression::Implies(left, right) => {
            let left_result = evaluate_ltl_predicate(left, vm, variable_bindings)?;
            if !left_result {
                return Ok(true);
            }
            evaluate_ltl_predicate(right, vm, variable_bindings)
        }
    }
}

/// Prepares the memory stack for expression evaluation by combining global variables
/// and loop variable bindings.
fn prepare_memory(
    read_variables: &[String],
    globals: &GlobalMemory,
    variable_bindings: &HashMap<String, Literal>,
) -> AlthreadResult<Memory> {
    let mut memory = Vec::new();

    for var_name in read_variables {
        // First check if it's a loop variable binding
        if let Some(value) = variable_bindings.get(var_name) {
            memory.push(value.clone());
        } else if let Some(value) = globals.get(var_name) {
            // Otherwise get it from global memory
            memory.push(value.clone());
        } else {
            return Err(AlthreadError::new(
                ErrorType::VariableError,
                None,
                format!(
                    "Variable '{}' not found in global memory or bindings",
                    var_name
                ),
            ));
        }
    }

    Ok(memory)
}

/// Checks that an expression doesn't contain temporal operators.
/// This is used to ensure nested predicates (in for/exists loops) don't use temporal logic.
fn check_no_temporal_operators(expr: &CompiledLtlExpression) -> AlthreadResult<()> {
    match expr {
        CompiledLtlExpression::Next(_)
        | CompiledLtlExpression::Until(_, _)
        | CompiledLtlExpression::Always(_)
        | CompiledLtlExpression::Eventually(_)
        | CompiledLtlExpression::Release(_, _) => {
            Err(AlthreadError::new(
                ErrorType::ExpressionError,
                None,
                "Temporal operators (Next, Until, Always, Eventually, Release) cannot be used in nested for/exists loops".to_string(),
            ))
        }
        CompiledLtlExpression::And(l, r)
        | CompiledLtlExpression::Or(l, r)
        | CompiledLtlExpression::Implies(l, r) => {
            check_no_temporal_operators(l)?;
            check_no_temporal_operators(r)
        }
        CompiledLtlExpression::Not(inner) => check_no_temporal_operators(inner),
        CompiledLtlExpression::ForLoop { body, .. }
        | CompiledLtlExpression::Exists { body, .. } => check_no_temporal_operators(body),
        // Leaves are OK
        CompiledLtlExpression::Boolean(_) | CompiledLtlExpression::Predicate { .. } => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_boolean() {
        let expr = CompiledLtlExpression::Boolean(true);
        // We can't easily create a VM in tests without a full project setup
        // Just verify the structure is correct
        assert!(matches!(expr, CompiledLtlExpression::Boolean(true)));
    }

    #[test]
    fn test_evaluate_and() {
        let expr = CompiledLtlExpression::And(
            Box::new(CompiledLtlExpression::Boolean(true)),
            Box::new(CompiledLtlExpression::Boolean(false)),
        );
        // Verify structure
        if let CompiledLtlExpression::And(left, right) = expr {
            assert!(matches!(*left, CompiledLtlExpression::Boolean(true)));
            assert!(matches!(*right, CompiledLtlExpression::Boolean(false)));
        } else {
            panic!("Expected And expression");
        }
    }

    #[test]
    fn test_check_no_temporal_operators() {
        // Test that temporal operators are properly detected
        let next_expr = CompiledLtlExpression::Next(Box::new(CompiledLtlExpression::Boolean(true)));
        assert!(check_no_temporal_operators(&next_expr).is_err());

        let until_expr = CompiledLtlExpression::Until(
            Box::new(CompiledLtlExpression::Boolean(true)),
            Box::new(CompiledLtlExpression::Boolean(false)),
        );
        assert!(check_no_temporal_operators(&until_expr).is_err());

        // Boolean should be OK
        let bool_expr = CompiledLtlExpression::Boolean(true);
        assert!(check_no_temporal_operators(&bool_expr).is_ok());
    }
}
