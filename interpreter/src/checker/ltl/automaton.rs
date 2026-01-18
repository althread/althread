use std::collections::VecDeque;

use super::compiled::CompiledLtlExpression;

#[derive(Debug, Clone)]
pub struct AutomatonState {
    pub id: usize,
    /// Formulas that must hold in this state (saturated set)
    pub formulas: Vec<CompiledLtlExpression>,
    /// Transition to other state IDs
    pub transitions: Vec<usize>,
    /// Indices of acceptance sets this state belongs to (for GBA)
    pub acceptance_sets: Vec<usize>,
}

impl AutomatonState {
    pub fn is_accepting(&self, set_index: usize) -> bool {
        self.acceptance_sets.contains(&set_index)
    }
    
    /// Returns true if the state requires specific atomic propositions that match the description.
    /// This is used for debugging.
    pub fn description(&self) -> String {
        let props: Vec<String> = self.formulas.iter().filter_map(|f| match f {
            CompiledLtlExpression::Predicate { read_variables, .. } => Some(format!("Pred{:?}", read_variables)),
            CompiledLtlExpression::Not(b) => match **b {
                 CompiledLtlExpression::Predicate { ref read_variables, .. } => Some(format!("!Pred{:?}", read_variables)),
                 _ => None
            },
           _ => None
        }).collect();
        format!("State {}: [{}]", self.id, props.join(", "))
    }
}

#[derive(Debug, Clone)]
pub struct BuchiAutomaton {
    pub states: Vec<AutomatonState>,
    pub initial_states: Vec<usize>,
    pub num_acceptance_sets: usize,
    /// The Untils corresponding to each acceptance set index
    pub until_constraints: Vec<CompiledLtlExpression>,
}

impl BuchiAutomaton {
    pub fn new(expression: CompiledLtlExpression) -> Self {
        // 1. Negate the expression to find counter-examples
        let neg_expr = expression.negate();
        
        // 2. Identify all Until subformulas to define acceptance sets
        let mut until_constraints = Vec::new();
        collect_untils(&neg_expr, &mut until_constraints);
        
        // 3. GBA Construction
        let mut states: Vec<AutomatonState> = Vec::new();
        // Determine initial states by expanding the start formula
        let init_nodes = expand(vec![neg_expr]);
        
        // We need to explore reachable states
        let mut known_nodes: Vec<Vec<CompiledLtlExpression>> = Vec::new();
        let mut queue: VecDeque<usize> = VecDeque::new();
        let mut initial_ids = Vec::new();

        // Register initial nodes
        for node in init_nodes {
            if let Some(id) = find_node_id(&known_nodes, &node) {
                if !initial_ids.contains(&id) { initial_ids.push(id); }
            } else {
                let id = known_nodes.len();
                known_nodes.push(node.clone());
                initial_ids.push(id);
                queue.push_back(id);
                
                // Create skeleton state
                states.push(AutomatonState {
                    id,
                    formulas: node, // placeholder
                    transitions: Vec::new(),
                    acceptance_sets: Vec::new(),
                });
            }
        }
        
        // Explore
        while let Some(current_id) = queue.pop_front() {
            let current_formulas = states[current_id].formulas.clone();
            
            // Compute acceptance sets for this state
            let mut acc_sets = Vec::new();
            for (i, until_expr) in until_constraints.iter().enumerate() {
                if let CompiledLtlExpression::Until(_, right) = until_expr {
                    let has_until = contains_formula(&current_formulas, until_expr);
                    let has_rhs = contains_formula(&current_formulas, right);
                    
                    if !has_until || has_rhs {
                        acc_sets.push(i);
                    }
                }
            }
            states[current_id].acceptance_sets = acc_sets;
            
            // Compute Transitions
            // Get all 'Next' obligations
            let mut next_obligations = Vec::new();
            for f in &current_formulas {
                if let CompiledLtlExpression::Next(inner) = f {
                    next_obligations.push(*inner.clone());
                }
            }
            
            let target_nodes = expand(next_obligations);
            
            for target in target_nodes {
                let target_id;
                if let Some(id) = find_node_id(&known_nodes, &target) {
                    target_id = id;
                } else {
                    target_id = known_nodes.len();
                    known_nodes.push(target.clone());
                    queue.push_back(target_id);
                    states.push(AutomatonState {
                        id: target_id,
                        formulas: target.clone(),
                        transitions: Vec::new(),
                        acceptance_sets: Vec::new(),
                    });
                }
                if !states[current_id].transitions.contains(&target_id) {
                    states[current_id].transitions.push(target_id);
                }
            }
        }
        
        Self {
            states,
            initial_states: initial_ids,
            num_acceptance_sets: until_constraints.len(),
            until_constraints,
        }
    }
}

// Helpers

fn collect_untils(expr: &CompiledLtlExpression, acc: &mut Vec<CompiledLtlExpression>) {
    // If expr is Until, add it. Then recurse children.
    if let CompiledLtlExpression::Until(l, r) = expr {
        if !acc.contains(expr) {
            acc.push(expr.clone());
        }
        collect_untils(l, acc);
        collect_untils(r, acc);
        return;
    }
    
    // Recurse
    match expr {
        CompiledLtlExpression::Not(e) => collect_untils(e, acc),
        CompiledLtlExpression::Next(e) | CompiledLtlExpression::Always(e) | CompiledLtlExpression::Eventually(e) => collect_untils(e, acc),
        CompiledLtlExpression::And(a, b) | CompiledLtlExpression::Or(a, b) | CompiledLtlExpression::Implies(a, b) | CompiledLtlExpression::Release(a, b) => {
            collect_untils(a, acc);
            collect_untils(b, acc);
        }
        CompiledLtlExpression::ForLoop { body, .. } | CompiledLtlExpression::Exists { body, .. } => collect_untils(body, acc),
        _ => {}
    }
}

fn contains_formula(list: &[CompiledLtlExpression], f: &CompiledLtlExpression) -> bool {
    list.iter().any(|item| item == f)
}

fn find_node_id(known: &[Vec<CompiledLtlExpression>], target: &[CompiledLtlExpression]) -> Option<usize> {
    for (i, k) in known.iter().enumerate() {
        if k.len() == target.len() && k.iter().all(|x| contains_formula(target, x)) && target.iter().all(|x| contains_formula(k, x)) {
            return Some(i);
        }
    }
    None
}

/// Expands a list of formulas into a set of saturated nodes (states).
fn expand(formulas: Vec<CompiledLtlExpression>) -> Vec<Vec<CompiledLtlExpression>> {
    let mut results = Vec::new();
    expand_recursive(formulas, &mut results);
    results
}

fn expand_recursive(current: Vec<CompiledLtlExpression>, results: &mut Vec<Vec<CompiledLtlExpression>>) {
    // Find the first non-literal formula
    let idx = current.iter().position(|f| !is_literal(f));
    
    match idx {
        None => {
            // All literals (or Next). Check consistency.
            if is_consistent(&current) {
                results.push(current);
            }
        }
        Some(i) => {
            let f = current[i].clone();
            // Remove f from list
            let mut remainder = current;
            remainder.remove(i);
            
            // Apply rules
            // Returns list of (new_formulas_to_add)
            let branches = apply_tableau_rule(&f);
            
            for branch in branches {
                let mut new_node = remainder.clone();
                // Add all formulas from branch
                for bf in branch {
                    if !contains_formula(&new_node, &bf) {
                        new_node.push(bf);
                    }
                }
                expand_recursive(new_node, results);
            }
        }
    }
}

fn is_literal(f: &CompiledLtlExpression) -> bool {
    match f {
        CompiledLtlExpression::Predicate { .. } => true,
        CompiledLtlExpression::Boolean(_) => true,
        // Loops are treated as literals if they appear during expansion (meaning they are nested booleans)
        CompiledLtlExpression::ForLoop { .. } => true, 
        CompiledLtlExpression::Exists { .. } => true,
        CompiledLtlExpression::Next(_) => true, // Next is treated as literal during state expansion
        CompiledLtlExpression::Not(inner) => matches!(**inner, 
            CompiledLtlExpression::Predicate { .. } | 
            CompiledLtlExpression::Boolean(_) | 
            CompiledLtlExpression::Next(_) |
            CompiledLtlExpression::ForLoop { .. } |
            CompiledLtlExpression::Exists { .. }
        ),
        _ => false,
    }
}

fn is_consistent(formulas: &[CompiledLtlExpression]) -> bool {
    for f in formulas {
        if matches!(f, CompiledLtlExpression::Boolean(false)) { return false; }
        if let CompiledLtlExpression::Not(inner) = f {
             if contains_formula(formulas, inner) { return false; }
             if matches!(**inner, CompiledLtlExpression::Boolean(true)) { return false; }
        }
    }
    true
}

fn apply_tableau_rule(f: &CompiledLtlExpression) -> Vec<Vec<CompiledLtlExpression>> {
    match f {
        CompiledLtlExpression::And(a, b) => {
            vec![vec![*a.clone(), *b.clone()]]
        }
        CompiledLtlExpression::Or(a, b) => {
            vec![vec![*a.clone()], vec![*b.clone()]]
        }
        CompiledLtlExpression::Until(a, b) => {
            // a U b <=> b \/ (a /\ X(a U b))
            // Branch 1: b
            // Branch 2: a, X(a U b)
            vec![
                vec![*b.clone()],
                vec![*a.clone(), CompiledLtlExpression::Next(Box::new(f.clone()))]
            ]
        }
        CompiledLtlExpression::Release(a, b) => {
            // a R b <=> b /\ (a \/ X(a R b))
            // <=> (b /\ a) \/ (b /\ X(a R b))
            // Branch 1: b, a
            // Branch 2: b, X(a R b)
            vec![
                vec![*b.clone(), *a.clone()],
                vec![*b.clone(), CompiledLtlExpression::Next(Box::new(f.clone()))]
            ]
        }
        CompiledLtlExpression::Eventually(a) => {
            // <> a <=> a \/ X <> a
            vec![
                vec![*a.clone()],
                vec![CompiledLtlExpression::Next(Box::new(f.clone()))]
            ]
        }
        CompiledLtlExpression::Always(a) => {
            // [] a <=> a /\ X [] a
            vec![
                vec![*a.clone(), CompiledLtlExpression::Next(Box::new(f.clone()))]
            ]
        }
        _ => vec![vec![f.clone()]] // Should not happen if expandable
    }
}
