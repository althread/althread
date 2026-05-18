//! Debug and diagnostic utilities for LTL verification.
//!
//! This module provides functions to output intermediate data during LTL verification:
//! - Negated LTL formulas
//! - Büchi automaton visualization
//! - State graph with relevant boolean values
//! - Counter-example search trace

use std::collections::HashMap;
use std::fmt::Write;

use crate::ast::token::literal::Literal;
use crate::vm::VM;

use super::automaton::BuchiAutomaton;
use super::compiled::CompiledLtlExpression;
use super::evaluator::evaluate_ltl_predicate;
use super::monitor::MonitoringState;

/// Configuration for debug output
#[derive(Debug, Clone, Default)]
pub struct DebugConfig {
    /// Output negated formulas
    pub show_negated_formulas: bool,
    /// Output Büchi automatons in DOT format
    pub show_automaton: bool,
    /// Output state graph
    pub show_state_graph: bool,
    /// Output counter-example search trace
    pub show_search_trace: bool,
    /// Output predicate evaluations at each state
    pub show_predicate_values: bool,
}

impl DebugConfig {
    pub fn all() -> Self {
        Self {
            show_negated_formulas: true,
            show_automaton: true,
            show_state_graph: true,
            show_search_trace: true,
            show_predicate_values: true,
        }
    }
}

/// Formats a compiled LTL expression for human-readable output
pub fn format_formula(formula: &CompiledLtlExpression) -> String {
    match formula {
        CompiledLtlExpression::Always(e) => format!("□ ({})", format_formula(e)),
        CompiledLtlExpression::Eventually(e) => format!("◇ ({})", format_formula(e)),
        CompiledLtlExpression::Next(e) => format!("○ ({})", format_formula(e)),
        CompiledLtlExpression::Not(e) => format!("¬({})", format_formula(e)),
        CompiledLtlExpression::Until(l, r) => {
            format!("({}) U ({})", format_formula(l), format_formula(r))
        }
        CompiledLtlExpression::Release(l, r) => {
            format!("({}) R ({})", format_formula(l), format_formula(r))
        }
        CompiledLtlExpression::And(l, r) => {
            format!("({}) ∧ ({})", format_formula(l), format_formula(r))
        }
        CompiledLtlExpression::Or(l, r) => {
            format!("({}) ∨ ({})", format_formula(l), format_formula(r))
        }
        CompiledLtlExpression::Implies(l, r) => {
            format!("({}) → ({})", format_formula(l), format_formula(r))
        }
        CompiledLtlExpression::Boolean(b) => format!("{}", b),
        CompiledLtlExpression::Predicate { read_variables, .. } => {
            format!("P[{}]", read_variables.join(", "))
        }
        CompiledLtlExpression::ForLoop {
            loop_var_name,
            body,
            ..
        } => {
            format!("∀{}: {}", loop_var_name, format_formula(body))
        }
        CompiledLtlExpression::Exists {
            loop_var_name,
            body,
            ..
        } => {
            format!("∃{}: {}", loop_var_name, format_formula(body))
        }
    }
}

/// Formats the negation of an LTL formula (used to find counter-examples)
pub fn format_negated_formula(formula: &CompiledLtlExpression) -> String {
    let negated = formula.clone().negate();
    format_formula(&negated)
}

/// Generates a report of all negated formulas
pub fn generate_negated_formulas_report(formulas: &[CompiledLtlExpression]) -> String {
    let mut report = String::new();
    writeln!(
        report,
        "=== Negated LTL Formulas (for counter-example search) ==="
    )
    .unwrap();
    writeln!(report).unwrap();

    for (i, formula) in formulas.iter().enumerate() {
        writeln!(report, "Formula #{}: {}", i + 1, format_formula(formula)).unwrap();
        writeln!(report, "Negated:    {}", format_negated_formula(formula)).unwrap();
        writeln!(report).unwrap();
    }

    report
}

/// Generates a DOT representation of a Büchi automaton
pub fn automaton_to_dot(automaton: &BuchiAutomaton, formula_index: usize) -> String {
    let mut dot = String::new();

    writeln!(dot, "digraph BuchiAutomaton{} {{", formula_index).unwrap();
    writeln!(dot, "  rankdir=LR;").unwrap();
    writeln!(dot, "  node [shape=circle];").unwrap();
    writeln!(dot).unwrap();

    // Initial states - add invisible start nodes
    for &init_id in &automaton.initial_states {
        writeln!(dot, "  _init_{} [shape=point];", init_id).unwrap();
        writeln!(dot, "  _init_{} -> S{};", init_id, init_id).unwrap();
    }
    writeln!(dot).unwrap();

    // States
    for state in &automaton.states {
        let shape = if !state.acceptance_sets.is_empty() {
            "doublecircle"
        } else {
            "circle"
        };

        // Format formulas for label
        let formulas_str: Vec<String> = state.formulas.iter().map(|f| format_formula(f)).collect();
        let label = if formulas_str.is_empty() {
            "true".to_string()
        } else {
            formulas_str.join("\\n")
        };

        let accept_label = if !state.acceptance_sets.is_empty() {
            format!(" (acc: {:?})", state.acceptance_sets)
        } else {
            String::new()
        };

        writeln!(
            dot,
            "  S{} [shape={}, label=\"S{}\\n{}{}\\n\"];",
            state.id, shape, state.id, label, accept_label
        )
        .unwrap();
    }
    writeln!(dot).unwrap();

    // Transitions
    for state in &automaton.states {
        for &target_id in &state.transitions {
            writeln!(dot, "  S{} -> S{};", state.id, target_id).unwrap();
        }
    }

    writeln!(dot, "}}").unwrap();

    dot
}

/// Generates a text summary of a Büchi automaton
pub fn automaton_summary(automaton: &BuchiAutomaton, formula_index: usize) -> String {
    let mut summary = String::new();

    writeln!(summary, "=== Büchi Automaton #{} ===", formula_index + 1).unwrap();
    writeln!(summary, "States: {}", automaton.states.len()).unwrap();
    writeln!(summary, "Initial states: {:?}", automaton.initial_states).unwrap();
    writeln!(
        summary,
        "Acceptance sets: {}",
        automaton.num_acceptance_sets
    )
    .unwrap();
    writeln!(summary).unwrap();

    for state in &automaton.states {
        let is_initial = automaton.initial_states.contains(&state.id);
        let initial_marker = if is_initial { " [INIT]" } else { "" };
        let accept_marker = if !state.acceptance_sets.is_empty() {
            format!(" [ACCEPT: {:?}]", state.acceptance_sets)
        } else {
            String::new()
        };

        writeln!(
            summary,
            "State {}{}{}:",
            state.id, initial_marker, accept_marker
        )
        .unwrap();

        writeln!(summary, "  Formulas:").unwrap();
        for f in &state.formulas {
            writeln!(summary, "    - {}", format_formula(f)).unwrap();
        }

        writeln!(summary, "  Transitions: {:?}", state.transitions).unwrap();
        writeln!(summary).unwrap();
    }

    summary
}

/// Generates a full automaton report for all formulas
pub fn generate_automaton_report(
    formulas: &[CompiledLtlExpression],
    automatons: &[BuchiAutomaton],
) -> String {
    let mut report = String::new();

    writeln!(report, "=== Büchi Automatons Report ===").unwrap();
    writeln!(report).unwrap();

    for (i, automaton) in automatons.iter().enumerate() {
        writeln!(report, "--- Formula #{} ---", i + 1).unwrap();
        writeln!(report, "Original: {}", format_formula(&formulas[i])).unwrap();
        writeln!(report, "Negated:  {}", format_negated_formula(&formulas[i])).unwrap();
        writeln!(report).unwrap();

        report.push_str(&automaton_summary(automaton, i));
        writeln!(report).unwrap();
    }

    report
}

/// Extracts all atomic predicates from a formula
pub fn extract_predicates(formula: &CompiledLtlExpression) -> Vec<CompiledLtlExpression> {
    let mut predicates = Vec::new();
    extract_predicates_recursive(formula, &mut predicates);
    predicates
}

fn extract_predicates_recursive(
    formula: &CompiledLtlExpression,
    acc: &mut Vec<CompiledLtlExpression>,
) {
    match formula {
        CompiledLtlExpression::Predicate { .. } => {
            if !acc.contains(formula) {
                acc.push(formula.clone());
            }
        }
        CompiledLtlExpression::Always(e)
        | CompiledLtlExpression::Eventually(e)
        | CompiledLtlExpression::Next(e)
        | CompiledLtlExpression::Not(e) => {
            extract_predicates_recursive(e, acc);
        }
        CompiledLtlExpression::Until(l, r)
        | CompiledLtlExpression::Release(l, r)
        | CompiledLtlExpression::And(l, r)
        | CompiledLtlExpression::Or(l, r)
        | CompiledLtlExpression::Implies(l, r) => {
            extract_predicates_recursive(l, acc);
            extract_predicates_recursive(r, acc);
        }
        CompiledLtlExpression::ForLoop { body, .. }
        | CompiledLtlExpression::Exists { body, .. } => {
            extract_predicates_recursive(body, acc);
        }
        CompiledLtlExpression::Boolean(_) => {}
    }
}

/// Evaluates all predicates on a VM state and returns their values
pub fn evaluate_predicates_on_state(
    predicates: &[CompiledLtlExpression],
    vm: &VM,
    bindings: &HashMap<String, Literal>,
) -> HashMap<String, bool> {
    let mut values = HashMap::new();

    for pred in predicates {
        if let CompiledLtlExpression::Predicate { read_variables, .. } = pred {
            let key = format!("P[{}]", read_variables.join(", "));
            match evaluate_ltl_predicate(pred, vm, bindings) {
                Ok(val) => {
                    values.insert(key, val);
                }
                Err(e) => {
                    values.insert(key, false);
                    log::warn!("Error evaluating predicate: {:?}", e);
                }
            }
        }
    }

    values
}

/// Formats a VM state with relevant predicate values for LTL formulas
pub fn format_vm_state_with_predicates(
    vm: &VM,
    formulas: &[CompiledLtlExpression],
    state_id: usize,
) -> String {
    let mut output = String::new();

    // Collect all predicates from all formulas
    let mut all_predicates = Vec::new();
    for formula in formulas {
        for pred in extract_predicates(formula) {
            if !all_predicates.contains(&pred) {
                all_predicates.push(pred);
            }
        }
    }

    // Format state header
    writeln!(output, "State S{}:", state_id).unwrap();

    // Global variables
    writeln!(output, "  Globals:").unwrap();
    for (name, value) in vm.globals.iter() {
        writeln!(output, "    {}: {:?}", name, value).unwrap();
    }

    // Predicate values
    let pred_values = evaluate_predicates_on_state(&all_predicates, vm, &HashMap::new());
    if !pred_values.is_empty() {
        writeln!(output, "  Predicates:").unwrap();
        for (pred_name, value) in &pred_values {
            writeln!(output, "    {} = {}", pred_name, value).unwrap();
        }
    }

    output
}

/// Formats the monitoring state
pub fn format_monitoring_state(
    monitors: &MonitoringState,
    automatons: &[BuchiAutomaton],
) -> String {
    let mut output = String::new();

    writeln!(output, "Monitoring State:").unwrap();

    for (formula_idx, formula_monitors) in monitors.monitors_per_formula.iter().enumerate() {
        writeln!(output, "  Formula #{}:", formula_idx + 1).unwrap();

        if formula_monitors.is_empty() {
            writeln!(output, "    (no active monitors)").unwrap();
        } else {
            for (i, monitor) in formula_monitors.iter().enumerate() {
                let automaton = &automatons[formula_idx];
                let is_accepting = monitor.is_accepting(automaton);
                let accept_marker = if is_accepting { " [ACCEPTING]" } else { "" };

                writeln!(
                    output,
                    "    Monitor {}: state={}{}, bindings={:?}",
                    i, monitor.current_state_id, accept_marker, monitor.bindings
                )
                .unwrap();
            }
        }
    }

    output
}

/// Log structure for tracking search progress
#[derive(Debug, Clone)]
pub struct SearchStep {
    pub step_number: usize,
    pub vm_state_id: usize,
    pub action: SearchAction,
    pub monitors_summary: String,
}

#[derive(Debug, Clone)]
pub enum SearchAction {
    Expand,
    Transition {
        from: usize,
        to: usize,
        edge_label: String,
    },
    AcceptingCycleFound,
    Backtrack,
    TerminalState,
}

/// Search trace accumulator
#[derive(Debug, Default)]
pub struct SearchTrace {
    pub steps: Vec<SearchStep>,
}

impl SearchTrace {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn add_step(&mut self, step: SearchStep) {
        self.steps.push(step);
    }

    pub fn to_string(&self) -> String {
        let mut output = String::new();

        writeln!(output, "=== Search Trace ===").unwrap();
        writeln!(output).unwrap();

        for step in &self.steps {
            writeln!(output, "Step {}:", step.step_number).unwrap();
            writeln!(output, "  VM State: {}", step.vm_state_id).unwrap();
            writeln!(output, "  Action: {:?}", step.action).unwrap();
            writeln!(output, "  Monitors: {}", step.monitors_summary).unwrap();
            writeln!(output).unwrap();
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_formula_basic() {
        let formula = CompiledLtlExpression::Always(Box::new(CompiledLtlExpression::Boolean(true)));
        assert_eq!(format_formula(&formula), "□ (true)");
    }

    #[test]
    fn test_format_formula_until() {
        let formula = CompiledLtlExpression::Until(
            Box::new(CompiledLtlExpression::Boolean(true)),
            Box::new(CompiledLtlExpression::Boolean(false)),
        );
        assert_eq!(format_formula(&formula), "(true) U (false)");
    }

    #[test]
    fn test_format_formula_complex() {
        // □ (a → ◇ b)
        let formula = CompiledLtlExpression::Always(Box::new(CompiledLtlExpression::Implies(
            Box::new(CompiledLtlExpression::Boolean(true)),
            Box::new(CompiledLtlExpression::Eventually(Box::new(
                CompiledLtlExpression::Boolean(false),
            ))),
        )));
        assert_eq!(format_formula(&formula), "□ ((true) → (◇ (false)))");
    }

    #[test]
    fn test_extract_predicates() {
        // Test with Boolean directly - predicates are embedded in complex formulas
        let formula = CompiledLtlExpression::Always(Box::new(CompiledLtlExpression::And(
            Box::new(CompiledLtlExpression::Boolean(true)),
            Box::new(CompiledLtlExpression::Boolean(false)),
        )));

        // No Predicate variants, so should return empty
        let predicates = extract_predicates(&formula);
        assert_eq!(predicates.len(), 0);
    }

    #[test]
    fn test_automaton_to_dot() {
        let formula =
            CompiledLtlExpression::Eventually(Box::new(CompiledLtlExpression::Boolean(true)));
        let automaton = BuchiAutomaton::new(formula);

        let dot = automaton_to_dot(&automaton, 0);

        // Basic structure checks
        assert!(dot.contains("digraph BuchiAutomaton0"));
        assert!(dot.contains("rankdir=LR"));
    }
}
