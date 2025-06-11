use std::collections::{HashMap, HashSet};

use crate::ast::node::Node;
use crate::ast::statement::Statement;
use crate::ast::block::Block;
use crate::error::Pos;

pub struct CFGNode<'a> {
    pub id: usize,
    pub ast_node: Option<&'a Node<Statement>>,
    pub is_return: bool,
    pub predecessors: Vec<usize>,
    pub successors: Vec<usize>
}

pub struct ControlFlowGraph<'a> {
    pub nodes: HashMap<usize, CFGNode<'a>>,
    pub entry: usize,
    pub exit: usize,
}

impl<'a> ControlFlowGraph<'a> {
    pub fn display(&self) {
        println!("Control Flow Graph:");
        for node in self.nodes.values() {
            println!("Node ID: {}, Is Return: {}, Predecessors: {:?}, Successors: {:?}",
                node.id, node.is_return, node.predecessors, node.successors);
            // if let Some(ast_node) = node.ast_node {
            //     println!("  AST Node: {:?}", ast_node.value);
            // }
        }
        println!("Entry Node ID: {}, Exit Node ID: {}", self.entry, self.exit);
    }



    pub fn from_function(fn_body_node: &'a Node<Block>) -> Self {
        let mut nodes : HashMap<usize, CFGNode<'a>> = HashMap::new();
        let mut next_node_id_counter = 0;


        // create entry node
        let entry_node_id = next_node_id_counter;
        next_node_id_counter += 1;
        nodes.insert(
            entry_node_id,
            CFGNode {
                id: entry_node_id,
                ast_node: None,
                is_return: false,
                predecessors: Vec::new(),
                successors: Vec::new(),
            }
        );


        // create exit node
        let exit_node_id = next_node_id_counter;
        next_node_id_counter += 1;
        nodes.insert(
            exit_node_id,
            CFGNode {
                id: exit_node_id,
                ast_node: None,
                is_return: false,
                predecessors: Vec::new(),
                successors: Vec::new(),
            }
        );


        // println!("Building CFG for function with entry node ID: {}, exit node ID: {}", entry_node_id, exit_node_id);


        let (_first_actual_stmt_node_id, open_ends_from_body) =
            ControlFlowGraph::build_cfg_recursive(
                &fn_body_node.value.children,
                vec![entry_node_id],
                &mut nodes,
                &mut next_node_id_counter,
                exit_node_id,
            ); 

        // println!("CFG built with {} nodes", nodes.len());

        if fn_body_node.value.children.is_empty() {
            if let Some(entry_cfg_node) = nodes.get_mut(&entry_node_id) {
                if entry_cfg_node.successors.is_empty() {
                    entry_cfg_node.successors.push(exit_node_id);
                    if let Some(exit_cfg_node) = nodes.get_mut(&exit_node_id) {
                        exit_cfg_node.predecessors.push(entry_node_id);
                    }
                }
            }
        }

        for id_of_open_end_node in open_ends_from_body {
            let node = nodes.get_mut(&id_of_open_end_node).unwrap();

            if !node.is_return {
                if !node.successors.contains(&exit_node_id) {
                    node.successors.push(exit_node_id);
                    if let Some(exit_node) = nodes.get_mut(&exit_node_id) {
                        exit_node.predecessors.push(id_of_open_end_node);
                    }
                }
            }
        }



        ControlFlowGraph {
            nodes,
            entry: entry_node_id,
            exit: exit_node_id,
        }
    }

    fn build_cfg_recursive<'b>(
        stmts: &'b [Node<Statement>],
        mut current_preceding_cfg_node_ids: Vec<usize>,
        nodes: &mut HashMap<usize, CFGNode<'b>>,
        next_id_counter: &mut usize,
        function_exit_id: usize,
    ) -> (Option<usize>, Vec<usize>) {
        let mut first_cfg_node_in_this_sequence: Option<usize> = None;

        if stmts.is_empty() {
            return (None, current_preceding_cfg_node_ids);
        }

        for stmt_ast_node in stmts {
            let current_stmt_cfg_node_id = *next_id_counter;
            *next_id_counter += 1;

            let is_return_stmt = matches!(stmt_ast_node.value, Statement::FnReturn(_));
            nodes.insert(
                current_stmt_cfg_node_id,
                CFGNode {
                    id: current_stmt_cfg_node_id,
                    ast_node: Some(stmt_ast_node),
                    is_return: is_return_stmt,
                    predecessors: Vec::new(),
                    successors: Vec::new(),
                },
            );

            if first_cfg_node_in_this_sequence.is_none() {
                first_cfg_node_in_this_sequence = Some(current_stmt_cfg_node_id);
            }

            for pred_id in &current_preceding_cfg_node_ids {
                if let Some(pred_node) = nodes.get_mut(pred_id) {
                    pred_node.successors.push(current_stmt_cfg_node_id);
                }
                if let Some(current_node) = nodes.get_mut(&current_stmt_cfg_node_id) {
                    current_node.predecessors.push(*pred_id);
                }
            }

            current_preceding_cfg_node_ids = vec![current_stmt_cfg_node_id];

            match &stmt_ast_node.value {
                Statement::FnReturn(_) => {
                    if let Some(curr_node) = nodes.get_mut(&current_stmt_cfg_node_id) {
                        curr_node.successors.push(function_exit_id);
                    }
                    if let Some(exit_node) = nodes.get_mut(&function_exit_id) {
                        exit_node.predecessors.push(current_stmt_cfg_node_id);
                    }
                    current_preceding_cfg_node_ids.clear();
                }
                Statement::Atomic(atomic_node) => {
                    let inner_stmt_node_ref: &Node<Statement> = &atomic_node.value.statement.as_ref();

                    let (_inner_atomic_entry_id, inner_atomic_open_ends) =
                        ControlFlowGraph::build_cfg_recursive(
                            match &inner_stmt_node_ref.value {
                                Statement::Block(inner_block) => &inner_block.value.children,
                                _ => std::slice::from_ref(inner_stmt_node_ref),
                            },
                            vec![current_stmt_cfg_node_id],
                            nodes,
                            next_id_counter,
                            function_exit_id,
                        );
                    current_preceding_cfg_node_ids = inner_atomic_open_ends;
                }
                Statement::If(if_control_node) => {
                    current_preceding_cfg_node_ids.clear();

                    let then_block_stmts = &if_control_node.value.then_block.value.children;
                    let (_then_entry_id, then_open_ends) = 
                        ControlFlowGraph::build_cfg_recursive(
                            then_block_stmts,
                            vec![current_stmt_cfg_node_id],
                            nodes,
                            next_id_counter,
                            function_exit_id,
                        );
                    current_preceding_cfg_node_ids.extend(then_open_ends);

                    if let Some(else_block_node) = &if_control_node.value.else_block {
                        let else_block_stmts = &else_block_node.value.children;
                        let (_else_entry_id, else_open_ends) = 
                            ControlFlowGraph::build_cfg_recursive(
                                else_block_stmts,
                                vec![current_stmt_cfg_node_id],
                                nodes,
                                next_id_counter,
                                function_exit_id,
                            );
                        current_preceding_cfg_node_ids.extend(else_open_ends);
                    } else {
                        current_preceding_cfg_node_ids.push(current_stmt_cfg_node_id);
                    }
                }
                Statement::Block(inner_block_node) => {
                    let inner_stmts = &inner_block_node.value.children;
                    let (_inner_block_entry_id, inner_block_open_ends) = 
                        ControlFlowGraph::build_cfg_recursive(
                            inner_stmts,
                            vec![current_stmt_cfg_node_id],
                            nodes,
                            next_id_counter,
                            function_exit_id,
                        );
                    current_preceding_cfg_node_ids = inner_block_open_ends;
                }
                // todo!("Handle while, for, loop, etc. statements")
                _ => {
                }
            }
        }

        (first_cfg_node_in_this_sequence, current_preceding_cfg_node_ids)
    }

    /// Finds the first point in the CFG where a path might be missing a return.
    ///
    /// Args:
    ///   - `fn_body_pos_for_empty_case`: The `Pos` of the entire function block. This is used
    ///     if a non-void function is empty and thus trivially misses a return.
    ///
    /// Returns:
    ///   - `Some(Pos)`: The position of an AST node related to the first detected non-returning path.
    ///     This could be the last statement on such a path, or an 'if' statement where a branch
    ///     doesn't return, or `fn_body_pos_for_empty_case` if the function body is empty.
    ///   - `None`: If all paths are found to have an explicit return statement.
    pub fn find_first_missing_return_point(&self, fn_body_for_empty_case: Pos) -> Option<Pos> {
        let mut visited_on_current_path = HashSet::new();
        let mut globally_visited_tuples = HashSet::new();

        let mut stack = vec![(self.entry, false, None)];

        while let Some((current_node_id, path_had_return_before_current, last_ast_node_pos_on_path)) = stack.pop() {
            if !globally_visited_tuples.insert((current_node_id, path_had_return_before_current)) {
                continue; // already visited this node with this return state
            }

            if visited_on_current_path.contains(&current_node_id) {
                continue; // already visited this node in the current path
            }
            visited_on_current_path.insert(current_node_id);

            let current_cfg_node = self.nodes.get(&current_node_id).expect("CFG node ID should exist");
    
            let current_node_is_explicit_return = current_cfg_node.is_return;
            let path_has_return_up_to_and_including_current = path_had_return_before_current || current_node_is_explicit_return;

            let current_node_ast_pos = current_cfg_node.ast_node
                .map(|node| node.pos);

            let effective_pos_for_this_step = current_node_ast_pos.or(last_ast_node_pos_on_path);

            if current_node_id == self.exit {
                if !path_has_return_up_to_and_including_current {
                    visited_on_current_path.remove(&current_node_id);
                    return Some(effective_pos_for_this_step.unwrap_or(fn_body_for_empty_case));
                }

                visited_on_current_path.remove(&current_node_id);
                continue; // exit node reached, no need to continue
            }

            if current_cfg_node.successors.is_empty() {
                if !path_has_return_up_to_and_including_current {
                    visited_on_current_path.remove(&current_node_id);
                    return Some(effective_pos_for_this_step.unwrap_or(fn_body_for_empty_case));
                }

                visited_on_current_path.remove(&current_node_id);
                continue; // no successors, nothing to do
            }

            for &succesor_id in &current_cfg_node.successors {
                stack.push((
                    succesor_id,
                    path_has_return_up_to_and_including_current,
                    effective_pos_for_this_step,
                ))
            }

            visited_on_current_path.remove(&current_node_id);
        }
        None
    }
}

