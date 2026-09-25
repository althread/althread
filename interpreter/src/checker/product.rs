//! Generalized Büchi emptiness and lasso reconstruction for one property.

use std::collections::{HashMap, VecDeque};

use super::{
    ltl::{
        automaton::BuchiAutomaton, compiled::CompiledLtlExpression, monitor::MonitoringState,
        quantifier,
    },
    StateGraph, StateId, StateLink,
};
use crate::error::AlthreadResult;

#[derive(Clone, PartialEq, Eq, Hash)]
struct ProductState {
    vm: StateId,
    monitors: MonitoringState,
}

#[derive(Clone, Copy)]
struct ProductEdge {
    to: usize,
    /// Index in the source VM node's successors; None denotes terminal stuttering.
    vm_edge: Option<usize>,
}

struct ProductNode {
    state: ProductState,
    successors: Vec<ProductEdge>,
    /// Source product node and outgoing edge index, preserving monitor history.
    predecessor: Option<(usize, usize)>,
}

#[derive(Default)]
struct ProductGraph {
    nodes: Vec<ProductNode>,
    known: HashMap<ProductState, usize>,
}

impl ProductGraph {
    fn intern(&mut self, state: ProductState, predecessor: Option<(usize, usize)>) -> usize {
        if let Some(&id) = self.known.get(&state) {
            return id;
        }
        let id = self.nodes.len();
        self.known.insert(state.clone(), id);
        self.nodes.push(ProductNode {
            state,
            successors: Vec::new(),
            predecessor,
        });
        id
    }

    fn components(&self) -> Vec<Vec<usize>> {
        let mut visited = vec![false; self.nodes.len()];
        let mut finish_order = Vec::with_capacity(self.nodes.len());
        for root in 0..self.nodes.len() {
            if visited[root] {
                continue;
            }
            visited[root] = true;
            let mut stack = vec![(root, 0)];
            while let Some((node, next_edge)) = stack.last_mut() {
                if let Some(edge) = self.nodes[*node].successors.get(*next_edge) {
                    *next_edge += 1;
                    if !visited[edge.to] {
                        visited[edge.to] = true;
                        stack.push((edge.to, 0));
                    }
                } else {
                    finish_order.push(*node);
                    stack.pop();
                }
            }
        }

        let mut reverse = vec![Vec::new(); self.nodes.len()];
        for (from, node) in self.nodes.iter().enumerate() {
            for edge in &node.successors {
                reverse[edge.to].push(from);
            }
        }
        let mut assigned = vec![false; self.nodes.len()];
        let mut components = Vec::new();
        for root in finish_order.into_iter().rev() {
            if assigned[root] {
                continue;
            }
            assigned[root] = true;
            let mut stack = vec![root];
            let mut component = Vec::new();
            while let Some(node) = stack.pop() {
                component.push(node);
                for &predecessor in &reverse[node] {
                    if !assigned[predecessor] {
                        assigned[predecessor] = true;
                        stack.push(predecessor);
                    }
                }
            }
            components.push(component);
        }
        components
    }

    /// Return states covering every acceptance set for the SAME instance.
    /// Exists requires coverage for every instance; forall needs only one
    /// counterexample instance. Sets need not be visited simultaneously.
    fn acceptance_targets(
        &self,
        component: &[usize],
        automaton: &BuchiAutomaton,
        existential: bool,
    ) -> Option<Vec<usize>> {
        let root = component[0];
        // Instances are appended and never removed or reordered (rejected
        // runs keep their binding). Thus an SCC has a fixed domain and each
        // monitor index denotes the same instance throughout the component.
        let monitors = &self.nodes[root].state.monitors.monitors_per_formula[0];
        let mut targets = Vec::new();
        for instance in 0..monitors.len() {
            let mut instance_targets = Vec::new();
            let active = component.iter().copied().find(|&node| {
                self.nodes[node].state.monitors.monitors_per_formula[0][instance].current_state_id
                    < automaton.states.len()
            });
            if let Some(active) = active {
                instance_targets.push(active);
                for set in 0..automaton.num_acceptance_sets {
                    if let Some(target) = component.iter().copied().find(|&node| {
                        self.nodes[node].state.monitors.monitors_per_formula[0][instance]
                            .is_in_accepting_state(automaton, set)
                    }) {
                        instance_targets.push(target);
                    } else {
                        instance_targets.clear();
                        break;
                    }
                }
            }
            if existential {
                if instance_targets.is_empty() {
                    return None;
                }
                targets.extend(instance_targets);
            } else if !instance_targets.is_empty() {
                return Some(instance_targets);
            }
        }
        if existential {
            // An empty domain cannot satisfy an existential property.
            targets.push(root);
            Some(targets)
        } else {
            None
        }
    }

    fn state_link(&self, from: usize, edge_index: usize, vm_graph: &StateGraph<'_>) -> StateLink {
        let edge = self.nodes[from].successors[edge_index];
        let vm = self.nodes[from].state.vm;
        if let Some(index) = edge.vm_edge {
            vm_graph.nodes[vm].successors[index].clone()
        } else {
            StateLink {
                to: vm,
                instructions: vec![],
                actions: vec![],
                lines: vec![],
                pid: 0,
                name: "_stutter_".to_string(),
            }
        }
    }

    /// A path inside one SCC, represented by (source, outgoing edge index).
    fn route(&self, from: usize, to: usize, in_component: &[bool]) -> Vec<(usize, usize)> {
        let mut predecessor = vec![None; self.nodes.len()];
        predecessor[from] = Some((from, usize::MAX));
        let mut queue = VecDeque::from([from]);
        while let Some(node) = queue.pop_front() {
            if node == to {
                break;
            }
            for (index, edge) in self.nodes[node].successors.iter().enumerate() {
                if in_component[edge.to] && predecessor[edge.to].is_none() {
                    predecessor[edge.to] = Some((node, index));
                    queue.push_back(edge.to);
                }
            }
        }
        let mut path = Vec::new();
        let mut current = to;
        while current != from {
            let (previous, edge) = predecessor[current].expect("SCC states are mutually reachable");
            path.push((previous, edge));
            current = previous;
        }
        path.reverse();
        path
    }

    fn witness(
        &self,
        component: &[usize],
        targets: &[usize],
        vm_graph: &StateGraph<'_>,
    ) -> (Vec<StateLink>, usize) {
        let start = targets[0];
        let mut prefix = Vec::new();
        let mut current = start;
        while let Some((previous, edge)) = self.nodes[current].predecessor {
            prefix.push(self.state_link(previous, edge, vm_graph));
            current = previous;
        }
        prefix.reverse();
        let cycle_start = prefix.len();

        let mut in_component = vec![false; self.nodes.len()];
        for &node in component {
            in_component[node] = true;
        }
        let mut cycle = Vec::new();
        current = start;
        for &target in targets.iter().skip(1).chain(std::iter::once(&start)) {
            cycle.extend(self.route(current, target, &in_component));
            current = target;
        }
        if cycle.is_empty() {
            // A single accepting state still requires a nonempty closed walk.
            let (index, edge) = self.nodes[start]
                .successors
                .iter()
                .enumerate()
                .find(|(_, edge)| in_component[edge.to])
                .expect("a cyclic component has an internal edge");
            cycle.push((start, index));
            cycle.extend(self.route(edge.to, start, &in_component));
        }
        prefix.extend(
            cycle
                .into_iter()
                .map(|(from, edge)| self.state_link(from, edge, vm_graph)),
        );
        (prefix, cycle_start)
    }
}

pub(super) fn check_formula(
    vm_graph: &StateGraph<'_>,
    formula: &CompiledLtlExpression,
    automaton: &BuchiAutomaton,
) -> AlthreadResult<Option<(Vec<StateLink>, usize)>> {
    let formulas = std::slice::from_ref(formula);
    let automatons = std::slice::from_ref(automaton);
    let initial = quantifier::initialize_monitoring(
        formulas,
        automatons,
        vm_graph.vm(vm_graph.initial_state),
    )?;
    let mut product = ProductGraph::default();
    for monitors in initial.initial_choices() {
        product.intern(
            ProductState {
                vm: vm_graph.initial_state,
                monitors,
            },
            None,
        );
    }

    // Newly interned nodes are appended once, so a cursor is also a BFS queue.
    let mut cursor = 0;
    while cursor < product.nodes.len() {
        let current = product.nodes[cursor].state.clone();
        let vm_node = &vm_graph.nodes[current.vm];
        let mut successors: Vec<_> = vm_node
            .successors
            .iter()
            .enumerate()
            .map(|(index, edge)| (edge.to, Some(index)))
            .collect();
        if vm_node.expanded && successors.is_empty() {
            successors.push((current.vm, None));
        }
        for (next_vm, vm_edge) in successors {
            for mut monitors in current
                .monitors
                .get_possible_successors(vm_graph.vm(next_vm), automatons)?
            {
                // Advance existing instances first. Newly spawned instances
                // consume this observation only when their initials are tested.
                quantifier::update_monitors_for_new_processes(
                    formulas,
                    automatons,
                    &mut monitors,
                    vm_graph.vm(current.vm),
                    vm_graph.vm(next_vm),
                )?;
                for monitors in monitors.initial_choices() {
                    let edge_index = product.nodes[cursor].successors.len();
                    let to = product.intern(
                        ProductState {
                            vm: next_vm,
                            monitors,
                        },
                        Some((cursor, edge_index)),
                    );
                    product.nodes[cursor]
                        .successors
                        .push(ProductEdge { to, vm_edge });
                }
            }
        }
        cursor += 1;
    }

    for component in product.components() {
        let root = component[0];
        let cyclic = component.len() > 1
            || product.nodes[root]
                .successors
                .iter()
                .any(|edge| edge.to == root);
        if cyclic {
            if let Some(targets) = product.acceptance_targets(
                &component,
                automaton,
                matches!(formula, CompiledLtlExpression::Exists { .. }),
            ) {
                return Ok(Some(product.witness(&component, &targets, vm_graph)));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checker::ltl::{automaton::AutomatonState, monitor::LtlMonitor};

    fn automaton() -> BuchiAutomaton {
        BuchiAutomaton {
            states: (0..2)
                .map(|id| AutomatonState {
                    id,
                    formulas: vec![],
                    transitions: vec![1 - id],
                    acceptance_sets: vec![id],
                })
                .collect(),
            initial_states: vec![0],
            num_acceptance_sets: 2,
            until_constraints: vec![],
        }
    }

    fn state(ids: &[usize]) -> ProductState {
        ProductState {
            vm: 0,
            monitors: MonitoringState {
                monitors_per_formula: vec![ids
                    .iter()
                    .map(|&id| LtlMonitor::new(id, HashMap::new()))
                    .collect()],
            },
        }
    }

    #[test]
    fn acceptance_sets_can_be_covered_at_different_states() {
        let mut graph = ProductGraph::default();
        graph.intern(state(&[0]), None);
        graph.intern(state(&[1]), None);
        let targets = graph
            .acceptance_targets(&[0, 1], &automaton(), false)
            .unwrap();
        assert!(targets.contains(&0) && targets.contains(&1));
        assert!(graph
            .acceptance_targets(&[0], &automaton(), false)
            .is_none());
    }

    #[test]
    fn acceptance_sets_cannot_be_combined_across_instances() {
        let mut graph = ProductGraph::default();
        graph.intern(state(&[0, 1]), None);
        assert!(graph
            .acceptance_targets(&[0], &automaton(), false)
            .is_none());
        assert!(graph.acceptance_targets(&[0], &automaton(), true).is_none());
    }

    #[test]
    fn existential_instances_need_not_accept_simultaneously() {
        let mut graph = ProductGraph::default();
        graph.intern(state(&[0, 1]), None);
        graph.intern(state(&[1, 0]), None);
        let targets = graph
            .acceptance_targets(&[0, 1], &automaton(), true)
            .unwrap();
        assert!(targets.contains(&0) && targets.contains(&1));
    }
}
