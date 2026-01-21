// Integration tests for LTL checker

#[cfg(test)]
mod tests {
    use crate::{
        checker::{
            check_program, ltl::automaton::BuchiAutomaton, ltl::compiled::CompiledLtlExpression,
        },
        compiler::CompiledProject,
        error::AlthreadResult,
    };

    #[test]
    fn test_ltl_checker_integration() -> AlthreadResult<()> {
        // Create a simple formula: [] (true)
        let formula = CompiledLtlExpression::Always(Box::new(CompiledLtlExpression::Boolean(true)));

        // Build automaton
        let automaton = BuchiAutomaton::new(formula.clone());

        // Verify automaton was created
        assert!(!automaton.states.is_empty());
        assert!(!automaton.initial_states.is_empty());

        Ok(())
    }

    #[test]
    fn test_no_ltl_formulas_uses_regular_checker() -> AlthreadResult<()> {
        // Create an empty project
        let project = CompiledProject::default_for_testing();

        // Check without LTL (should use regular checker)
        let (_violations, _graph) = check_program(&project, Some(10))?;

        // Should complete without errors
        Ok(())
    }
}
