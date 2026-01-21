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
    use crate::{ast::Ast, module_resolver::StandardFileSystem};
    use std::collections::HashMap;
    use std::path::Path;

    fn compile_from_source(source: &str) -> CompiledProject {
        let mut input_map = HashMap::new();
        input_map.insert("".to_string(), source.to_string());

        let pairs = crate::parser::parse(source, "").unwrap();
        let ast = Ast::build(pairs, "").unwrap();
        ast.compile(Path::new(""), StandardFileSystem, &mut input_map)
            .unwrap()
    }

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

    #[test]
    fn test_ltl_top_level_for_eventually_violation_on_deadlock() -> AlthreadResult<()> {
        let source = r#"
shared {
    let Flag: bool = false;
}

program Worker() {
    await Flag;
}

main {
    run Worker();
}

check {
    for p in $.procs.Worker { eventually p.reaches(end) };
}
"#;

        let project = compile_from_source(source);
        let (violations, _graph) = check_program(&project, Some(1000))?;
        assert!(!violations.is_empty(), "Expected LTL violation on deadlock");
        Ok(())
    }

    #[test]
    fn test_ltl_top_level_for_eventually_passes_when_all_end() -> AlthreadResult<()> {
        let source = r#"
program Worker() {
    // ends immediately
}

main {
    run Worker();
}

check {
    for p in $.procs.Worker { eventually p.reaches(end) };
}
"#;

        let project = compile_from_source(source);
        let (violations, _graph) = check_program(&project, Some(1000))?;
        assert!(violations.is_empty(), "Expected no LTL violation");
        Ok(())
    }
}
