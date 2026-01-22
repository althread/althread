//42 <- used to easily remove all comments made with this id
pub mod ltl;

#[cfg(test)]
mod ltl_integration_tests;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::{Hash, Hasher},
    rc::Rc,
};

use ltl::{automaton::BuchiAutomaton, compiled::CompiledLtlExpression, monitor::MonitoringState};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::{
    compiler::CompiledProject,
    error::{AlthreadError, AlthreadResult, ErrorType},
    vm::{instruction::Instruction, GlobalAction, VM},
};

#[derive(Debug, Clone)]
pub struct StateLink<'a> {
    pub instructions: Vec<Instruction>,
    pub actions: Vec<GlobalAction>,
    pub lines: Vec<usize>,
    pub pid: usize,
    pub name: String,
    pub to: Rc<VM<'a>>,
}

#[derive(Debug)]
pub struct GraphNode<'a> {
    pub level: usize,
    pub predecessor: Option<Rc<VM<'a>>>,
    pub successors: Vec<StateLink<'a>>,
    pub eventually: bool,
}

#[derive(Debug)]
pub struct StateGraph<'a> {
    pub nodes: HashMap<Rc<VM<'a>>, GraphNode<'a>>,
    pub exhaustive: bool,
}

impl<'a> std::fmt::Display for StateLink<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StateLink {{ lines: {:?}, pid: {}, name: {}, to: ... }}",
            self.lines, self.pid, self.name
        )
    }
}

impl<'a> Serialize for StateLink<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("StateLink", 5)?;
        state.serialize_field("lines", &self.lines)?;
        state.serialize_field("pid", &self.pid)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("to", &self.to.as_ref())?;
        state.serialize_field("actions", &self.actions)?;
        state.end()
    }
}
impl<'a> Serialize for GraphNode<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("GraphNode", 4)?;
        state.serialize_field("level", &self.level)?;
        let pred = if self.predecessor.is_some() {
            Some(self.predecessor.as_ref().unwrap().as_ref().clone())
        } else {
            None
        };
        state.serialize_field("predecessor", &pred)?;
        state.serialize_field("successors", &self.successors)?;
        state.serialize_field("eventually", &self.eventually)?;
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
                .nodes
                .iter()
                .map(|(key, node)| (key.as_ref(), node))
                .collect::<Vec<(&VM, &GraphNode)>>(),
        )?;
        state.serialize_field("exhaustive", &self.exhaustive)?;
        state.end()
    }
}

impl<'a> GraphNode<'a> {
    pub fn new(predecessor: Option<Rc<VM<'a>>>, level: usize) -> Self {
        Self {
            level,
            predecessor,
            eventually: false,
            successors: Vec::new(), //42 that’s what makes the state appear green even though it might be false.
        }
    }
}
/// Checks a given project, returning a path from an initial state to the first state that violates an invariant. (return an empty vector if no invariant is violated)
pub fn check_program<'a>(
    compiled_project: &'a CompiledProject,
    max_states: Option<usize>,
) -> AlthreadResult<(Vec<StateLink<'a>>, StateGraph<'a>)> {
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

    let mut state_graph = StateGraph {
        nodes: HashMap::new(),
        exhaustive: true,
    };

    //42 initialize a VM with the compiled project
    let mut init_vm = VM::new(compiled_project);
    init_vm.start(0);
    let initial_vm = Rc::new(init_vm);

    //42 initialize the state graph with the initial state of the state graph
    state_graph
        .nodes
        .insert(initial_vm.clone(), GraphNode::new(None, 0));

    //42 successors vector
    let mut next_nodes = VecDeque::new();
    next_nodes.push_back(initial_vm.clone());

    //42 while the successor list isn't empty
    while !next_nodes.is_empty() {
        if let Some(max) = max_states {
            if state_graph.nodes.len() >= max {
                state_graph.exhaustive = false;
                break;
            }
        }
        //42 we pick on the the next nodes and remove it from the vector
        let current_node = next_nodes.pop_front().unwrap();
        let current_level = state_graph.nodes.get_mut(&current_node).unwrap().level;
        //42 successors vector of current node
        let successors = current_node.next()?;

        //42 we go through all successors
        for (name, pid, instructions, actions, vm) in successors.into_iter() {
            let vm: Rc<VM<'_>> = Rc::new(vm);

            let mut lines: Vec<usize> = instructions
                .iter()
                .map(|x| x.pos.clone().unwrap_or_default().line)
                .filter(|l| *l > 0)
                .collect();
            //42 remove all dupes
            lines.sort();
            lines.dedup();

            //42 add all the state links allowing transition from the current node to another one to the state graph
            state_graph
                .nodes
                .get_mut(&current_node)
                .unwrap()
                .successors
                .push(StateLink {
                    to: vm.clone(),
                    lines,
                    instructions,
                    actions,
                    pid,
                    name,
                });

            //42 if the graphnode resulting from a statelink transition don't yet exist, create it
            if !state_graph.nodes.contains_key(&vm.clone()) {
                state_graph.nodes.insert(
                    vm.clone(),
                    GraphNode::new(Some(current_node.clone()), current_level + 1),
                );
                next_nodes.push_back(vm.clone());
            }
        }

        //42 check invariants
        let check_ret = current_node.check_invariants();
        if let Err(e) = check_ret {
            let mut path = Vec::new();
            let mut back_node = current_node.clone();

            if state_graph
                .nodes
                .get(&back_node)
                .unwrap()
                .predecessor
                .is_none()
            {
                let lines = if let Some(pos) = &e.pos {
                    vec![pos.line]
                } else {
                    vec![]
                };
                path.push(StateLink {
                    to: back_node.clone(),
                    lines,
                    instructions: vec![],
                    actions: vec![],
                    pid: 0,
                    name: "_init_".to_string(),
                });
                return Ok((path, state_graph));
            }

            while let Some(pred) = state_graph
                .nodes
                .get(&back_node)
                .unwrap()
                .predecessor
                .clone()
            {
                path.push(
                    state_graph
                        .nodes
                        .get(&pred)
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
            state_graph.nodes.get_mut(&current_node).unwrap().eventually = true;
        }
    }

    //42 if the search was not exhaustive, we cannot check eventually violations
    if !state_graph.exhaustive {
        return Ok((vec![], state_graph));
    }

    //42 now checking eventually violations

    // path visit is used to keep track of the successors we've already checked
    let mut path_visit: Vec<usize> = Vec::new();
    let mut path = Vec::new();
    let mut path_set = std::collections::HashSet::new();
    // if root node check eventually condition no path can exist
    if state_graph.nodes.get(&initial_vm).unwrap().eventually {
        return Ok((vec![], state_graph));
    }

    // retrieving the state Link of the initial VM
    path.push(initial_vm.clone());
    path_set.insert(initial_vm.clone());
    // no successors have yet been visited
    path_visit.push(0);

    while !path.is_empty() {
        let curr_vm = {
            let temp = path.last().unwrap();
            temp.clone() // Drops immutable borrow IMMEDIATELY
        };

        let mut visited_succ = path_visit.pop().unwrap();

        // get all the successors of the current node
        let mut succ = Vec::new();
        for link in state_graph
            .nodes
            .get(&curr_vm)
            .unwrap()
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
                // reconstruct_path takes a Vec of VMs.
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
            let graph_node = state_graph.nodes.get(curr_succ.to.as_ref()).unwrap();
            if !graph_node.eventually {
                explorable_path = true;
                path.push(curr_succ.to.clone());
                path_set.insert(curr_succ.to.clone());
                // we update the number of visited successors of the current node
                path_visit.push(visited_succ);
                // we then init the number of visited successors from the new node in the path
                path_visit.push(0);
            }
        }
        // if no explorable path was found we condemn this node (it is a dead end)
        if !explorable_path {
            state_graph.nodes.get_mut(&curr_vm).unwrap().eventually = true;
            let popped = path.pop();
            if let Some(p) = popped {
                path_set.remove(&p);
            }
        }
    }
    Ok((vec![], state_graph))
}

pub fn reconstruct_path<'a>(
    mut vec_vm: Vec<Rc<VM>>,
    state_graph: &StateGraph<'a>,
) -> AlthreadResult<Vec<StateLink<'a>>> {
    let mut ret_path = Vec::new();
    let mut back_node = vec_vm.pop().unwrap();

    while let Some(pred) = vec_vm.pop() {
        ret_path.push(
            state_graph
                .nodes
                .get(&pred)
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
struct CombinedProductState<'a> {
    vm: Rc<VM<'a>>,
    monitors: MonitoringState,
}

/// Checks a program with LTL formulas using Nested DFS algorithm for cycle detection
/// 
/// The Nested DFS algorithm works as follows:
/// 1. First DFS explores the product automaton (VM states × monitor states)
/// 2. When backtracking from an accepting state (post-order), launch a second DFS
/// 3. If the second DFS can reach the accepting state again, we found an accepting cycle
/// 4. An accepting cycle means the negated LTL formula is satisfiable → original formula violated
fn check_program_with_ltl<'a>(
    compiled_project: &'a CompiledProject,
    max_states: Option<usize>,
) -> AlthreadResult<(Vec<StateLink<'a>>, StateGraph<'a>)> {
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

    // Step 2: Initialize VM
    let mut init_vm = VM::new(compiled_project);
    init_vm.start(0);

    // Step 3: Initialize monitoring state with proper quantifier handling
    let initial_monitoring = ltl::quantifier::initialize_monitoring(
        &compiled_project.compiled_ltl_formulas,
        &automatons,
        &init_vm,
    )?;

    let initial_vm = Rc::new(init_vm);

    // Step 4: Initialize state graph (for visualization/debugging)
    let mut state_graph = StateGraph {
        nodes: HashMap::new(),
        exhaustive: true,
    };

    state_graph
        .nodes
        .insert(initial_vm.clone(), GraphNode::new(None, 0));

    // ============================================================
    // NESTED DFS ALGORITHM FOR ACCEPTING CYCLE DETECTION
    // ============================================================
    
    // Track visited states for the outer DFS
    let mut visited_outer: HashSet<CombinedProductState<'a>> = HashSet::new();
    // Track states on the current DFS stack (for cycle detection in inner DFS)
    let mut on_stack: HashSet<CombinedProductState<'a>> = HashSet::new();
    // Track visited states for the inner DFS (reset for each accepting state)
    let mut visited_inner: HashSet<CombinedProductState<'a>> = HashSet::new();
    
    // Store the graph edges for path reconstruction
    let mut product_edges: HashMap<CombinedProductState<'a>, Vec<CombinedProductState<'a>>> = HashMap::new();
    
    // Initial product state
    let initial_product_state = CombinedProductState {
        vm: initial_vm.clone(),
        monitors: initial_monitoring.clone(),
    };
    
    // Stack for iterative DFS: (state, phase)
    // phase 0 = first visit, phase 1 = post-order (after children explored)
    let mut dfs_stack: Vec<(CombinedProductState<'a>, usize)> = vec![(initial_product_state.clone(), 0)];
    
    // State counter for limiting exploration
    let mut state_count = 0;
    
    while let Some((current_state, phase)) = dfs_stack.pop() {
        if phase == 0 {
            // First visit to this state
            if visited_outer.contains(&current_state) {
                continue;
            }
            
            state_count += 1;
            if let Some(max) = max_states {
                if state_count >= max {
                    state_graph.exhaustive = false;
                    break;
                }
            }
            
            visited_outer.insert(current_state.clone());
            on_stack.insert(current_state.clone());
            
            // Push post-order visit
            dfs_stack.push((current_state.clone(), 1));
            
            let current_vm = &current_state.vm;
            let current_monitors = &current_state.monitors;
            let current_level = state_graph.nodes.get(current_vm).map(|n| n.level).unwrap_or(0);
            
            // Get VM successors
            let successors = current_vm.next()?;
            
            // Handle terminal states (stuttering)
            // This includes both proper termination (is_finished=true) and deadlock states
            // (no successors but processes still waiting). In both cases, the execution
            // can only "stutter" in place forever.
            if successors.is_empty() {
                let is_finished = current_vm.is_finished();
                log::debug!("DEBUG: Terminal state - is_finished={}", is_finished);
                
                // For both termination and deadlock, we need to check if the monitor
                // can accept on a stuttering loop (staying in the same state forever).
                // This is required for "eventually P" properties - if P never becomes true
                // and we reach a state with no further progress, it's a violation.
                
                let mut base_next_monitors = current_monitors.clone();
                ltl::quantifier::update_monitors_for_new_processes(
                    &compiled_project.compiled_ltl_formulas,
                    &automatons,
                    &mut base_next_monitors,
                    current_vm,
                    current_vm,
                )?;
                
                let possible_next_monitoring_states =
                    base_next_monitors.get_possible_successors(current_vm, &automatons)?;
                
                for next_monitors in possible_next_monitoring_states {
                    let is_accepting = monitors_in_accepting_state(
                        &next_monitors,
                        &automatons,
                        &compiled_project.compiled_ltl_formulas,
                    );
                    
                    // Terminal/deadlock state with accepting monitor = infinite accepting run via stuttering
                    if is_accepting {
                        let state_type = if is_finished { "terminal" } else { "deadlock" };
                        log::debug!("LTL violation: {} state with accepting monitor (stuttering loop)", state_type);
                        println!("LTL violation detected: accepting {} state (stuttering)", state_type);
                        let violation_path = build_violation_path(&state_graph, current_vm)?;
                        return Ok((violation_path, state_graph));
                    }
                }
                continue;
            }
            
            // Process successors
            for (name, pid, instructions, actions, next_vm) in successors.into_iter() {
                let next_vm = Rc::new(next_vm);
                
                let mut lines: Vec<usize> = instructions
                    .iter()
                    .map(|x| x.pos.clone().unwrap_or_default().line)
                    .filter(|l| *l > 0)
                    .collect();
                lines.sort();
                lines.dedup();
                
                // Add to state graph for visualization
                state_graph
                    .nodes
                    .entry(current_vm.clone())
                    .or_insert_with(|| GraphNode::new(None, current_level))
                    .successors
                    .push(StateLink {
                        to: next_vm.clone(),
                        lines,
                        instructions,
                        actions,
                        pid,
                        name,
                    });
                
                if !state_graph.nodes.contains_key(&next_vm) {
                    state_graph.nodes.insert(
                        next_vm.clone(),
                        GraphNode::new(Some(current_vm.clone()), current_level + 1),
                    );
                }
                
                // Update monitors for this transition
                let mut base_next_monitors = current_monitors.clone();
                ltl::quantifier::update_monitors_for_new_processes(
                    &compiled_project.compiled_ltl_formulas,
                    &automatons,
                    &mut base_next_monitors,
                    current_vm,
                    &next_vm,
                )?;
                
                let possible_next_monitoring_states =
                    base_next_monitors.get_possible_successors(&next_vm, &automatons)?;
                
                for next_monitors in possible_next_monitoring_states {
                    // Check if we've reached an accepting state with only propositional formulas.
                    // In that case, the "eventually" is immediately satisfied - no cycle needed.
                    let is_immediate_violation = monitors_in_immediate_accepting_state(
                        &next_monitors,
                        &automatons,
                        &compiled_project.compiled_ltl_formulas,
                    );
                    
                    if is_immediate_violation {
                        log::debug!("LTL violation: reached accepting state with propositional formulas only");
                        println!("LTL violation detected: immediate acceptance (propositional formula satisfied)");
                        let violation_path = build_violation_path(&state_graph, &next_vm)?;
                        return Ok((violation_path, state_graph));
                    }
                    
                    let next_product_state = CombinedProductState {
                        vm: next_vm.clone(),
                        monitors: next_monitors,
                    };
                    
                    // Record edge for path reconstruction
                    product_edges
                        .entry(current_state.clone())
                        .or_insert_with(Vec::new)
                        .push(next_product_state.clone());
                    
                    // Add to DFS stack if not visited
                    if !visited_outer.contains(&next_product_state) {
                        dfs_stack.push((next_product_state, 0));
                    }
                }
            }
        } else {
            // Post-order visit (phase 1): all children have been explored
            on_stack.remove(&current_state);
            
            // Check if this is an accepting state
            let is_accepting = monitors_in_accepting_state(
                &current_state.monitors,
                &automatons,
                &compiled_project.compiled_ltl_formulas,
            );
            
            if is_accepting {
                log::debug!("DEBUG: Post-order visit of accepting state, launching inner DFS");
                
                // Launch inner DFS to find a cycle back to this accepting state
                visited_inner.clear();
                let mut inner_stack: Vec<CombinedProductState<'a>> = vec![current_state.clone()];
                
                while let Some(inner_current) = inner_stack.pop() {
                    if visited_inner.contains(&inner_current) {
                        continue;
                    }
                    visited_inner.insert(inner_current.clone());
                    
                    // Get successors of inner_current
                    if let Some(successors) = product_edges.get(&inner_current) {
                        for successor in successors {
                            // Check if we found a cycle back to the accepting state
                            if *successor == current_state {
                                log::debug!("DEBUG: Found accepting cycle!");
                                println!("LTL violation detected: accepting cycle found");
                                let violation_path = build_violation_path(&state_graph, &current_state.vm)?;
                                return Ok((violation_path, state_graph));
                            }
                            
                            // Also check if successor is on the current DFS stack
                            // (this means there's a path from accepting state through successor back to stack)
                            if on_stack.contains(successor) {
                                // There's a cycle, and we're starting from an accepting state
                                // Check if any state in the cycle is accepting
                                log::debug!("DEBUG: Found cycle through stack from accepting state");
                                println!("LTL violation detected: accepting cycle found (via stack)");
                                let violation_path = build_violation_path(&state_graph, &current_state.vm)?;
                                return Ok((violation_path, state_graph));
                            }
                            
                            if !visited_inner.contains(successor) {
                                inner_stack.push(successor.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Traditional invariant checking (separate pass for safety properties)
    // This is done on the state graph we built
    for (vm, node) in state_graph.nodes.iter() {
        let check_ret = vm.check_invariants();
        if let Err(e) = check_ret {
            let violation_path = build_violation_path(&state_graph, vm)?;
            if violation_path.is_empty() {
                // Initial state violation
                let lines = if let Some(pos) = &e.pos {
                    vec![pos.line]
                } else {
                    vec![]
                };
                return Ok((vec![StateLink {
                    to: vm.clone(),
                    lines,
                    instructions: vec![],
                    actions: vec![],
                    pid: 0,
                    name: "_init_".to_string(),
                }], state_graph));
            }
            return Ok((violation_path, state_graph));
        }
    }

    // No violations found
    println!("LTL verification completed: no violations found");
    Ok((vec![], state_graph))
}

fn build_violation_path<'a>(
    state_graph: &StateGraph<'a>,
    target: &Rc<VM<'a>>,
) -> AlthreadResult<Vec<StateLink<'a>>> {
    let mut path = Vec::new();
    let mut back_node = target.clone();

    while let Some(pred) = state_graph
        .nodes
        .get(&back_node)
        .unwrap()
        .predecessor
        .clone()
    {
        let link = state_graph
            .nodes
            .get(&pred)
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
                    monitors.iter().all(|monitor| monitor.is_accepting(automaton))
                }
                _ => monitors
                    .iter()
                    .any(|monitor| monitor.is_accepting(automaton)),
            }
        })
}

/// Check if any monitor is in an accepting state that leads to a sink state.
/// A sink state is a state with only self-loops or transitions to other sink states.
/// This handles the case of "eventually P" where reaching the accepting state once
/// is sufficient (no cycle through accepting state needed).
#[allow(dead_code)]
fn monitors_in_accepting_sink(
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
                        monitor.is_accepting(automaton) && 
                        is_in_sink_region(monitor.current_state_id, automaton)
                    })
                }
                _ => monitors.iter().any(|monitor| {
                    monitor.is_accepting(automaton) && 
                    is_in_sink_region(monitor.current_state_id, automaton)
                }),
            }
        })
}

/// Check if any monitor is in an accepting state where all formulas are propositional.
/// This means the "eventually P" has been satisfied immediately - no cycle needed.
/// 
/// For example:
/// - `eventually (Request ∧ ¬Granted)` - when Request=true and Granted=false, 
///   the formula is immediately satisfied (propositional)
/// - `eventually (Request ∧ always ¬Granted)` - even when Request=true,
///   we still need to verify `always ¬Granted` holds infinitely (temporal)
fn monitors_in_immediate_accepting_state(
    monitors: &MonitoringState,
    automatons: &[BuchiAutomaton],
    formulas: &[CompiledLtlExpression],
) -> bool {
    monitors
        .monitors_per_formula
        .iter()
        .enumerate()
        .any(|(formula_idx, monitors_for_formula)| {
            let automaton = &automatons[formula_idx];
            
            match &formulas[formula_idx] {
                CompiledLtlExpression::Exists { .. } => {
                    if monitors_for_formula.is_empty() {
                        return true;
                    }
                    monitors_for_formula.iter().all(|monitor| {
                        monitor.is_accepting(automaton) && 
                        state_has_only_propositional_formulas(monitor.current_state_id, automaton)
                    })
                }
                _ => monitors_for_formula.iter().any(|monitor| {
                    monitor.is_accepting(automaton) && 
                    state_has_only_propositional_formulas(monitor.current_state_id, automaton)
                }),
            }
        })
}

/// Check if all formulas in an automaton state are propositional (no temporal operators).
/// If true, reaching this accepting state is an immediate violation.
fn state_has_only_propositional_formulas(state_id: usize, automaton: &BuchiAutomaton) -> bool {
    let state = &automaton.states[state_id];
    state.formulas.iter().all(|f| f.is_propositional())
}

/// Check if a state is in a "sink region" - a region of the automaton 
/// from which no accepting state can be reached (other than possibly itself).
/// This includes:
/// 1. Accepting states that only transition to sink states
/// 2. Non-accepting states with only self-loops or transitions to other sinks
fn is_in_sink_region(state_id: usize, automaton: &BuchiAutomaton) -> bool {
    // Get the state
    let state = &automaton.states[state_id];
    
    // If the state has transitions to other accepting states, it's not a pure sink
    // Check if all transitions go to non-accepting states or to itself
    for &next_id in &state.transitions {
        if next_id == state_id {
            // Self-loop is fine
            continue;
        }
        let next_state = &automaton.states[next_id];
        // If we can reach another accepting state, not a sink
        if !next_state.acceptance_sets.is_empty() {
            return false;
        }
        // Check if the next state can eventually reach an accepting state
        // (do a simple reachability check)
        if can_reach_accepting_state(next_id, automaton, &mut vec![state_id]) {
            return false;
        }
    }
    true
}

/// Check if from a given state we can reach any accepting state
fn can_reach_accepting_state(
    start_id: usize,
    automaton: &BuchiAutomaton,
    visited: &mut Vec<usize>,
) -> bool {
    if visited.contains(&start_id) {
        return false;
    }
    visited.push(start_id);
    
    let state = &automaton.states[start_id];
    
    // If this state is accepting, we found one
    if !state.acceptance_sets.is_empty() {
        return true;
    }
    
    // Check successors
    for &next_id in &state.transitions {
        if can_reach_accepting_state(next_id, automaton, visited) {
            return true;
        }
    }
    
    false
}
