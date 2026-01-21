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
/// Extended state for LTL verification combining VM state and monitor states
#[derive(Debug, Clone)]
struct ProductState {
    pub monitors: MonitoringState,
}

impl PartialEq for ProductState {
    fn eq(&self, other: &Self) -> bool {
        self.monitors == other.monitors
    }
}

impl Eq for ProductState {}

impl Hash for ProductState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash monitor states
        for monitors in &self.monitors.monitors_per_formula {
            for monitor in monitors {
                monitor.current_state_id.hash(state);
                // Hash bindings keys (values are too complex to hash reliably)
                let mut keys: Vec<_> = monitor.bindings.keys().collect();
                keys.sort();
                for key in keys {
                    key.hash(state);
                }
            }
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

/// Checks a program with LTL formulas using product automaton approach
fn check_program_with_ltl<'a>(
    compiled_project: &'a CompiledProject,
    max_states: Option<usize>,
) -> AlthreadResult<(Vec<StateLink<'a>>, StateGraph<'a>)> {
    // Step 1: Build Büchi automatons from compiled LTL formulas
    let automatons: Vec<BuchiAutomaton> = compiled_project
        .compiled_ltl_formulas
        .iter()
        .map(|formula| BuchiAutomaton::new(formula.clone()))
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

    // Step 4: Initialize state graph
    let mut state_graph = StateGraph {
        nodes: HashMap::new(),
        exhaustive: true,
    };

    state_graph
        .nodes
        .insert(initial_vm.clone(), GraphNode::new(None, 0));

    // Step 4: Track product states (VM + Monitors)
    let mut visited_product_states: HashMap<Rc<VM<'a>>, HashSet<ProductState>> = HashMap::new();
    visited_product_states.insert(
        initial_vm.clone(),
        [ProductState {
            monitors: initial_monitoring.clone(),
        }]
        .into_iter()
        .collect(),
    );

    let mut next_nodes = VecDeque::new();
    next_nodes.push_back((initial_vm.clone(), initial_monitoring));

    // Step 5: Explore state space with LTL checking
    while !next_nodes.is_empty() {
        if let Some(max) = max_states {
            if state_graph.nodes.len() >= max {
                state_graph.exhaustive = false;
                break;
            }
        }

        let (current_vm, current_monitors) = next_nodes.pop_front().unwrap();
        let current_level = state_graph.nodes.get(&current_vm).unwrap().level;

        // Get VM successors
        let successors = current_vm.next()?;

        for (name, pid, instructions, actions, next_vm) in successors.into_iter() {
            let next_vm = Rc::new(next_vm);

            let mut lines: Vec<usize> = instructions
                .iter()
                .map(|x| x.pos.clone().unwrap_or_default().line)
                .filter(|l| *l > 0)
                .collect();
            lines.sort();
            lines.dedup();

            // Add state link
            state_graph
                .nodes
                .get_mut(&current_vm)
                .unwrap()
                .successors
                .push(StateLink {
                    to: next_vm.clone(),
                    lines,
                    instructions,
                    actions,
                    pid,
                    name,
                });

            // Create new graph node if needed
            if !state_graph.nodes.contains_key(&next_vm) {
                state_graph.nodes.insert(
                    next_vm.clone(),
                    GraphNode::new(Some(current_vm.clone()), current_level + 1),
                );
            }

            // Monitor update and evolution
            // Debug print global state
            if let Some(req) = next_vm.globals.get("Request") {
                if let Some(grant) = next_vm.globals.get("Granted") {
                   log::debug!("State: Request={:?}, Granted={:?}", req, grant);
                }
            }

            let mut base_next_monitors = current_monitors.clone();

            // Check for new processes and update monitors
            ltl::quantifier::update_monitors_for_new_processes(
                &compiled_project.compiled_ltl_formulas,
                &automatons,
                &mut base_next_monitors,
                &next_vm,
            )?;

            // Get all possible next states for monitors (branching on non-determinism)
            let possible_next_monitoring_states =
                base_next_monitors.get_possible_successors(&next_vm, &automatons)?;

            for next_monitors in possible_next_monitoring_states {
                // Detect accepting runs of the Büchi automaton (observed counter-examples)
                if monitors_in_accepting_state(&next_monitors, &automatons) {
                    println!("LTL violation detected: acceptance condition reached");
                    let violation_path = build_violation_path(&state_graph, &next_vm)?;
                    return Ok((violation_path, state_graph));
                }

                // Check if this product state was already visited
                let product_state = ProductState {
                    monitors: next_monitors.clone(),
                };

                let already_visited = visited_product_states
                    .entry(next_vm.clone())
                    .or_insert_with(HashSet::new)
                    .contains(&product_state);

                if !already_visited {
                    visited_product_states
                        .get_mut(&next_vm)
                        .unwrap()
                        .insert(product_state);
                    next_nodes.push_back((next_vm.clone(), next_monitors));
                }
            }
        }

        // Check invariants (traditional safety properties)
        let check_ret = current_vm.check_invariants();
        if let Err(e) = check_ret {
            let mut path = Vec::new();
            let mut back_node = current_vm.clone();

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
            state_graph.nodes.get_mut(&current_vm).unwrap().eventually = true;
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

fn monitors_in_accepting_state(monitors: &MonitoringState, automatons: &[BuchiAutomaton]) -> bool {
    monitors
        .monitors_per_formula
        .iter()
        .enumerate()
        .any(|(formula_idx, monitors)| {
            let automaton = &automatons[formula_idx];
            monitors
                .iter()
                .any(|monitor| monitor.is_accepting(automaton))
        })
}
