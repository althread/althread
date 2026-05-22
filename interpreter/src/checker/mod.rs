//! Model checking module for Althread programs.
//!
//! This module provides state-space exploration and verification capabilities:
//! - Basic invariant checking via `check_program`
//! - LTL model checking via `check_program_with_ltl` using Büchi automatons
//!
//! # LTL Verification Algorithm
//!
//! The LTL checker uses the automata-theoretic approach:
//! 1. Negate the LTL formula (to find counter-examples)
//! 2. Build a Büchi automaton from the negated formula
//! 3. Explore the product automaton (program × Büchi automaton)
//! 4. Use Nested DFS to detect accepting cycles
//! 5. An accepting cycle means the negated formula is satisfiable → original violated

pub mod ltl;

#[cfg(test)]
mod ltl_integration_tests;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::Hash,
    rc::Rc,
};

use ltl::{automaton::BuchiAutomaton, compiled::CompiledLtlExpression, monitor::MonitoringState};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::{
    compiler::CompiledProject,
    error::{AlthreadError, AlthreadResult, ErrorType},
    vm::{instruction::Instruction, GlobalAction, VM},
};

pub type StateId = usize;

#[derive(Debug, Clone)]
pub struct StateLink {
    pub instructions: Vec<Instruction>,
    pub actions: Vec<GlobalAction>,
    pub lines: Vec<usize>,
    pub pid: usize,
    pub name: String,
    pub to: StateId,
}

#[derive(Debug)]
pub struct GraphNode {
    pub level: usize,
    pub predecessor: Option<StateId>,
    pub successors: Vec<StateLink>,
    pub eventually: bool,
    pub expanded: bool,
}

#[derive(Debug)]
pub struct StateGraph<'a> {
    pub states: Vec<Rc<VM<'a>>>,
    pub nodes: Vec<GraphNode>,
    pub initial_state: StateId,
    pub exhaustive: bool,
}

impl std::fmt::Display for StateLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StateLink {{ lines: {:?}, pid: {}, name: {}, to: {} }}",
            self.lines, self.pid, self.name, self.to
        )
    }
}

impl Serialize for StateLink {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("StateLink", 5)?;
        state.serialize_field("lines", &self.lines)?;
        state.serialize_field("pid", &self.pid)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("to", &self.to)?;
        state.serialize_field("actions", &self.actions)?;
        state.end()
    }
}
impl Serialize for GraphNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("GraphNode", 5)?;
        state.serialize_field("level", &self.level)?;
        state.serialize_field("predecessor", &self.predecessor)?;
        state.serialize_field("successors", &self.successors)?;
        state.serialize_field("eventually", &self.eventually)?;
        state.serialize_field("expanded", &self.expanded)?;
        state.end()
    }
}
impl<'a> Serialize for StateGraph<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("StateGraph", 2)?;
        state.serialize_field(
            "nodes",
            &self
                .states
                .iter()
                .zip(self.nodes.iter())
                .map(|(vm, node)| (vm.as_ref(), node))
                .collect::<Vec<(&VM, &GraphNode)>>(),
        )?;
        state.serialize_field("exhaustive", &self.exhaustive)?;
        state.end()
    }
}

impl GraphNode {
    pub fn new(predecessor: Option<StateId>, level: usize) -> Self {
        Self {
            level,
            predecessor,
            eventually: false,
            successors: Vec::new(),
            expanded: false,
        }
    }
}

impl<'a> StateGraph<'a> {
    pub fn new(initial_vm: Rc<VM<'a>>) -> Self {
        Self {
            states: vec![initial_vm],
            nodes: vec![GraphNode::new(None, 0)],
            initial_state: 0,
            exhaustive: true,
        }
    }

    pub fn push_state(
        &mut self,
        vm: Rc<VM<'a>>,
        predecessor: Option<StateId>,
        level: usize,
    ) -> StateId {
        let id = self.states.len();
        self.states.push(vm);
        self.nodes.push(GraphNode::new(predecessor, level));
        id
    }

    pub fn vm(&self, state_id: StateId) -> &Rc<VM<'a>> {
        &self.states[state_id]
    }
}

fn collect_instruction_lines(instructions: &[Instruction]) -> Vec<usize> {
    let mut lines: Vec<usize> = instructions
        .iter()
        .map(|instruction| instruction.pos.clone().unwrap_or_default().line())
        .filter(|line| *line > 0)
        .collect();
    lines.sort();
    lines.dedup();
    lines
}

fn build_state_graph<'a>(
    compiled_project: &'a CompiledProject,
    max_states: Option<usize>,
) -> AlthreadResult<StateGraph<'a>> {
    let mut init_vm = VM::new(compiled_project);
    init_vm.start(0);

    let initial_vm = Rc::new(init_vm);
    let mut state_graph = StateGraph::new(initial_vm.clone());
    let mut known_states = HashMap::new();
    known_states.insert(initial_vm, state_graph.initial_state);

    let mut next_nodes = VecDeque::new();
    next_nodes.push_back(state_graph.initial_state);

    while let Some(current_state) = next_nodes.pop_front() {
        if let Some(max) = max_states {
            if state_graph.nodes.len() >= max {
                state_graph.exhaustive = false;
                break;
            }
        }

        let current_vm = state_graph.vm(current_state).clone();
        let current_level = state_graph.nodes[current_state].level;
        let successors = current_vm.next()?;

        for (name, pid, instructions, actions, vm) in successors.into_iter() {
            let next_vm = Rc::new(vm);
            let lines = collect_instruction_lines(&instructions);
            let next_state = if let Some(existing_state) = known_states.get(&next_vm) {
                *existing_state
            } else {
                let new_state =
                    state_graph.push_state(next_vm.clone(), Some(current_state), current_level + 1);
                known_states.insert(next_vm.clone(), new_state);
                next_nodes.push_back(new_state);
                new_state
            };

            state_graph.nodes[current_state].successors.push(StateLink {
                to: next_state,
                lines,
                instructions,
                actions,
                pid,
                name,
            });
        }

        state_graph.nodes[current_state].expanded = true;
    }

    Ok(state_graph)
}
/// Checks a given project, returning a path from an initial state to the first state that violates an invariant. (return an empty vector if no invariant is violated)
pub fn check_program<'a>(
    compiled_project: &'a CompiledProject,
    max_states: Option<usize>,
) -> AlthreadResult<(Vec<StateLink>, StateGraph<'a>)> {
    if !compiled_project.compiled_ltl_formulas.is_empty() {
        println!(
            "Found {} compiled LTL formulas in the project",
            compiled_project.compiled_ltl_formulas.len()
        );
        for (i, formula) in compiled_project.compiled_ltl_formulas.iter().enumerate() {
            println!("Compiled LTL Formula #{}: {}", i + 1, formula);
        }
        println!("Starting LTL verification...");
        return check_program_with_ltl(compiled_project, max_states);
    }

    let mut state_graph = build_state_graph(compiled_project, max_states)?;

    for current_state in 0..state_graph.nodes.len() {
        let check_ret = state_graph.vm(current_state).check_invariants();
        if let Err(e) = check_ret {
            let mut path = Vec::new();
            let mut back_node = current_state;

            if state_graph.nodes[back_node].predecessor.is_none() {
                let lines = if let Some(pos) = &e.pos {
                    vec![pos.line()]
                } else {
                    vec![]
                };
                path.push(StateLink {
                    to: back_node,
                    lines,
                    instructions: vec![],
                    actions: vec![],
                    pid: 0,
                    name: "_init_".to_string(),
                });
                return Ok((path, state_graph));
            }

            while let Some(pred) = state_graph.nodes[back_node].predecessor {
                path.push(
                    state_graph
                        .nodes
                        .get(pred)
                        .unwrap()
                        .successors
                        .iter()
                        .find(|x| x.to == back_node)
                        .unwrap()
                        .clone(),
                );
                back_node = pred;
            }

            return Ok((path.into_iter().rev().collect(), state_graph));
        } else if check_ret.is_ok_and(|x| x == 1) {
            state_graph.nodes[current_state].eventually = true;
        }
    }

    // If the search was not exhaustive, we cannot check eventually violations
    if !state_graph.exhaustive {
        return Ok((vec![], state_graph));
    }

    // Now check for eventually violations using path exploration

    // path visit is used to keep track of the successors we've already checked
    let mut path_visit: Vec<usize> = Vec::new();
    let mut path = Vec::new();
    let mut path_set = std::collections::HashSet::new();
    // if root node check eventually condition no path can exist
    if state_graph.nodes[state_graph.initial_state].eventually {
        return Ok((vec![], state_graph));
    }

    path.push(state_graph.initial_state);
    path_set.insert(state_graph.initial_state);
    // no successors have yet been visited
    path_visit.push(0);

    while !path.is_empty() {
        let curr_state = {
            let temp = path.last().unwrap();
            *temp
        };

        let mut visited_succ = path_visit.pop().unwrap();

        // get all the successors of the current node
        let mut succ = Vec::new();
        for link in state_graph.nodes[curr_state]
            .successors
            .iter()
            .skip(visited_succ)
        {
            succ.push(link.clone());
        }

        // if the current node have no successors then we found an invalid path of execution
        if succ.is_empty() && visited_succ == 0 {
            let ret = reconstruct_path(path, &state_graph);

            match ret {
                Ok(vec) => {
                    return Ok((vec.into_iter().rev().collect(), state_graph));
                }
                Err(e) => {
                    return Err(AlthreadError::new(
                        ErrorType::ExpressionError,
                        None,
                        e.message,
                    ))
                }
            }
        }

        // we search an explorable path in the successors list
        let mut explorable_path = false;
        while !succ.is_empty() && !explorable_path {
            let curr_succ = succ.pop().unwrap();
            visited_succ += 1;

            // if the successor is already in the path we found an invalid execution path
            if path_set.contains(&curr_succ.to) {
                // If it is in the path, we push it temporarily just to have it for reconstruction,
                // OR we can reconstruct including the cycle closing edge.
                path.push(curr_succ.to.clone());
                let ret = reconstruct_path(path, &state_graph);
                match ret {
                    Ok(vec) => return Ok((vec.into_iter().rev().collect(), state_graph)),
                    // safety purpose
                    Err(e) => {
                        return Err(AlthreadError::new(
                            ErrorType::ExpressionError,
                            None,
                            e.message,
                        ))
                    }
                }
            }

            // we get the corresponding graphnode and check wheter he has the eventually flag or not
            let graph_node = &state_graph.nodes[curr_succ.to];
            if !graph_node.eventually {
                explorable_path = true;
                path.push(curr_succ.to);
                path_set.insert(curr_succ.to);
                // we update the number of visited successors of the current node
                path_visit.push(visited_succ);
                // we then init the number of visited successors from the new node in the path
                path_visit.push(0);
            }
        }
        // if no explorable path was found we condemn this node (it is a dead end)
        if !explorable_path {
            state_graph.nodes[curr_state].eventually = true;
            let popped = path.pop();
            if let Some(p) = popped {
                path_set.remove(&p);
            }
        }
    }
    Ok((vec![], state_graph))
}

pub fn reconstruct_path<'a>(
    mut vec_vm: Vec<StateId>,
    state_graph: &StateGraph<'a>,
) -> AlthreadResult<Vec<StateLink>> {
    let mut ret_path = Vec::new();
    let mut back_node = vec_vm.pop().unwrap();

    while let Some(pred) = vec_vm.pop() {
        ret_path.push(
            state_graph
                .nodes
                .get(pred)
                .unwrap()
                .successors
                .iter()
                .find(|x| x.to == back_node)
                .unwrap()
                .clone(),
        );

        back_node = pred;
    }
    Ok(ret_path)
}

/// Combined state for product automaton (VM state + monitor states)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CombinedProductState {
    vm: StateId,
    monitors: MonitoringState,
}

/// Checks a program with LTL formulas by exploring the reachable product automaton
/// and then computing strongly connected components to detect accepting cycles.
///
/// This keeps the product exploration linear in the number of reachable states and
/// edges, which is much more stable than relaunching an inner DFS from every
/// accepting state when several formulas introduce additional nondeterminism.
fn check_program_with_ltl<'a>(
    compiled_project: &'a CompiledProject,
    max_states: Option<usize>,
) -> AlthreadResult<(Vec<StateLink>, StateGraph<'a>)> {
    // Step 1: Build Büchi automatons from compiled LTL formulas
    let automatons: Vec<BuchiAutomaton> = compiled_project
        .compiled_ltl_formulas
        .iter()
        .map(|formula| match formula {
            CompiledLtlExpression::ForLoop { body, .. }
            | CompiledLtlExpression::Exists { body, .. } => {
                BuchiAutomaton::new(body.as_ref().clone())
            }
            _ => BuchiAutomaton::new(formula.clone()),
        })
        .collect();

    for (i, aut) in automatons.iter().enumerate() {
        log::debug!("Automaton #{}:", i + 1);
        for state in &aut.states {
            log::debug!("  State {}: accept={:?}", state.id, state.acceptance_sets);
            log::debug!("    Formulas: {:?}", state.formulas);
            log::debug!("    Transitions: {:?}", state.transitions);
        }
    }

    println!("Built {} Büchi automatons", automatons.len());

    // Step 2: Build the VM state graph once and reuse it for all formulas.
    let state_graph = build_state_graph(compiled_project, max_states)?;
    let initial_vm = state_graph.vm(state_graph.initial_state).clone();

    // Step 3: Initialize monitoring state with proper quantifier handling
    let initial_monitoring = ltl::quantifier::initialize_monitoring(
        &compiled_project.compiled_ltl_formulas,
        &automatons,
        initial_vm.as_ref(),
    )?;

    let mut visited_outer: HashSet<CombinedProductState> = HashSet::new();

    // Store the graph edges for path reconstruction
    let mut product_edges: HashMap<CombinedProductState, Vec<CombinedProductState>> =
        HashMap::new();

    // Initial product state
    let initial_product_state = CombinedProductState {
        vm: state_graph.initial_state,
        monitors: initial_monitoring.clone(),
    };

    let mut dfs_stack: Vec<CombinedProductState> = vec![initial_product_state.clone()];

    while let Some(current_state) = dfs_stack.pop() {
        if visited_outer.contains(&current_state) {
            continue;
        }

        visited_outer.insert(current_state.clone());
        let current_vm_id = current_state.vm;

        let is_terminal_state =
            state_graph.nodes[current_vm_id].expanded && state_graph.nodes[current_vm_id].successors.is_empty();
        let is_immediate_accepting = is_terminal_state
            && monitors_in_immediate_accepting_state(
                &current_state.monitors,
                &automatons,
                &compiled_project.compiled_ltl_formulas,
            );

        if is_immediate_accepting {
            log::debug!("DEBUG: Immediate accepting state detected (no temporal obligations)");
            println!("LTL violation detected: accepting state with no temporal obligations");
            let violation_path = build_violation_path(&state_graph, current_state.vm)?;
            return Ok((violation_path, state_graph));
        }

        let current_vm = state_graph.vm(current_vm_id).clone();
        let current_monitors = &current_state.monitors;
        let successors = state_graph.nodes[current_vm_id].successors.clone();

        if successors.is_empty() && state_graph.nodes[current_vm_id].expanded {
            log::debug!(
                "DEBUG: Terminal state - is_finished={}",
                current_vm.is_finished()
            );

            let mut base_next_monitors = current_monitors.clone();
            ltl::quantifier::update_monitors_for_new_processes(
                &compiled_project.compiled_ltl_formulas,
                &automatons,
                &mut base_next_monitors,
                current_vm.as_ref(),
                current_vm.as_ref(),
            )?;

            let possible_next_monitoring_states =
                base_next_monitors.get_possible_successors(current_vm.as_ref(), &automatons)?;

            for next_monitors in possible_next_monitoring_states {
                let next_product_state = CombinedProductState {
                    vm: current_vm_id,
                    monitors: next_monitors,
                };

                push_product_edge(
                    &mut product_edges,
                    current_state.clone(),
                    next_product_state.clone(),
                );

                if !visited_outer.contains(&next_product_state) {
                    dfs_stack.push(next_product_state);
                }
            }
            continue;
        }

        if successors.is_empty() {
            log::debug!(
                "DEBUG: Frontier state reached before full expansion, skipping terminal-state reasoning"
            );
            continue;
        }

        for successor in successors.into_iter() {
            let next_state = successor.to;
            let next_vm = state_graph.vm(next_state).clone();

            let mut base_next_monitors = current_monitors.clone();
            ltl::quantifier::update_monitors_for_new_processes(
                &compiled_project.compiled_ltl_formulas,
                &automatons,
                &mut base_next_monitors,
                current_vm.as_ref(),
                next_vm.as_ref(),
            )?;

            let possible_next_monitoring_states =
                base_next_monitors.get_possible_successors(next_vm.as_ref(), &automatons)?;

            for next_monitors in possible_next_monitoring_states {
                let next_product_state = CombinedProductState {
                    vm: next_state,
                    monitors: next_monitors,
                };

                push_product_edge(
                    &mut product_edges,
                    current_state.clone(),
                    next_product_state.clone(),
                );

                if !visited_outer.contains(&next_product_state) {
                    dfs_stack.push(next_product_state);
                }
            }
        }
    }

    if let Some(accepting_state) = find_accepting_cycle_state(
        &visited_outer,
        &product_edges,
        &automatons,
        &compiled_project.compiled_ltl_formulas,
    ) {
        println!("LTL violation detected: accepting cycle found");
        let violation_path = build_violation_path(&state_graph, accepting_state.vm)?;
        return Ok((violation_path, state_graph));
    }

    // Traditional invariant checking (separate pass for safety properties)
    // This is done on the state graph we built
    for state_id in 0..state_graph.nodes.len() {
        let vm = state_graph.vm(state_id).clone();
        let check_ret = vm.check_invariants();
        if let Err(e) = check_ret {
            let violation_path = build_violation_path(&state_graph, state_id)?;
            if violation_path.is_empty() {
                // Initial state violation
                let lines = if let Some(pos) = &e.pos {
                    vec![pos.line()]
                } else {
                    vec![]
                };
                return Ok((
                    vec![StateLink {
                        to: state_id,
                        lines,
                        instructions: vec![],
                        actions: vec![],
                        pid: 0,
                        name: "_init_".to_string(),
                    }],
                    state_graph,
                ));
            }
            return Ok((violation_path, state_graph));
        }
    }

    // No violations found
    println!("LTL verification completed: no violations found");
    Ok((vec![], state_graph))
}

fn push_product_edge(
    product_edges: &mut HashMap<CombinedProductState, Vec<CombinedProductState>>,
    from: CombinedProductState,
    to: CombinedProductState,
) {
    let successors = product_edges.entry(from).or_default();
    if !successors.contains(&to) {
        successors.push(to);
    }
}

fn find_accepting_cycle_state(
    reachable_states: &HashSet<CombinedProductState>,
    product_edges: &HashMap<CombinedProductState, Vec<CombinedProductState>>,
    automatons: &[BuchiAutomaton],
    formulas: &[CompiledLtlExpression],
) -> Option<CombinedProductState> {
    let mut reverse_edges: HashMap<CombinedProductState, Vec<CombinedProductState>> =
        HashMap::new();
    for (from, successors) in product_edges {
        reverse_edges.entry(from.clone()).or_default();
        for successor in successors {
            reverse_edges
                .entry(successor.clone())
                .or_default()
                .push(from.clone());
        }
    }

    let mut visited = HashSet::new();
    let mut finish_order = Vec::new();

    for state in reachable_states {
        if visited.contains(state) {
            continue;
        }

        let mut stack = vec![(state.clone(), false)];
        while let Some((current, expanded)) = stack.pop() {
            if expanded {
                finish_order.push(current);
                continue;
            }

            if !visited.insert(current.clone()) {
                continue;
            }

            stack.push((current.clone(), true));
            if let Some(successors) = product_edges.get(&current) {
                for successor in successors {
                    if !visited.contains(successor) {
                        stack.push((successor.clone(), false));
                    }
                }
            }
        }
    }

    let mut assigned = HashSet::new();

    while let Some(state) = finish_order.pop() {
        if assigned.contains(&state) {
            continue;
        }

        let mut component = Vec::new();
        let mut stack = vec![state.clone()];
        assigned.insert(state.clone());

        while let Some(current) = stack.pop() {
            component.push(current.clone());
            if let Some(predecessors) = reverse_edges.get(&current) {
                for predecessor in predecessors {
                    if assigned.insert(predecessor.clone()) {
                        stack.push(predecessor.clone());
                    }
                }
            }
        }

        let is_cyclic = component.len() > 1
            || product_edges
                .get(&component[0])
                .map(|successors| successors.contains(&component[0]))
                .unwrap_or(false);

        if !is_cyclic {
            continue;
        }

        if let Some(accepting_state) = component.into_iter().find(|state| {
            monitors_in_accepting_state(&state.monitors, automatons, formulas)
        }) {
            return Some(accepting_state);
        }
    }

    None
}

fn build_violation_path<'a>(
    state_graph: &StateGraph<'a>,
    target: StateId,
) -> AlthreadResult<Vec<StateLink>> {
    let mut path = Vec::new();
    let mut back_node = target;

    while let Some(pred) = state_graph.nodes[back_node].predecessor {
        let link = state_graph
            .nodes
            .get(pred)
            .unwrap()
            .successors
            .iter()
            .find(|x| x.to == back_node)
            .unwrap()
            .clone();
        path.push(link);
        back_node = pred;
    }

    Ok(path.into_iter().rev().collect())
}

/// Check if any monitor is in an accepting state on a cycle (or terminal state).
/// Check if any monitor is currently in an accepting state.
/// Used by the Nested DFS algorithm to identify accepting states.
fn monitors_in_accepting_state(
    monitors: &MonitoringState,
    automatons: &[BuchiAutomaton],
    formulas: &[CompiledLtlExpression],
) -> bool {
    monitors
        .monitors_per_formula
        .iter()
        .enumerate()
        .any(|(formula_idx, monitors)| {
            let automaton = &automatons[formula_idx];

            // For Büchi automatons (with acceptance sets), we check if any monitor
            // is in an accepting state. The cycle will ensure we visit it infinitely often.
            // For degenerate automatons (without acceptance sets), all states are accepting.

            match &formulas[formula_idx] {
                CompiledLtlExpression::Exists { .. } => {
                    // Exists: violation only if all monitors accept (or no monitor at all)
                    if monitors.is_empty() {
                        return true;
                    }
                    monitors
                        .iter()
                        .all(|monitor| monitor.is_accepting(automaton))
                }
                _ => monitors
                    .iter()
                    .any(|monitor| monitor.is_accepting(automaton)),
            }
        })
}

/// Check if any monitor is in an accepting state with no temporal obligations.
///
/// This is an optimization: when a Büchi state has no temporal obligations (no Next formulas),
/// it means any infinite continuation will stay in accepting states. We can immediately
/// report a violation without needing to find the actual cycle.
///
/// This provides:
/// 1. Shorter counter-example traces (shows exactly where violation occurs)
/// 2. Faster detection (no need to explore further)
fn monitors_in_immediate_accepting_state(
    monitors: &MonitoringState,
    automatons: &[BuchiAutomaton],
    formulas: &[CompiledLtlExpression],
) -> bool {
    monitors
        .monitors_per_formula
        .iter()
        .enumerate()
        .any(|(formula_idx, monitors)| {
            let automaton = &automatons[formula_idx];

            match &formulas[formula_idx] {
                CompiledLtlExpression::Exists { .. } => {
                    if monitors.is_empty() {
                        return true;
                    }
                    monitors.iter().all(|monitor| {
                        monitor.is_accepting(automaton)
                            && state_has_only_propositional_formulas(
                                automaton,
                                monitor.current_state_id,
                            )
                    })
                }
                _ => monitors.iter().any(|monitor| {
                    monitor.is_accepting(automaton)
                        && state_has_only_propositional_formulas(
                            automaton,
                            monitor.current_state_id,
                        )
                }),
            }
        })
}

/// Check if a Büchi state has only propositional formulas
/// (no temporal obligations like Next, Until, Eventually, Always).
///
/// When a state has no temporal obligations, any infinite suffix from this state
/// will remain in accepting states, so we can detect violations immediately.
fn state_has_only_propositional_formulas(automaton: &BuchiAutomaton, state_id: usize) -> bool {
    if let Some(state) = automaton.states.get(state_id) {
        state.formulas.iter().all(|f| is_propositional(f))
    } else {
        false
    }
}

/// Check if an LTL expression is purely propositional (no temporal operators).
fn is_propositional(expr: &CompiledLtlExpression) -> bool {
    match expr {
        CompiledLtlExpression::Boolean(_) => true,
        CompiledLtlExpression::Predicate { .. } => true,
        CompiledLtlExpression::Not(inner) => is_propositional(inner),
        CompiledLtlExpression::And(a, b)
        | CompiledLtlExpression::Or(a, b)
        | CompiledLtlExpression::Implies(a, b) => is_propositional(a) && is_propositional(b),
        // Temporal operators
        CompiledLtlExpression::Next(_)
        | CompiledLtlExpression::Eventually(_)
        | CompiledLtlExpression::Always(_)
        | CompiledLtlExpression::Until(_, _)
        | CompiledLtlExpression::Release(_, _) => false,
        // Quantifiers contain temporal formulas
        CompiledLtlExpression::ForLoop { .. } | CompiledLtlExpression::Exists { .. } => false,
    }
}
