// Example demonstrating how to use the LTL evaluator and monitor
// This is a simplified example showing the API usage

#[cfg(test)]
mod usage_examples {
    use crate::{
        ast::token::literal::Literal,
        checker::ltl::{
            automaton::BuchiAutomaton,
            compiled::CompiledLtlExpression,
            monitor::{LtlMonitor, MonitoringState},
        },
    };
    use std::collections::HashMap;

    /// Example 1: Creating and using a simple monitor
    #[test]
    fn example_simple_monitor() {
        // Step 1: Create a simple LTL formula: [] (x > 0)
        let formula = CompiledLtlExpression::Always(Box::new(CompiledLtlExpression::Boolean(true)));

        // Step 2: Build the Büchi automaton from the formula
        let automaton = BuchiAutomaton::new(formula);

        // Step 3: Create a monitor with no variable bindings
        let bindings = HashMap::new();
        let initial_state = automaton.initial_states[0];
        let monitor = LtlMonitor::new(initial_state, bindings);

        // Verify the monitor was created correctly
        assert_eq!(monitor.current_state_id, initial_state);
        assert!(monitor.bindings.is_empty());
    }

    /// Example 2: Monitor with variable bindings (for process monitoring)
    #[test]
    fn example_monitor_with_bindings() {
        // Scenario: Monitoring a formula like "for p in $.procs: [] (p.x > 0)"
        // Each process gets its own monitor instance

        let formula = CompiledLtlExpression::Always(Box::new(CompiledLtlExpression::Boolean(true)));

        let automaton = BuchiAutomaton::new(formula);

        // Create a monitor for process with PID 42
        let mut bindings = HashMap::new();
        bindings.insert(
            "p".to_string(),
            Literal::Process("MyProgram".to_string(), 42),
        );

        let monitor = LtlMonitor::new(automaton.initial_states[0], bindings.clone());

        // Verify bindings are stored correctly
        assert_eq!(
            monitor.bindings.get("p"),
            Some(&Literal::Process("MyProgram".to_string(), 42))
        );
    }

    /// Example 3: Managing multiple monitors
    #[test]
    fn example_monitoring_state() {
        // Scenario: We have 2 LTL formulas to check
        let num_formulas = 2;
        let mut monitoring_state = MonitoringState::new(num_formulas);

        // Create automaton for first formula
        let formula1 =
            CompiledLtlExpression::Always(Box::new(CompiledLtlExpression::Boolean(true)));
        let automaton1 = BuchiAutomaton::new(formula1);

        // Add monitors for different processes
        let mut bindings1 = HashMap::new();
        bindings1.insert("p".to_string(), Literal::Process("Prog1".to_string(), 1));
        monitoring_state
            .add_monitor(0, &automaton1, bindings1)
            .unwrap();

        let mut bindings2 = HashMap::new();
        bindings2.insert("p".to_string(), Literal::Process("Prog1".to_string(), 2));
        monitoring_state
            .add_monitor(0, &automaton1, bindings2)
            .unwrap();

        // Verify we have 2 monitors for formula 0
        assert_eq!(monitoring_state.monitors_per_formula[0].len(), 2);
        assert_eq!(monitoring_state.monitors_per_formula[1].len(), 0);
    }

    /// Example 4: Evaluating simple predicates
    #[test]
    fn example_evaluate_boolean_predicate() {
        // Test evaluating a simple boolean
        let expr = CompiledLtlExpression::Boolean(true);
        // Note: We can't easily create a VM in unit tests,
        // but the API would be:
        // let result = evaluate_ltl_predicate(&expr, &vm, &bindings).unwrap();
        // assert!(result);

        // For now, just verify the expression structure
        assert!(matches!(expr, CompiledLtlExpression::Boolean(true)));
    }

    /// Example 5: Evaluating logical operators
    #[test]
    fn example_evaluate_logical_operators() {
        // Test AND operator
        let and_expr = CompiledLtlExpression::And(
            Box::new(CompiledLtlExpression::Boolean(true)),
            Box::new(CompiledLtlExpression::Boolean(false)),
        );

        // In real usage: evaluate_ltl_predicate(&and_expr, &vm, &bindings)
        // Would return false because true && false = false

        // Test OR operator
        let or_expr = CompiledLtlExpression::Or(
            Box::new(CompiledLtlExpression::Boolean(true)),
            Box::new(CompiledLtlExpression::Boolean(false)),
        );

        // Would return true because true || false = true

        // Test NOT operator
        let not_expr = CompiledLtlExpression::Not(Box::new(CompiledLtlExpression::Boolean(true)));

        // Would return false because !true = false

        // Verify structures are correct
        assert!(matches!(and_expr, CompiledLtlExpression::And(_, _)));
        assert!(matches!(or_expr, CompiledLtlExpression::Or(_, _)));
        assert!(matches!(not_expr, CompiledLtlExpression::Not(_)));
    }

    /// Example 6: Workflow for LTL verification
    #[test]
    fn example_verification_workflow() {
        // This shows the typical workflow (simplified)

        // 1. Parse and compile LTL formulas (already done by compiler)
        let formula = CompiledLtlExpression::Always(Box::new(CompiledLtlExpression::Boolean(true)));

        // 2. Build Büchi automaton
        let automaton = BuchiAutomaton::new(formula);
        assert!(!automaton.states.is_empty());
        assert!(!automaton.initial_states.is_empty());

        // 3. Initialize monitoring state
        let mut monitoring_state = MonitoringState::new(1); // 1 formula

        // 4. Add initial monitors (one per top-level quantifier binding)
        let bindings = HashMap::new();
        monitoring_state
            .add_monitor(0, &automaton, bindings)
            .unwrap();

        // 5. During state exploration, advance monitors
        // (This would happen in checker/mod.rs)
        // let violations = monitoring_state.advance_all(&vm, &[automaton]).unwrap();

        // 6. Check for violations
        // if !violations.is_empty() { /* violation found */ }

        // Verify setup is correct
        assert_eq!(monitoring_state.monitors_per_formula.len(), 1);
        assert_eq!(monitoring_state.monitors_per_formula[0].len(), 1);
    }
}
