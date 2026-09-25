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

pub mod ltl;
mod product;

#[cfg(test)]
mod ltl_integration_tests;

use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use ltl::{automaton::BuchiAutomaton, compiled::CompiledLtlExpression};
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
    /// Index of the first loop edge in the returned LTL counterexample path.
    /// Terminal executions use an explicit `_stutter_` self-loop.
    pub violation_cycle_start: Option<usize>,
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
        let mut state = serializer.serialize_struct("StateGraph", 3)?;
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
    if max_states == Some(0) {
        return Err(AlthreadError::new(
            ErrorType::RuntimeError,
            None,
            "The state limit must be at least 1".to_string(),
        ));
    }
    let mut init_vm = VM::new(compiled_project);
    init_vm.start(0);

    let initial_vm = Rc::new(init_vm);
    let mut state_graph = StateGraph::new(initial_vm.clone());
    let mut known_states = HashMap::new();
    known_states.insert(initial_vm, state_graph.initial_state);

    let mut next_nodes = VecDeque::new();
    next_nodes.push_back(state_graph.initial_state);

    while let Some(current_state) = next_nodes.pop_front() {
        let current_vm = state_graph.vm(current_state).clone();
        let current_level = state_graph.nodes[current_state].level;
        let successors = current_vm.next()?;
        let mut fully_expanded = true;

        for (name, pid, instructions, actions, vm) in successors.into_iter() {
            let next_vm = Rc::new(vm);
            let lines = collect_instruction_lines(&instructions);
            let next_state = if let Some(existing_state) = known_states.get(&next_vm) {
                *existing_state
            } else {
                if max_states.is_some_and(|max| state_graph.nodes.len() >= max) {
                    state_graph.exhaustive = false;
                    fully_expanded = false;
                    continue;
                }
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

        state_graph.nodes[current_state].expanded = fully_expanded;
    }

    Ok(state_graph)
}
/// Return a nonempty counterexample when a property is violated. LTL witnesses
/// include a closed loop, whose first edge is `graph.violation_cycle_start`.
/// An empty path proves the properties only when `graph.exhaustive` is true;
/// otherwise the result is inconclusive. A state limit of zero is invalid.
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

/// Reuse the VM graph, but check independent properties in separate products.
/// A violation of any one property is sufficient; multiplying their automata
/// together adds exponential work without changing the verdict.
fn check_program_with_ltl<'a>(
    compiled_project: &'a CompiledProject,
    max_states: Option<usize>,
) -> AlthreadResult<(Vec<StateLink>, StateGraph<'a>)> {
    let mut state_graph = build_state_graph(compiled_project, max_states)?;

    for state_id in 0..state_graph.nodes.len() {
        if let Err(error) = state_graph.vm(state_id).check_invariants() {
            let mut path = build_violation_path(&state_graph, state_id);
            if path.is_empty() {
                path.push(StateLink {
                    to: state_id,
                    lines: error.pos.map(|pos| vec![pos.line()]).unwrap_or_default(),
                    instructions: vec![],
                    actions: vec![],
                    pid: 0,
                    name: "_init_".to_string(),
                });
            }
            return Ok((path, state_graph));
        }
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

fn build_violation_path(state_graph: &StateGraph<'_>, target: StateId) -> Vec<StateLink> {
    let mut path = Vec::new();
    let mut current = target;
    while let Some(predecessor) = state_graph.nodes[current].predecessor {
        let link = state_graph.nodes[predecessor]
            .successors
            .iter()
            .find(|link| link.to == current)
            .expect("every discovered VM state has its predecessor edge");
        path.push(link.clone());
        current = predecessor;
    }
    path.reverse();
    path
}
