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
//! 4. Find strongly connected components covering every acceptance set
//! 5. An accepting cycle means the negated formula is satisfiable → original violated

pub(crate) mod fxhash;
pub mod ltl;
mod product;

#[cfg(test)]
mod ltl_integration_tests;

use std::{
    collections::{hash_map::Entry, VecDeque},
    rc::Rc,
};

use fxhash::FxHashMap;

use ltl::{automaton::BuchiAutomaton, compiled::CompiledLtlExpression};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::{
    compiler::CompiledProject,
    error::{AlthreadError, AlthreadResult, ErrorType},
    vm::{instruction::Instruction, CheckerState, GlobalAction, ScheduledStep, VM},
};

pub type StateId = usize;

#[derive(Debug, Clone)]
pub struct StateLink<'a> {
    pub instructions: Vec<&'a Instruction>,
    pub actions: Vec<GlobalAction>,
    pub lines: Vec<usize>,
    pub pid: usize,
    pub name: String,
    pub to: StateId,
    step: Option<ScheduledStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge {
    pub to: StateId,
    pub(crate) step: ScheduledStep,
}

impl Serialize for GraphEdge {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("GraphEdge", 1)?;
        state.serialize_field("to", &self.to)?;
        state.end()
    }
}

#[derive(Debug)]
pub struct GraphNode {
    pub level: usize,
    pub predecessor: Option<StateId>,
    pub successors: Vec<GraphEdge>,
    pub eventually: bool,
    pub expanded: bool,
}

#[derive(Debug)]
pub struct StateGraph<'a> {
    pub states: Vec<Rc<VM<'a>>>,
    pub nodes: Vec<GraphNode>,
    pub initial_state: StateId,
    pub exhaustive: bool,
    /// Index of the first loop edge in the returned LTL counterexample path.
    /// Terminal executions use an explicit `_stutter_` self-loop.
    pub violation_cycle_start: Option<usize>,
}

impl StateLink<'_> {
    pub fn is_initial_marker(&self) -> bool {
        self.step.is_none() && self.name == "_init_"
    }
}

impl std::fmt::Display for StateLink<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StateLink {{ lines: {:?}, pid: {}, name: {}, to: {} }}",
            self.lines, self.pid, self.name, self.to
        )
    }
}

impl Serialize for StateLink<'_> {
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
        let mut state = serializer.serialize_struct("StateGraph", 3)?;
        state.serialize_field(
            "nodes",
            &self
                .states
                .iter()
                .zip(self.nodes.iter())
                .map(|(vm, node)| (vm.as_ref(), node))
                .collect::<Vec<_>>(),
        )?;
        state.serialize_field("exhaustive", &self.exhaustive)?;
        state.serialize_field("violation_cycle_start", &self.violation_cycle_start)?;
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
            violation_cycle_start: None,
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

    /// Re-execute a single stored graph edge to build its full `StateLink` details
    /// (instructions, actions, lines, process name/id) on demand.
    pub fn materialize_edge(
        &self,
        from: StateId,
        edge: &GraphEdge,
    ) -> AlthreadResult<StateLink<'a>> {
        let mut vm = self.vm(from).as_ref().clone();
        let info = vm.advance(&edge.step)?.ok_or_else(|| {
            AlthreadError::new(
                ErrorType::RuntimeError,
                None,
                "Stored graph edge could not be materialized".to_string(),
            )
        })?;
        Ok(StateLink {
            to: edge.to,
            lines: collect_instruction_lines(&info.instructions),
            instructions: info.instructions,
            actions: info.actions,
            pid: info.prog_id,
            name: info.prog_name,
            step: Some(edge.step.clone()),
        })
    }

    /// Materialize full `StateLink` details for all outgoing edges of `state_id`.
    pub fn successor_links(&self, state_id: StateId) -> AlthreadResult<Vec<StateLink<'a>>> {
        self.nodes[state_id]
            .successors
            .iter()
            .map(|edge| self.materialize_edge(state_id, edge))
            .collect()
    }

    /// Re-execute the exact scheduled steps to recover path-specific message IDs.
    pub fn replay_path(
        &self,
        path: &[StateLink<'a>],
        mut visit: impl FnMut(&StateLink<'a>, &VM<'a>, &[GlobalAction]),
    ) -> AlthreadResult<()> {
        let mut vm = self.vm(self.initial_state).as_ref().clone();
        for link in path {
            if let Some(step) = &link.step {
                let info = vm.advance(step)?.ok_or_else(|| {
                    AlthreadError::new(
                        ErrorType::RuntimeError,
                        None,
                        "Counterexample step could not be replayed".to_string(),
                    )
                })?;
                visit(link, &vm, &info.actions);
            } else {
                visit(link, &vm, &[]);
            }
        }
        Ok(())
    }
}

fn collect_instruction_lines(instructions: &[&Instruction]) -> Vec<usize> {
    let mut lines: Vec<usize> = instructions
        .iter()
        .filter_map(|instruction| instruction.pos.as_ref().map(|pos| pos.line()))
        .filter(|line| *line > 0)
        .collect();
    lines.sort();
    lines.dedup();
    lines
}

fn build_state_graph<'a>(
    compiled_project: &'a CompiledProject,
    max_states: Option<usize>,
) -> AlthreadResult<(StateGraph<'a>, Option<(StateId, AlthreadError)>)> {
    if max_states == Some(0) {
        return Err(AlthreadError::new(
            ErrorType::RuntimeError,
            None,
            "The state limit must be at least 1".to_string(),
        ));
    }
    let check_invariants = !compiled_project.always_conditions.is_empty();

    let mut init_vm = VM::new(compiled_project);
    init_vm.start(0);

    let initial_vm = Rc::new(init_vm);
    let mut state_graph = StateGraph::new(initial_vm.clone());
    if check_invariants {
        if let Err(err) = initial_vm.check_invariants() {
            if !matches!(err.error_type, ErrorType::InvariantError) {
                return Err(err);
            }
            let initial_state = state_graph.initial_state;
            state_graph.exhaustive = false;
            return Ok((state_graph, Some((initial_state, err))));
        }
    }

    let mut known_states: FxHashMap<CheckerState<'a>, StateId> = FxHashMap::default();
    known_states.insert(CheckerState(initial_vm), state_graph.initial_state);

    let mut next_nodes = VecDeque::new();
    next_nodes.push_back(state_graph.initial_state);

    while let Some(current_state) = next_nodes.pop_front() {
        let current_vm = state_graph.vm(current_state).clone();
        let current_level = state_graph.nodes[current_state].level;
        let successors = current_vm.successors_lightweight()?;
        let mut fully_expanded = true;

        for (step, vm) in successors {
            let next_vm = Rc::new(vm);
            let mut newly_discovered = false;
            let next_state = match known_states.entry(CheckerState(next_vm.clone())) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    if max_states.is_some_and(|max| state_graph.nodes.len() >= max) {
                        state_graph.exhaustive = false;
                        fully_expanded = false;
                        continue;
                    }
                    let new_state =
                        state_graph.push_state(next_vm, Some(current_state), current_level + 1);
                    entry.insert(new_state);
                    next_nodes.push_back(new_state);
                    newly_discovered = true;
                    new_state
                }
            };

            state_graph.nodes[current_state]
                .successors
                .push(GraphEdge {
                    to: next_state,
                    step,
                });

            if newly_discovered && check_invariants {
                if let Err(err) = state_graph.vm(next_state).check_invariants() {
                    if !matches!(err.error_type, ErrorType::InvariantError) {
                        return Err(err);
                    }
                    state_graph.exhaustive = false;
                    return Ok((state_graph, Some((next_state, err))));
                }
            }
        }

        state_graph.nodes[current_state].expanded = fully_expanded;
    }

    Ok((state_graph, None))
}
#[derive(Debug)]
pub enum Verdict<'a> {
    Verified,
    Violated {
        path: Vec<StateLink<'a>>,
        cycle_start: Option<usize>,
    },
    Inconclusive,
}

#[derive(Debug)]
pub struct CheckResult<'a> {
    pub verdict: Verdict<'a>,
    pub graph: StateGraph<'a>,
}

/// Check the project and return its verdict together with the explored graph.
pub fn check(
    compiled_project: &CompiledProject,
    max_states: Option<usize>,
) -> AlthreadResult<CheckResult<'_>> {
    let (path, graph) = check_program(compiled_project, max_states)?;
    let verdict = if !path.is_empty() {
        Verdict::Violated {
            path,
            cycle_start: graph.violation_cycle_start,
        }
    } else if graph.exhaustive {
        Verdict::Verified
    } else {
        Verdict::Inconclusive
    };
    Ok(CheckResult { verdict, graph })
}

/// Return a nonempty counterexample when a property is violated. LTL witnesses
/// include a closed loop, whose first edge is `graph.violation_cycle_start`.
/// An empty path proves the properties only when `graph.exhaustive` is true;
/// otherwise the result is inconclusive. A state limit of zero is invalid.
pub fn check_program<'a>(
    compiled_project: &'a CompiledProject,
    max_states: Option<usize>,
) -> AlthreadResult<(Vec<StateLink<'a>>, StateGraph<'a>)> {
    if !compiled_project.compiled_ltl_formulas.is_empty() {
        log::debug!(
            "Found {} compiled LTL formulas in the project",
            compiled_project.compiled_ltl_formulas.len()
        );
        for (i, formula) in compiled_project.compiled_ltl_formulas.iter().enumerate() {
            log::debug!("Compiled LTL Formula #{}: {}", i + 1, formula);
        }
        log::debug!("Starting LTL verification...");
        return check_program_with_ltl(compiled_project, max_states);
    }

    let (state_graph, invariant_violation) = build_state_graph(compiled_project, max_states)?;

    if let Some((violated_state, error)) = invariant_violation {
        let mut path = build_violation_path(&state_graph, violated_state)?;
        if path.is_empty() {
            path.push(StateLink {
                to: violated_state,
                lines: error.pos.map(|pos| vec![pos.line()]).unwrap_or_default(),
                instructions: vec![],
                actions: vec![],
                pid: 0,
                name: "_init_".to_string(),
                step: None,
            });
        }
        return Ok((path, state_graph));
    }

    Ok((vec![], state_graph))
}

pub fn reconstruct_path<'a>(
    mut vec_vm: Vec<StateId>,
    state_graph: &StateGraph<'a>,
) -> AlthreadResult<Vec<StateLink<'a>>> {
    let mut ret_path = Vec::new();
    let mut back_node = vec_vm.pop().unwrap();

    while let Some(pred) = vec_vm.pop() {
        let edge = state_graph
            .nodes
            .get(pred)
            .unwrap()
            .successors
            .iter()
            .find(|x| x.to == back_node)
            .unwrap();
        ret_path.push(state_graph.materialize_edge(pred, edge)?);

        back_node = pred;
    }
    Ok(ret_path)
}

/// Reuse the VM graph, but check independent properties in separate products.
/// A violation of any one property is sufficient; multiplying their automata
/// together adds exponential work without changing the verdict.
fn check_program_with_ltl<'a>(
    compiled_project: &'a CompiledProject,
    max_states: Option<usize>,
) -> AlthreadResult<(Vec<StateLink<'a>>, StateGraph<'a>)> {
    let (mut state_graph, invariant_violation) = build_state_graph(compiled_project, max_states)?;

    if let Some((state_id, error)) = invariant_violation {
        let mut path = build_violation_path(&state_graph, state_id)?;
        if path.is_empty() {
            path.push(StateLink {
                to: state_id,
                lines: error.pos.map(|pos| vec![pos.line()]).unwrap_or_default(),
                instructions: vec![],
                actions: vec![],
                pid: 0,
                name: "_init_".to_string(),
                step: None,
            });
        }
        return Ok((path, state_graph));
    }

    for formula in &compiled_project.compiled_ltl_formulas {
        let body = match formula {
            CompiledLtlExpression::ForLoop { body, .. }
            | CompiledLtlExpression::Exists { body, .. } => body.as_ref(),
            _ => formula,
        };
        let automaton = BuchiAutomaton::new(body.clone());
        if let Some((path, cycle_start)) =
            product::check_formula(&state_graph, formula, &automaton)?
        {
            state_graph.violation_cycle_start = Some(cycle_start);
            return Ok((path, state_graph));
        }
    }

    Ok((vec![], state_graph))
}

fn build_violation_path<'a>(
    state_graph: &StateGraph<'a>,
    target: StateId,
) -> AlthreadResult<Vec<StateLink<'a>>> {
    let mut path = Vec::new();
    let mut current = target;
    while let Some(predecessor) = state_graph.nodes[current].predecessor {
        let edge = state_graph.nodes[predecessor]
            .successors
            .iter()
            .find(|edge| edge.to == current)
            .expect("every discovered VM state has its predecessor edge");
        path.push(state_graph.materialize_edge(predecessor, edge)?);
        current = predecessor;
    }
    path.reverse();
    Ok(path)
}
