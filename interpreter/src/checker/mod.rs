use std::{cell::{Cell, OnceCell}, collections::{HashMap, HashSet}, path, rc::Rc};

use serde::ser::{Serialize, Serializer, SerializeStruct};

use crate::{compiler::{stdlib::Stdlib, CompiledProject}, error::AlthreadResult, vm::{instruction::Instruction, VM}};


#[derive(Debug, Clone)]
pub struct StateLink<'a> {
    pub instructions: Vec<Instruction>,
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
}

#[derive(Debug)]
pub struct StateGraph<'a> {
    nodes: HashMap<Rc<VM<'a>>, GraphNode<'a>>,
}

impl<'a> Serialize for StateLink<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("StateLink", 4)?;
        state.serialize_field("lines", &self.lines)?;
        state.serialize_field("pid", &self.pid)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("to", &self.to.as_ref())?;
        state.end()
    }
}
impl<'a> Serialize for GraphNode<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("GraphNode", 3)?;
        state.serialize_field("level", &self.level)?;
        let pred = if self.predecessor.is_some() {
            Some(self.predecessor.as_ref().unwrap().as_ref().clone())
        } else {
            None
        };
        state.serialize_field("predecessor", &pred)?;
        state.serialize_field("successors", &self.successors)?;
        state.end()
    }
}
impl<'a> Serialize for StateGraph<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("StateGraph", 1)?;
        state.serialize_field("nodes", &self.nodes.iter().map(|(key, node)| 
        (
            key.as_ref(), 
            node
        )).collect::<Vec<(&VM, &GraphNode)>>())?;
        state.end()
    }
}

impl<'a> GraphNode<'a> {
    pub fn new(predecessor: Option<Rc<VM<'a>>>, level:usize) -> Self {
        Self {
            level,
            predecessor,
            successors: Vec::new(),
        }
    }
}

/// Checks a given project, returning a path from an initial state to the first state that violates an invariant. (return an empty vector if no invariant is violated)
pub fn check_program<'a>(compiled_project: &'a CompiledProject) -> AlthreadResult<(Vec<StateLink>, StateGraph)> {

    let mut state_graph = StateGraph {
        nodes: HashMap::new(),
    };
    
    let mut initial_vm = VM::new(compiled_project);
    initial_vm.start(0);
    let initial_vm = Rc::new(initial_vm);

    state_graph.nodes.insert(initial_vm.clone(), GraphNode::new(None, 0));
    
    let mut next_nodes = Vec::new();
    next_nodes.push(initial_vm);

    while !next_nodes.is_empty() {
        let current_node = next_nodes.pop().unwrap();
        let current_level = state_graph.nodes.get_mut(&current_node).unwrap().level;
        let successors = current_node.next()?;

        for (name, pid, instructions, vm) in successors.into_iter() {
            let vm: Rc<VM<'_>> = Rc::new(vm);

            let mut lines :Vec<usize> = instructions.iter().map(|x| x.pos.unwrap_or_default().line).filter(|l| *l > 0).collect();
            lines.dedup();

            state_graph.nodes.get_mut(&current_node).unwrap().successors.push(StateLink {
                to: vm.clone(),
                lines,
                instructions,
                pid,
                name,
            });

            if !state_graph.nodes.contains_key(&vm.clone()) {
                state_graph.nodes.insert(vm.clone(), GraphNode::new(Some(current_node.clone()), current_level + 1));
                next_nodes.push(vm.clone());
            }
        }
        if current_node.check_invariants().is_err() {
            let mut path = Vec::new();
            let mut back_node = current_node.clone();

            while let Some(pred) = state_graph.nodes.get(&back_node).unwrap().predecessor.clone() {
                path.push(state_graph.nodes.get(&pred).unwrap().successors.iter().find(|x| x.to == back_node).unwrap().clone());
                back_node = pred;
            }

            return Ok((path.into_iter().rev().collect(), state_graph));
        }
    }

    Ok((vec![], state_graph))
}