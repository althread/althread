use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::{
    ast::token::literal::Literal,
    checker::ltl::{
        automaton::{BuchiAutomaton}, compiled::CompiledLtlExpression,
        evaluator::evaluate_ltl_predicate,
    },
    error::{AlthreadError, AlthreadResult, ErrorType},
    vm::VM,
};

/// A monitor instance for a single Büchi automaton.
/// Each monitor tracks the current state in the automaton and maintains
/// variable bindings (e.g., for a "for p in $.procs" formula, the binding maps "p" to a specific process).
#[derive(Debug, Clone, PartialEq)]
pub struct LtlMonitor {
    /// The current state ID in the Büchi automaton
    pub current_state_id: usize,

    /// Variable bindings for this monitor instance.
    /// For example, if the formula is "check { for p in $.procs: [] (p.x > 0) }",
    /// then this map will contain {"p": ProcessLiteral(42)} for one monitor instance.
    pub bindings: HashMap<String, Literal>,
}

impl Eq for LtlMonitor {}

impl Hash for LtlMonitor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.current_state_id.hash(state);
        // Hash the bindings in a deterministic order
        let mut keys: Vec<_> = self.bindings.keys().collect();
        keys.sort();
        for key in keys {
            key.hash(state);
            // Hash a simplified representation of the value
            // We use Debug format as a proxy since Literal may not implement Hash
            format!("{:?}", self.bindings.get(key)).hash(state);
        }
    }
}

impl LtlMonitor {
    /// Creates a new monitor starting at one of the automaton's initial states.
    pub fn new(initial_state_id: usize, bindings: HashMap<String, Literal>) -> Self {
        Self {
            current_state_id: initial_state_id,
            bindings,
        }
    }

    /// Determines possible successor states for this monitor based on the current VM state.
    /// Returns a list of next monitor states (representing non-deterministic choices).
    /// If the list contains `None`, it means the monitor can stop tracking this path.
    pub fn get_possible_successors(
        &self,
        vm: &VM,
        automaton: &BuchiAutomaton,
    ) -> AlthreadResult<Vec<Option<LtlMonitor>>> {
        let current_state = automaton
            .states
            .iter()
            .find(|s| s.id == self.current_state_id)
            .ok_or_else(|| {
                AlthreadError::new(
                    ErrorType::ExpressionError,
                    None,
                    format!(
                        "Monitor state {} not found in automaton",
                        self.current_state_id
                    ),
                )
            })?;

        // Check all outgoing transitions
        let mut successors = Vec::new();

        for &target_state_id in &current_state.transitions {
            let target_state = &automaton.states[target_state_id];

            // Check if this transition is enabled by evaluating the state's formulas
            if self.check_transition_enabled(target_state, vm)? {
                let mut next_monitor = self.clone();
                next_monitor.current_state_id = target_state_id;
                successors.push(Some(next_monitor));
            }
        }

        // If no transition is enabled, the automaton cannot make progress.
        // For a negated formula (counter-example search), this means the current execution
        // does not satisfy the counter-example pattern so far.
        // We stop monitoring this specific instance (it effectively dies).
        if successors.is_empty() {
            successors.push(None);
        }

        Ok(successors)
    }

    /// Checks if a transition to a target state is enabled.
    /// A transition is enabled if all formulas (literals) in the target state are satisfied.
    fn check_transition_enabled(
        &self,
        target_state: &crate::checker::ltl::automaton::AutomatonState,
        vm: &VM,
    ) -> AlthreadResult<bool> {
        state_formulas_satisfied(&target_state.formulas, vm, &self.bindings)
    }

    /// Checks if the monitor is currently in an accepting state for a given acceptance set.
    pub fn is_in_accepting_state(&self, automaton: &BuchiAutomaton, set_index: usize) -> bool {
        if let Some(state) = automaton
            .states
            .iter()
            .find(|s| s.id == self.current_state_id)
        {
            state.is_accepting(set_index)
        } else {
            false
        }
    }

    /// Returns true if the monitor is in any accepting state.
    pub fn is_accepting(&self, automaton: &BuchiAutomaton) -> bool {
        if let Some(state) = automaton
            .states
            .iter()
            .find(|s| s.id == self.current_state_id)
        {
            if automaton.num_acceptance_sets == 0 {
                true
            } else {
                !state.acceptance_sets.is_empty()
            }
        } else {
            false
        }
    }
}

/// Represents the complete monitoring state for all LTL formulas.
/// This includes all monitor instances across all formulas and all process bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitoringState {
    /// For each LTL formula (by index), a list of monitor instances.
    /// Multiple instances exist when formulas use "for p in list" - one monitor per element.
    pub monitors_per_formula: Vec<Vec<LtlMonitor>>,
}

impl Hash for MonitoringState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for monitors in &self.monitors_per_formula {
            monitors.len().hash(state);
            for monitor in monitors {
                monitor.hash(state);
            }
        }
    }
}

impl MonitoringState {
    /// Creates a new monitoring state with no active monitors.
    pub fn new(num_formulas: usize) -> Self {
        Self {
            monitors_per_formula: vec![Vec::new(); num_formulas],
        }
    }

    /// Computes all possible next monitoring states given the current VM state.
    /// This handles non-determinism by computing the Cartesian product of all monitor choices.
    pub fn get_possible_successors(
        &self,
        vm: &VM,
        automatons: &[BuchiAutomaton],
    ) -> AlthreadResult<Vec<MonitoringState>> {
        // Collect choices for every single monitor
        let mut flattened_choices: Vec<Vec<Option<LtlMonitor>>> = Vec::new();
        let mut formula_monitor_counts = Vec::new();

        for (formula_idx, monitors) in self.monitors_per_formula.iter().enumerate() {
            let automaton = &automatons[formula_idx];
            formula_monitor_counts.push(monitors.len());

            for monitor in monitors {
                let successors = monitor.get_possible_successors(vm, automaton)?;
                flattened_choices.push(successors);
            }
        }

        let product = cartesian_product(&flattened_choices);

        let mut result_states = Vec::new();

        for combination in product {
            let mut new_monitors_per_formula = Vec::with_capacity(self.monitors_per_formula.len());
            let mut combo_iter = combination.into_iter();

            for &count in &formula_monitor_counts {
                let mut f_monitors = Vec::new();
                for _ in 0..count {
                    if let Some(m) = combo_iter.next().unwrap() {
                        f_monitors.push(m);
                    }
                }
                new_monitors_per_formula.push(f_monitors);
            }

            result_states.push(MonitoringState {
                monitors_per_formula: new_monitors_per_formula,
            });
        }

        Ok(result_states)
    }

    /// Adds monitors for initial automaton states that are enabled under the given VM.
    pub fn add_monitors_for_enabled_initial_states(
        &mut self,
        formula_index: usize,
        automaton: &BuchiAutomaton,
        bindings: HashMap<String, Literal>,
        vm: &VM,
    ) -> AlthreadResult<()> {
        if formula_index >= self.monitors_per_formula.len() {
            return Err(AlthreadError::new(
                ErrorType::ExpressionError,
                None,
                format!("Formula index {} out of bounds", formula_index),
            ));
        }

        if automaton.initial_states.is_empty() {
            return Err(AlthreadError::new(
                ErrorType::ExpressionError,
                None,
                "Automaton has no initial states".to_string(),
            ));
        }

        let mut added = 0;
        for &initial_state_id in &automaton.initial_states {
            let state = &automaton.states[initial_state_id];
            if state_formulas_satisfied(&state.formulas, vm, &bindings)? {
                let monitor = LtlMonitor::new(initial_state_id, bindings.clone());
                self.monitors_per_formula[formula_index].push(monitor);
                added += 1;
            }
        }

        if added == 0 {
            log::debug!(
                "No initial Büchi states were enabled for formula {} with the provided bindings",
                formula_index
            );
        }

        Ok(())
    }

    /// Creates a new monitor instance for a specific formula.
    /// This is used when a new process is spawned that matches a "for p in $.procs" pattern.
    pub fn add_monitor(
        &mut self,
        formula_index: usize,
        automaton: &BuchiAutomaton,
        bindings: HashMap<String, Literal>,
    ) -> AlthreadResult<()> {
        if formula_index >= self.monitors_per_formula.len() {
            return Err(AlthreadError::new(
                ErrorType::ExpressionError,
                None,
                format!("Formula index {} out of bounds", formula_index),
            ));
        }

        if automaton.initial_states.is_empty() {
            return Err(AlthreadError::new(
                ErrorType::ExpressionError,
                None,
                "Automaton has no initial states".to_string(),
            ));
        }

        for &initial_state_id in &automaton.initial_states {
            let monitor = LtlMonitor::new(initial_state_id, bindings.clone());
            self.monitors_per_formula[formula_index].push(monitor);
        }

        Ok(())
    }
}

fn cartesian_product<T: Clone>(lists: &[Vec<T>]) -> Vec<Vec<T>> {
    let mut res = vec![vec![]];
    for list in lists {
        let mut new_res = Vec::with_capacity(res.len() * list.len());
        for combo in res {
            for item in list {
                let mut new_combo = combo.clone();
                new_combo.push(item.clone());
                new_res.push(new_combo);
            }
        }
        res = new_res;
    }
    res
}

fn state_formulas_satisfied(
    formulas: &[CompiledLtlExpression],
    vm: &VM,
    bindings: &HashMap<String, Literal>,
) -> AlthreadResult<bool> {
    for formula in formulas {
        if matches!(formula, CompiledLtlExpression::Next(_)) {
            continue;
        }

        let is_satisfied = match formula {
            CompiledLtlExpression::Predicate { .. }
            | CompiledLtlExpression::Boolean(_)
            | CompiledLtlExpression::ForLoop { .. }
            | CompiledLtlExpression::Exists { .. } => {
                evaluate_ltl_predicate(formula, vm, bindings)?
            }
            CompiledLtlExpression::Not(inner) => match inner.as_ref() {
                CompiledLtlExpression::Predicate { .. }
                | CompiledLtlExpression::Boolean(_)
                | CompiledLtlExpression::ForLoop { .. }
                | CompiledLtlExpression::Exists { .. } => {
                    !evaluate_ltl_predicate(inner, vm, bindings)?
                }
                _ => continue,
            },
            CompiledLtlExpression::Until(_, _) | CompiledLtlExpression::Release(_, _) => {
                continue;
            }
            _ => continue,
        };

        if !is_satisfied {
            return Ok(false);
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_creation() {
        let mut bindings = HashMap::new();
        bindings.insert("p".to_string(), Literal::Int(42));

        let monitor = LtlMonitor::new(0, bindings.clone());

        assert_eq!(monitor.current_state_id, 0);
        assert_eq!(monitor.bindings.get("p"), Some(&Literal::Int(42)));
    }
}
