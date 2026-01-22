//! Büchi automaton construction from LTL formulas.
//!
//! This module implements the tableau-based construction of Generalized Büchi Automatons (GBA)
//! from LTL formulas. The automaton accepts exactly the infinite words that satisfy the
//! negation of the input formula, which is used for counter-example detection.
//!
//! # Algorithm Overview
//!
//! 1. Negate the input formula (to search for violating traces)
//! 2. Identify Until subformulas to create acceptance sets
//! 3. Use tableau expansion to construct states (saturated formula sets)
//! 4. Build transitions based on Next-step obligations
//! 5. Mark accepting states based on Until progress requirements

use log::debug;
use std::collections::VecDeque;

use super::compiled::CompiledLtlExpression;

/// A state in the Büchi automaton.
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
        let props: Vec<String> = self
            .formulas
            .iter()
            .filter_map(|f| match f {
                CompiledLtlExpression::Predicate { read_variables, .. } => {
                    Some(format!("Pred{:?}", read_variables))
                }
                CompiledLtlExpression::Not(b) => match **b {
                    CompiledLtlExpression::Predicate {
                        ref read_variables, ..
                    } => Some(format!("!Pred{:?}", read_variables)),
                    _ => None,
                },
                _ => None,
            })
            .collect();
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
        
        log::debug!("Until constraints for automaton:");
        for (i, uc) in until_constraints.iter().enumerate() {
            log::debug!("  [{}]: {:?}", i, uc);
        }

        // 3. GBA Construction
        let mut states: Vec<AutomatonState> = Vec::new();
        // Determine initial states by expanding the start formula
        let init_nodes = expand(vec![neg_expr.clone()]);

        // We need to explore reachable states
        let mut known_nodes: Vec<Vec<CompiledLtlExpression>> = Vec::new();
        let mut queue: VecDeque<usize> = VecDeque::new();
        let mut initial_ids = Vec::new();

        // Register initial nodes
        for node in init_nodes {
            if let Some(id) = find_node_id(&known_nodes, &node) {
                if !initial_ids.contains(&id) {
                    initial_ids.push(id);
                }
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
        debug!("Initial Büchi states: {:?}", initial_ids);

        // Explore
        while let Some(current_id) = queue.pop_front() {
            let current_formulas = states[current_id].formulas.clone();

            // Compute acceptance sets for this state
            // 
            // For GBA acceptance, a state is accepting for an Until(A, B) constraint if:
            // 1. The right side B is satisfied in this state, OR
            // 2. The state is a "continuation" state that maintains a temporal property from B
            //    (e.g., if B = Request ∧ □¬Granted, and we're in a state maintaining □¬Granted)
            //
            // For case 2: if the state contains Next(Release(...)) from the Until's right side,
            // and the atomic part of the Until's right side was satisfied when entering this region,
            // then this state should be accepting.
            let mut acc_sets = Vec::new();
            for (i, until_expr) in until_constraints.iter().enumerate() {
                let is_accepting = match until_expr {
                    CompiledLtlExpression::Until(_, right) => {
                        // Standard check: right side is satisfied
                        if check_satisfaction(right, &current_formulas) {
                            true
                        } else {
                            // Additional check: if we're maintaining a Release obligation
                            // that comes from the right side of this Until, we're in an
                            // accepting region (the Until has been "satisfied" and we're
                            // in the continuation).
                            check_release_continuation(right, &current_formulas)
                        }
                    },
                    CompiledLtlExpression::Eventually(inner) => check_satisfaction(inner, &current_formulas),
                    // Note: Release is not collected as an until_constraint anymore,
                    // so this branch should never be reached
                    _ => false,
                };
                
                if is_accepting {
                    acc_sets.push(i);
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

            debug!(
                "Automaton {:?} state {} has formulas {:?} and transitions to {:?}",
                neg_expr, current_id, states[current_id].formulas, target_nodes
            );

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
    // If expr is Until or Eventually, add it. Then recurse children.
    match expr {
        CompiledLtlExpression::Until(l, r) => {
           if !acc.contains(expr) {
                acc.push(expr.clone());
            }
            collect_untils(l, acc);
            collect_untils(r, acc);
        }
        CompiledLtlExpression::Eventually(e) => {
            if !acc.contains(expr) {
                acc.push(expr.clone());
            }
            collect_untils(e, acc);
        }
        CompiledLtlExpression::Release(l, r) => {
            // Release does NOT generate an acceptance set in GBA.
            // Only Until/Eventually generate acceptance conditions.
            // Release(A, B) = ¬(¬A U ¬B) is a safety property when A=false.
            // We still need to recurse into the subformulas.
            collect_untils(l, acc);
            collect_untils(r, acc);
        }
        CompiledLtlExpression::Not(e) => collect_untils(e, acc),
        CompiledLtlExpression::Next(e)
        | CompiledLtlExpression::Always(e) => collect_untils(e, acc),
        CompiledLtlExpression::And(a, b)
        | CompiledLtlExpression::Or(a, b)
        | CompiledLtlExpression::Implies(a, b) => {
            collect_untils(a, acc);
            collect_untils(b, acc);
        }
        CompiledLtlExpression::ForLoop { body, .. }
        | CompiledLtlExpression::Exists { body, .. } => collect_untils(body, acc),
        _ => {}
    }
}

/// Check if the current state is a "continuation" of a Release obligation from the Until's right side.
/// This is used for acceptance: if the Until's right side contains a Release (like □¬Granted),
/// and we're in a state that maintains that Release (has Next(Release(...)) in its formulas),
/// then we're in an accepting region.
/// 
/// For example, for Until(true, Request ∧ Release(false, ¬Granted)):
/// - State 0: [Request, ¬Granted, Next(Release(false, ¬Granted))] is accepting (full satisfaction)
/// - State 2: [¬Granted, Next(Release(false, ¬Granted))] is also accepting (continuation)
fn check_release_continuation(right: &CompiledLtlExpression, formulas: &[CompiledLtlExpression]) -> bool {
    // Extract Release subformulas from the right side of the Until
    let releases = extract_releases(right);
    
    for release in releases {
        // Check if this Release is being maintained in the current state
        // A Release(A, B) is maintained if:
        // 1. B is satisfied in the current state
        // 2. Next(Release(A, B)) is in the formulas
        if let CompiledLtlExpression::Release(_, b) = &release {
            let next_release = CompiledLtlExpression::Next(Box::new(release.clone()));
            if check_satisfaction(b, formulas) && contains_formula(formulas, &next_release) {
                return true;
            }
        }
    }
    
    false
}

/// Extract all Release subformulas from an expression
fn extract_releases(expr: &CompiledLtlExpression) -> Vec<CompiledLtlExpression> {
    let mut releases = Vec::new();
    extract_releases_recursive(expr, &mut releases);
    releases
}

fn extract_releases_recursive(expr: &CompiledLtlExpression, releases: &mut Vec<CompiledLtlExpression>) {
    match expr {
        CompiledLtlExpression::Release(_, _) => {
            releases.push(expr.clone());
        }
        CompiledLtlExpression::And(a, b)
        | CompiledLtlExpression::Or(a, b)
        | CompiledLtlExpression::Until(a, b)
        | CompiledLtlExpression::Implies(a, b) => {
            extract_releases_recursive(a, releases);
            extract_releases_recursive(b, releases);
        }
        CompiledLtlExpression::Not(inner)
        | CompiledLtlExpression::Next(inner)
        | CompiledLtlExpression::Eventually(inner)
        | CompiledLtlExpression::Always(inner) => {
            extract_releases_recursive(inner, releases);
        }
        _ => {}
    }
}

/// Checks if a formula is satisfied by a set of formulas (which represents a state).
/// This handles the case where the formula 'f' is composite (e.g., And(A, B))
/// and the state contains decomposed literals (A, B).
fn check_satisfaction(f: &CompiledLtlExpression, formulas: &[CompiledLtlExpression]) -> bool {
    // 1. If f is in the set, it's satisfied.
    if contains_formula(formulas, f) {
        return true;
    }

    // 2. If f is a literal (atomic from POV of state), and not in set (step 1), then it's false.
    if is_literal(f) {
        return false;
    }

    // 3. Structural recursion
    match f {
        CompiledLtlExpression::And(a, b) => check_satisfaction(a, formulas) && check_satisfaction(b, formulas),
        CompiledLtlExpression::Or(a, b) => check_satisfaction(a, formulas) || check_satisfaction(b, formulas),
        CompiledLtlExpression::Implies(a, b) => !check_satisfaction(a, formulas) || check_satisfaction(b, formulas),
        CompiledLtlExpression::Not(_) => {
             // For Not, apply negation and check that.
             check_satisfaction(&f.clone().negate(), formulas)
        }
        // Release(A, B) is satisfied if:
        // - B is satisfied AND (A is satisfied OR Next(Release(A, B)) is in the state)
        // For Release(false, B) = □B: B must be satisfied and ○(□B) must be in the state
        CompiledLtlExpression::Release(a, b) => {
            if !check_satisfaction(b, formulas) {
                return false;
            }
            // Either A is true (releasing) or we continue with Next(Release(A, B))
            let next_release = CompiledLtlExpression::Next(Box::new(f.clone()));
            check_satisfaction(a, formulas) || contains_formula(formulas, &next_release)
        }
        // Until(A, B) is satisfied if:
        // - B is satisfied, OR
        // - A is satisfied AND Next(Until(A, B)) is in the state
        CompiledLtlExpression::Until(a, b) => {
            if check_satisfaction(b, formulas) {
                return true;
            }
            let next_until = CompiledLtlExpression::Next(Box::new(f.clone()));
            check_satisfaction(a, formulas) && contains_formula(formulas, &next_until)
        }
        CompiledLtlExpression::Eventually(_) |
        CompiledLtlExpression::Always(_) |
        CompiledLtlExpression::Next(_) => {
             // These should be in the set if they are true
             contains_formula(formulas, f)
        }
        
        _ => false 
    }
}

fn contains_formula(list: &[CompiledLtlExpression], f: &CompiledLtlExpression) -> bool {
    list.iter().any(|item| item == f)
}

fn find_node_id(
    known: &[Vec<CompiledLtlExpression>],
    target: &[CompiledLtlExpression],
) -> Option<usize> {
    for (i, k) in known.iter().enumerate() {
        if k.len() == target.len()
            && k.iter().all(|x| contains_formula(target, x))
            && target.iter().all(|x| contains_formula(k, x))
        {
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

fn expand_recursive(
    current: Vec<CompiledLtlExpression>,
    results: &mut Vec<Vec<CompiledLtlExpression>>,
) {
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
        CompiledLtlExpression::Not(inner) => matches!(
            **inner,
            CompiledLtlExpression::Predicate { .. }
                | CompiledLtlExpression::Boolean(_)
                | CompiledLtlExpression::Next(_)
                | CompiledLtlExpression::ForLoop { .. }
                | CompiledLtlExpression::Exists { .. }
        ),
        _ => false,
    }
}

fn is_consistent(formulas: &[CompiledLtlExpression]) -> bool {
    for f in formulas {
        if matches!(f, CompiledLtlExpression::Boolean(false)) {
            return false;
        }
        if let CompiledLtlExpression::Not(inner) = f {
            if contains_formula(formulas, inner) {
                return false;
            }
            if matches!(**inner, CompiledLtlExpression::Boolean(true)) {
                return false;
            }
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
                vec![*a.clone(), CompiledLtlExpression::Next(Box::new(f.clone()))],
            ]
        }
        CompiledLtlExpression::Release(a, b) => {
            // a R b <=> b /\ (a \/ X(a R b))
            // <=> (b /\ a) \/ (b /\ X(a R b))
            // Branch 1: b, a
            // Branch 2: b, X(a R b)
            vec![
                vec![*b.clone(), *a.clone()],
                vec![*b.clone(), CompiledLtlExpression::Next(Box::new(f.clone()))],
            ]
        }
        CompiledLtlExpression::Eventually(a) => {
            // <> a <=> a \/ X <> a
            vec![
                vec![*a.clone()],
                vec![CompiledLtlExpression::Next(Box::new(f.clone()))],
            ]
        }
        CompiledLtlExpression::Always(a) => {
            // [] a <=> a /\ X [] a
            vec![vec![
                *a.clone(),
                CompiledLtlExpression::Next(Box::new(f.clone())),
            ]]
        }
        _ => vec![vec![f.clone()]], // Should not happen if expandable
    }
}
