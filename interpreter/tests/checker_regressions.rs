use althread::{
    checker::{check_program, ltl::compiled::CompiledLtlExpression as L, StateGraph, StateLink},
    compiler::CompiledProject,
    module_resolver::StandardFileSystem,
    parser,
};
use std::{collections::HashMap, path::Path};

fn compile(source: &str) -> CompiledProject {
    let mut inputs = HashMap::from([(String::new(), source.to_string())]);
    parser::parse_ast(source, "")
        .unwrap()
        .compile(Path::new(""), StandardFileSystem, &mut inputs)
        .unwrap()
}

/// Follow the exact returned edges, including the terminal stutter, and verify
/// that the advertised loop is nonempty and closes at its starting VM state.
fn assert_lasso(path: &[StateLink], graph: &StateGraph<'_>) {
    let cycle_start = graph.violation_cycle_start.expect("LTL witness has a loop");
    assert!(cycle_start < path.len());
    let mut current = graph.initial_state;
    let mut loop_state = None;
    for (index, edge) in path.iter().enumerate() {
        if index == cycle_start {
            loop_state = Some(current);
        }
        let node = &graph.nodes[current];
        if edge.name == "_stutter_" {
            assert!(node.expanded && node.successors.is_empty());
            assert_eq!(edge.to, current);
        } else {
            assert!(node.successors.iter().any(|candidate| {
                candidate.to == edge.to
                    && candidate.pid == edge.pid
                    && candidate.instructions == edge.instructions
                    && candidate.actions == edge.actions
            }));
        }
        current = edge.to;
    }
    assert_eq!(Some(current), loop_state);
}

// Next and temporal Exists are tested through the compiled API because the
// current source parser does not expose those temporal forms.
fn make_existential(project: &mut CompiledProject) {
    let L::ForLoop {
        list_expression,
        list_read_variables,
        loop_var_name,
        body,
    } = project.compiled_ltl_formulas.remove(0)
    else {
        panic!("expected a quantified test formula")
    };
    project.compiled_ltl_formulas.push(L::Exists {
        list_expression,
        list_read_variables,
        loop_var_name,
        body,
    });
}

#[test]
fn gba_requires_every_acceptance_set() {
    let project = compile("main { print(1); } check { (always true) || (always false); }");
    let (path, graph) = check_program(&project, Some(100)).unwrap();
    assert!(graph.exhaustive);
    assert!(
        path.is_empty(),
        "A tautology was reported as violated: {path:?}"
    );
}

#[test]
fn gba_accepts_when_both_eventual_obligations_are_met() {
    let project = compile("shared { let X: int = 0; } main { loop { X = 1 - X; } } check { (always X == 0) || (always X == 1); }");
    let (path, graph) = check_program(&project, Some(100)).unwrap();
    assert!(graph.exhaustive);
    assert_lasso(&path, &graph);
    // The negated property requires eventually seeing both values. Its witness
    // must visit them, rather than accepting just one of its acceptance sets.
    let mut values = std::collections::HashSet::from([graph.vm(0).globals["X"].clone()]);
    for edge in &path {
        values.insert(graph.vm(edge.to).globals["X"].clone());
    }
    assert_eq!(values.len(), 2);
}

#[test]
fn independent_formulas_still_report_a_failing_property() {
    let project = compile("shared { let X: int = 0; } main { X = 1; } check { eventually X == 1; eventually X == 2; }");
    let (path, graph) = check_program(&project, Some(100)).unwrap();
    assert_lasso(&path, &graph);
}

#[test]
fn true_formula_is_valid() {
    let mut project = compile("main {}");
    project.compiled_ltl_formulas.push(L::Boolean(true));
    let (path, _) =
        check_program(&project, Some(100)).expect("Empty counterexample automaton means valid");
    assert!(path.is_empty());
}

#[test]
fn exists_true_is_valid() {
    let mut project = compile("main { print(1); } check { for p in 0..1 { eventually true }; }");
    make_existential(&mut project);
    let (path, _) = check_program(&project, Some(100)).unwrap();
    assert!(
        path.is_empty(),
        "An existential with a satisfying witness was reported as violated"
    );
}

#[test]
fn spawn_does_not_skip_first_predicate() {
    let project = compile("program Worker() { await false; } main { run Worker(); } check { for p in $.procs.Worker { p.reaches(end) }; }");
    let (path, _) = check_program(&project, Some(100)).unwrap();
    assert!(!path.is_empty(), "Worker is not at end when first observed");
}

#[test]
fn liveness_witness_contains_a_cycle() {
    let project = compile("shared { let Done: bool = false; } main {} check { eventually Done; }");
    let (path, graph) = check_program(&project, Some(100)).unwrap();
    assert_lasso(&path, &graph);
    assert!(!path.is_empty());
    let mut states = std::collections::HashSet::from([0]);
    assert!(
        path.iter().any(|edge| !states.insert(edge.to)),
        "Liveness counterexample has no cycle: {path:?}"
    );
}

#[test]
fn state_limit_is_a_hard_limit() {
    let project = compile(
        "shared { let X: int = 0; } program A() { X = 1; } main { run A(); run A(); run A(); }",
    );
    let (_, graph) = check_program(&project, Some(3)).unwrap();
    assert!(
        graph.nodes.len() <= 3,
        "Limit 3 yielded {} states",
        graph.nodes.len()
    );
}

#[test]
fn function_return_addresses_are_part_of_state() {
    let project = compile("shared { let X: int = 0; } fn f() -> void { X = 0; } main { f(); f(); X = 1; } check { eventually X == 1; }");
    let mut vm = althread::vm::VM::new(&project);
    vm.start(0);
    for _ in 0..20 {
        let successors = vm.next().unwrap();
        if successors.is_empty() {
            break;
        }
        assert_eq!(successors.len(), 1);
        vm = successors.into_iter().next().unwrap().4;
    }
    assert_eq!(
        vm.globals["X"],
        althread::ast::token::literal::Literal::Int(1)
    );
    let (path, graph) = check_program(&project, Some(100)).unwrap();
    assert!(
        path.is_empty(),
        "Deterministically terminating function calls got merged: {path:?}, {} states",
        graph.nodes.len()
    );
}

#[test]
fn function_state_merging_must_not_hide_invariant_failure() {
    let project = compile("shared { let X: int = 0; } fn f() -> void { X = 0; } main { f(); f(); X = 1; } always { X == 0; }");
    let (path, graph) = check_program(&project, Some(100)).unwrap();
    assert!(graph.exhaustive);
    assert!(
        !path.is_empty(),
        "The checker omitted the reachable state X=1"
    );
    assert!(graph
        .vm(path.last().unwrap().to)
        .check_invariants()
        .is_err());
}

#[test]
fn exact_state_limit_can_still_be_exhaustive() {
    let project = compile("main { print(1); }");
    let (_, graph) = check_program(&project, Some(2)).unwrap();
    assert_eq!(graph.nodes.len(), 2);
    assert!(
        graph.exhaustive,
        "Both reachable states fit within the limit"
    );
}

#[test]
fn new_monitor_consumes_spawn_state_once() {
    let mut project = compile("program Worker() { print(1); } main { run Worker(); } check { for p in $.procs.Worker { p.reaches(end) }; }");
    if let L::ForLoop { body, .. } = &mut project.compiled_ltl_formulas[0] {
        **body = L::Next(body.clone());
    }
    let (path, _) = check_program(&project, Some(100)).unwrap();
    assert!(
        path.is_empty(),
        "Worker reaches end in the next step after spawn"
    );
}

#[test]
fn liveness_witness_includes_loop() {
    let project =
        compile("shared { let Done: bool = false; } main { print(1); } check { eventually Done; }");
    let (path, graph) = check_program(&project, Some(100)).unwrap();
    assert_lasso(&path, &graph);
    assert!(!path.is_empty());
    let mut states = std::collections::HashSet::from([0]);
    assert!(
        path.iter().any(|edge| !states.insert(edge.to)),
        "Liveness counterexample has no cycle: {path:?}"
    );
}

#[test]
fn cyclic_execution_has_a_replayable_lasso() {
    let project = compile(
        "shared { let X: int = 0; } main { loop { X = 1 - X; } } check { eventually X == 2; }",
    );
    let (path, graph) = check_program(&project, Some(100)).unwrap();
    assert!(graph.exhaustive);
    assert_lasso(&path, &graph);
    assert!(path.iter().all(|edge| edge.name != "_stutter_"));
}

#[test]
fn witness_preserves_the_violating_history_when_vm_paths_merge() {
    let project = compile(
        r#"
shared { let X: int = 0; let Gate: bool = false; }
program Chooser() {
    if Gate {
        X = 1;
        X = 0;
    } else {
        X = 0;
        X = 0;
        X = 0;
    }
}
program Setter() { Gate = true; }
main { run Chooser(); run Setter(); }
check { eventually X == 1; }
"#,
    );
    let (path, graph) = check_program(&project, Some(1000)).unwrap();
    assert!(graph.exhaustive);
    assert_lasso(&path, &graph);
    // Both branches merge into the same terminal VM state. The shorter branch
    // satisfies the property; the counterexample must follow the longer one.
    for edge in &path {
        assert_eq!(
            graph.vm(edge.to).globals["X"],
            althread::ast::token::literal::Literal::Int(0)
        );
    }
}

#[test]
fn existential_keeps_a_satisfied_instance_among_failing_instances() {
    let mut project = compile("main { print(1); } check { for p in 0..2 { eventually p == 1 }; }");
    make_existential(&mut project);
    let (path, graph) = check_program(&project, Some(100)).unwrap();
    assert!(graph.exhaustive && path.is_empty());
}

#[test]
fn existential_initial_alternatives_are_not_separate_obligations() {
    let mut project = compile(
        "main { print(1); } check { for p in 0..2 { (always p == 0) && (eventually p == 1) }; }",
    );
    make_existential(&mut project);
    let (path, graph) = check_program(&project, Some(100)).unwrap();
    // Neither p=0 nor p=1 can satisfy both conjuncts.
    assert_lasso(&path, &graph);
}

#[test]
fn empty_quantified_domains_have_opposite_verdicts() {
    let mut project = compile("main {} check { for p in 0..0 { eventually true }; }");
    let (path, _) = check_program(&project, Some(100)).unwrap();
    assert!(path.is_empty());
    make_existential(&mut project);
    let (path, graph) = check_program(&project, Some(100)).unwrap();
    assert_lasso(&path, &graph);
}

#[test]
fn false_formula_at_initial_state_has_a_witness() {
    let mut project = compile("main {}");
    project.compiled_ltl_formulas.push(L::Boolean(false));
    let (path, graph) = check_program(&project, Some(1)).unwrap();
    assert!(graph.exhaustive);
    assert_lasso(&path, &graph);
}

#[test]
fn zero_state_limit_is_rejected() {
    let project = compile("main {}");
    assert!(check_program(&project, Some(0)).is_err());
}

#[test]
fn budget_does_not_hide_a_cycle_between_known_states() {
    let project = compile(
        "shared { let X: int = 0; } main { loop { X = 1 - X; } } check { eventually X == 2; }",
    );
    let (_, full) = check_program(&project, None).unwrap();
    let (path, graph) = check_program(&project, Some(full.nodes.len())).unwrap();
    assert!(graph.exhaustive);
    assert_lasso(&path, &graph);
}

#[test]
#[ignore = "manual release-mode benchmark; timings are not test assertions"]
fn distinct_formula_scaling() {
    for count in [1, 4, 8, 12] {
        let formulas = (1..=count)
            .map(|i| format!("always eventually X >= {i};"))
            .collect::<Vec<_>>()
            .join(" ");
        let project = compile(&format!(
            "shared {{ let X: int = 0; }} main {{ print(1); X = 100; }} check {{ {formulas} }}"
        ));
        let mut samples = Vec::new();
        let mut states = 0;
        for _ in 0..3 {
            let start = std::time::Instant::now();
            let (path, graph) = check_program(&project, Some(100)).unwrap();
            samples.push(start.elapsed());
            assert!(path.is_empty());
            states = graph.nodes.len();
        }
        samples.sort();
        println!(
            "DISTINCT_SCALING formulas={count} vm_states={states} median={:?}",
            samples[1]
        );
    }
}
