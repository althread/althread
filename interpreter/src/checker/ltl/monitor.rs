//! LTL monitoring during state-space exploration.
//!
//! This module provides runtime tracking of Büchi automaton states during program execution.
//! Each monitor instance tracks its position in the automaton and any variable bindings
//! (for quantified formulas like `for p in $.procs { always P(p) }`).
//!
//! # Key Concepts
//!
//! - **LtlMonitor**: Tracks a single automaton state with variable bindings
//! - **MonitoringState**: Manages all monitors for all LTL formulas
//! - Monitors advance based on predicate evaluations at each VM state
//! - Multiple successor states represent non-determinism in the automaton

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::{
    ast::token::literal::Literal,
    checker::{
        fxhash::FxHashMap,
        ltl::{
            automaton::BuchiAutomaton, compiled::CompiledLtlExpression,
            evaluator::evaluate_ltl_predicate,
        },
    },
    error::{AlthreadError, AlthreadResult, ErrorType},
    vm::VM,
};

/// Caches `state_formulas_satisfied` per `(vm_id, automaton_state_id, bindings)`
/// during one product-graph exploration.
pub(crate) struct PredicateCache {
    num_vm_states: usize,
    num_aut_states: usize,
    /// Flat cache for empty-binding formulas: 0 = unknown, 1 = false, 2 = true.
    unquantified: Vec<u8>,
    /// Flat caches for up to 32 distinct single-binding values (zero-clone lookup on hit).
    single_binding: FxHashMap<Literal, Vec<u8>>,
    /// Fallback cache when there are many bindings or multi-variable bindings.
    sparse: FxHashMap<(usize, usize, Literal), bool>,
}

impl PredicateCache {
    pub(crate) fn new(num_vm_states: usize, num_aut_states: usize) -> Self {
        Self {
            num_vm_states,
            num_aut_states,
            unquantified: Vec::new(),
            single_binding: FxHashMap::default(),
            sparse: FxHashMap::default(),
        }
    }

    pub(crate) fn check(
        &mut self,
        vm_id: usize,
        vm: &VM,
        automaton: &BuchiAutomaton,
        target_state_id: usize,
        bindings: &HashMap<String, Literal>,
    ) -> AlthreadResult<bool> {
        let flat_len = self.num_vm_states * self.num_aut_states;
        let slot = vm_id * self.num_aut_states + target_state_id;

        if bindings.is_empty() && slot < flat_len {
            if self.unquantified.is_empty() {
                self.unquantified.resize(flat_len, 0);
            }
            let cached = self.unquantified[slot];
            if cached != 0 {
                return Ok(cached == 2);
            }
            let satisfied = state_formulas_satisfied(
                &automaton.states[target_state_id].formulas,
                vm,
                bindings,
            )?;
            self.unquantified[slot] = if satisfied { 2 } else { 1 };
            return Ok(satisfied);
        }

        if bindings.len() == 1 && slot < flat_len {
            let value = bindings.values().next().unwrap();
            if let Some(table) = self.single_binding.get_mut(value) {
                let cached = table[slot];
                if cached != 0 {
                    return Ok(cached == 2);
                }
                let satisfied = state_formulas_satisfied(
                    &automaton.states[target_state_id].formulas,
                    vm,
                    bindings,
                )?;
                table[slot] = if satisfied { 2 } else { 1 };
                return Ok(satisfied);
            }
            if flat_len <= 500_000 && self.single_binding.len() < 32 {
                let satisfied = state_formulas_satisfied(
                    &automaton.states[target_state_id].formulas,
                    vm,
                    bindings,
                )?;
                let mut table = vec![0u8; flat_len];
                table[slot] = if satisfied { 2 } else { 1 };
                self.single_binding.insert(value.clone(), table);
                return Ok(satisfied);
            }
            let key = (vm_id, target_state_id, value.clone());
            if let Some(&cached) = self.sparse.get(&key) {
                return Ok(cached);
            }
            let satisfied = state_formulas_satisfied(
                &automaton.states[target_state_id].formulas,
                vm,
                bindings,
            )?;
            self.sparse.insert(key, satisfied);
            return Ok(satisfied);
        }

        state_formulas_satisfied(&automaton.states[target_state_id].formulas, vm, bindings)
    }
}

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
        match self.bindings.len() {
            0 => {}
            1 => {
                let (key, value) = self.bindings.iter().next().unwrap();
                key.hash(state);
                value.hash(state);
            }
            _ => {
                // Hash the bindings in a deterministic order
                let mut keys: Vec<_> = self.bindings.keys().collect();
                keys.sort_unstable();
                for key in keys {
                    key.hash(state);
                    self.bindings[key].hash(state);
                }
            }
        }
    }
}

impl LtlMonitor {
    /// A rejecting sink retains the quantified binding after its counterexample
    /// run dies. Dropping it would turn a satisfied existential into a violation.
    pub const REJECTED: usize = usize::MAX;

    /// Creates a new monitor starting at one of the automaton's initial states.
    pub fn new(initial_state_id: usize, bindings: HashMap<String, Literal>) -> Self {
        Self {
            current_state_id: initial_state_id,
            bindings,
        }
    }

    /// Determines possible successor states for this monitor based on the current VM state.
    /// Returns a list of next monitor states (representing non-deterministic choices).
    /// Rejected runs enter a rejecting sink, preserving their quantified binding.
    pub fn get_possible_successors(
        &self,
        vm: &VM,
        automaton: &BuchiAutomaton,
    ) -> AlthreadResult<Vec<Option<LtlMonitor>>> {
        if self.current_state_id == Self::REJECTED {
            return Ok(vec![Some(self.clone())]);
        }
        let current_state = automaton.states.get(self.current_state_id).ok_or_else(|| {
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

        // Preserve the binding even when this counterexample run is rejected.
        if successors.is_empty() {
            successors.push(Some(Self::new(Self::REJECTED, self.bindings.clone())));
        }

        Ok(successors)
    }

    pub(crate) fn get_possible_successors_cached(
        &self,
        vm_id: usize,
        vm: &VM,
        automaton: &BuchiAutomaton,
        cache: &mut PredicateCache,
    ) -> AlthreadResult<Vec<LtlMonitor>> {
        if self.current_state_id == Self::REJECTED {
            return Ok(vec![self.clone()]);
        }
        let current_state = automaton.states.get(self.current_state_id).ok_or_else(|| {
            AlthreadError::new(
                ErrorType::ExpressionError,
                None,
                format!(
                    "Monitor state {} not found in automaton",
                    self.current_state_id
                ),
            )
        })?;

        let mut successors = Vec::new();
        for &target_state_id in &current_state.transitions {
            if cache.check(vm_id, vm, automaton, target_state_id, &self.bindings)? {
                let mut next_monitor = self.clone();
                next_monitor.current_state_id = target_state_id;
                successors.push(next_monitor);
            }
        }

        if successors.is_empty() {
            successors.push(Self::new(Self::REJECTED, self.bindings.clone()));
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
        if let Some(state) = automaton.states.get(self.current_state_id) {
            state.is_accepting(set_index)
        } else {
            false
        }
    }

    /// Whether this state alone covers every acceptance set. A cycle can also
    /// accept by covering different sets at different states; the SCC checker
    /// performs that test separately.
    pub fn is_accepting(&self, automaton: &BuchiAutomaton) -> bool {
        if let Some(state) = automaton.states.get(self.current_state_id) {
            if automaton.num_acceptance_sets == 0 {
                true
            } else {
                (0..automaton.num_acceptance_sets).all(|set| state.is_accepting(set))
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

    /// Select one initial automaton alternative per binding. Initial states
    /// of the same instance are nondeterministic alternatives, not independent
    /// quantified obligations (especially important for existential formulas).
    #[must_use]
    pub fn initial_choices(&self) -> Vec<Self> {
        let mut choices = vec![Self::new(self.monitors_per_formula.len())];
        for (formula, monitors) in self.monitors_per_formula.iter().enumerate() {
            let mut groups: Vec<Vec<&LtlMonitor>> = Vec::new();
            for monitor in monitors {
                if let Some(group) = groups
                    .iter_mut()
                    .find(|g| g[0].bindings == monitor.bindings)
                {
                    group.push(monitor);
                } else {
                    groups.push(vec![monitor]);
                }
            }
            for group in groups {
                let mut next = Vec::new();
                for choice in &choices {
                    for monitor in &group {
                        let mut choice = choice.clone();
                        choice.monitors_per_formula[formula].push((*monitor).clone());
                        next.push(choice);
                    }
                }
                choices = next;
            }
        }
        choices
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

    /// Optimized single-formula successor step using a shared `PredicateCache`.
    pub(crate) fn get_possible_successors_single_cached(
        &self,
        vm_id: usize,
        vm: &VM,
        automaton: &BuchiAutomaton,
        cache: &mut PredicateCache,
    ) -> AlthreadResult<Vec<MonitoringState>> {
        let monitors = &self.monitors_per_formula[0];
        match monitors.len() {
            0 => Ok(vec![MonitoringState {
                monitors_per_formula: vec![Vec::new()],
            }]),
            1 => {
                let next = monitors[0].get_possible_successors_cached(vm_id, vm, automaton, cache)?;
                Ok(next
                    .into_iter()
                    .map(|m| MonitoringState {
                        monitors_per_formula: vec![vec![m]],
                    })
                    .collect())
            }
            _ => {
                let mut choices = Vec::with_capacity(monitors.len());
                for monitor in monitors {
                    choices.push(monitor.get_possible_successors_cached(vm_id, vm, automaton, cache)?);
                }
                let product = cartesian_product(&choices);
                Ok(product
                    .into_iter()
                    .map(|f_monitors| MonitoringState {
                        monitors_per_formula: vec![f_monitors],
                    })
                    .collect())
            }
        }
    }

    pub(crate) fn add_monitors_for_enabled_initial_states_cached(
        &mut self,
        formula_index: usize,
        vm_id: usize,
        automaton: &BuchiAutomaton,
        bindings: HashMap<String, Literal>,
        vm: &VM,
        cache: &mut PredicateCache,
    ) -> AlthreadResult<bool> {
        if formula_index >= self.monitors_per_formula.len() {
            return Err(AlthreadError::new(
                ErrorType::ExpressionError,
                None,
                format!("Formula index {} out of bounds", formula_index),
            ));
        }

        if self.monitors_per_formula[formula_index]
            .iter()
            .any(|monitor| monitor.bindings == bindings)
        {
            return Ok(false);
        }

        let mut added = 0;
        for &initial_state_id in &automaton.initial_states {
            if cache.check(vm_id, vm, automaton, initial_state_id, &bindings)? {
                let monitor = LtlMonitor::new(initial_state_id, bindings.clone());
                self.monitors_per_formula[formula_index].push(monitor);
                added += 1;
            }
        }

        if added == 0 {
            self.monitors_per_formula[formula_index]
                .push(LtlMonitor::new(LtlMonitor::REJECTED, bindings));
        }

        Ok(true)
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

        // Repeated list values (or a value re-entering a dynamic domain) do
        // not introduce a second instance of the same quantified obligation.
        if self.monitors_per_formula[formula_index]
            .iter()
            .any(|monitor| monitor.bindings == bindings)
        {
            return Ok(());
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
            self.monitors_per_formula[formula_index]
                .push(LtlMonitor::new(LtlMonitor::REJECTED, bindings));
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
            self.monitors_per_formula[formula_index]
                .push(LtlMonitor::new(LtlMonitor::REJECTED, bindings));
            return Ok(());
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
