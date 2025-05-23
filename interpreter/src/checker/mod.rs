//42 <- used to easily remove all comments made with this id
use std::{collections::HashMap, rc::Rc};

use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::{
    compiler::CompiledProject,
    error::{AlthreadError, AlthreadResult, ErrorType},
    vm::{instruction::Instruction, VM},
};

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
    pub eventually: bool,
}

#[derive(Debug)]
pub struct StateGraph<'a> {
    nodes: HashMap<Rc<VM<'a>>, GraphNode<'a>>,
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
        let mut state = serializer.serialize_struct("GraphNode", 4)?;
        state.serialize_field("level", &self.level)?;
        let pred = if self.predecessor.is_some() {
            Some(self.predecessor.as_ref().unwrap().as_ref().clone())
        } else {
            None
        };
        state.serialize_field("predecessor", &pred)?;
        state.serialize_field("successors", &self.successors)?;
        state.serialize_field("eventually",&self.eventually)?;
        state.end()
    }
}
impl<'a> Serialize for StateGraph<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("StateGraph", 1)?;
        state.serialize_field(
            "nodes",
            &self
                .nodes
                .iter()
                .map(|(key, node)| (key.as_ref(), node))
                .collect::<Vec<(&VM, &GraphNode)>>(),
        )?;
        state.end()
    }
}

impl<'a> GraphNode<'a> {
    pub fn new(predecessor: Option<Rc<VM<'a>>>, level: usize) -> Self {
        Self {
            level,
            predecessor,
            eventually: false,
            successors: Vec::new(), //42 c'est ça qui fait l'état vert alors qu'il est possiblement faux
        }
    }
}

/// Checks a given project, returning a path from an initial state to the first state that violates an invariant. (return an empty vector if no invariant is violated)
pub fn check_program<'a>(
    compiled_project: &'a CompiledProject,
) -> AlthreadResult<(Vec<StateLink<'a>>, StateGraph<'a>)> {
    let mut state_graph = StateGraph {
        nodes: HashMap::new(),
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
    let mut next_nodes = Vec::new();
    next_nodes.push(initial_vm.clone());

    //42 while the successor list isn't empty
    while !next_nodes.is_empty() {
        //42 we pick on the the next nodes and remove it from the vector
        let current_node = next_nodes.pop().unwrap();
        let current_level = state_graph.nodes.get_mut(&current_node).unwrap().level;
        //42 successors vector of current node
        let successors = current_node.next()?;

        //42 we go through all successors 
        for (name, pid, instructions, vm) in successors.into_iter() {
            let vm: Rc<VM<'_>> = Rc::new(vm);
            
            let mut lines: Vec<usize> = instructions
                .iter()
                .map(|x| x.pos.unwrap_or_default().line)
                .filter(|l| *l > 0)
                .collect();
            //42 remove all dupes 
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
                    pid,
                    name,
                });
            
            //42 if the graphnode resulting from a statelink transition don't yet exist, create it
            if !state_graph.nodes.contains_key(&vm.clone()) {
                state_graph.nodes.insert(
                    vm.clone(),
                    GraphNode::new(Some(current_node.clone()), current_level + 1),
                );
                next_nodes.push(vm.clone());
            }
        }

        //42 check invariants
        let check_ret = current_node.check_invariants();
        if check_ret.is_err() {
            let mut path = Vec::new();
            let mut back_node = current_node.clone();

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
        } else if check_ret.is_ok_and(|x| x == 1){
            state_graph
                .nodes
                .get_mut(&current_node)
                .unwrap()
                .eventually = true;
        }
    }

    //42 now checking eventually violations

    // path visit is used to keep track of the successors we've already checked
    let mut path_visit: Vec<usize> = Vec::new();
    let mut path = Vec::new();
    // if root node check eventually condition no path can exist
    if state_graph.nodes.get(&initial_vm).unwrap().eventually {
        return Ok((vec![],state_graph));
    }

    // retrieving the state Link of the initial VM
    path.push(initial_vm.clone());
    // no successors have yet been visited
    path_visit.push(0);


    while !path.is_empty()
    {
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
        if succ.is_empty() && visited_succ == 0
        {
            let ret  = vec_vm_to_stalin(path, &state_graph);

            match ret {
                Ok(vec) => {
                    return Ok((vec.into_iter().rev().collect(),state_graph));
                },
                Err(e) => return Err(AlthreadError::new(
                    ErrorType::ExpressionError,
                    None,
                    e.message,
                )),
            }
        }

        // we search an explorable path in the successors list
        let mut explorable_path = false;
        while !succ.is_empty() && !explorable_path
        {
            let curr_succ = succ.pop().unwrap();
            visited_succ += 1;

            // if the successor is already in the path we found an invalid execution path
            if path.iter().any(|x| x == &curr_succ.to)
            {
                let ret  = vec_vm_to_stalin(path, &state_graph);
                match ret {
                    Ok(vec) => return Ok((vec.into_iter().rev().collect(),state_graph)),
                    // safety purpose 
                    Err(e) => return Err(AlthreadError::new(
                        ErrorType::ExpressionError,
                        None,
                        e.message,
                    )),
                }
            }

            // we get the corresponding graphnode and check wheter he has the eventually flag or not
            let graph_node = state_graph
                .nodes
                .get(curr_succ.to.as_ref())
                .unwrap();
            if !graph_node.eventually
            {
                explorable_path = true;
                path.push(curr_succ.to.clone());
                // we update the number of visited successors of the current node
                path_visit.push(visited_succ);
                // we then init the number of visited successors from the new node in the path
                path_visit.push(0);
            }
            
        }
        // if no explorable path was found we condemn this node (it is a dead end)
        if !explorable_path
        {
            state_graph
                .nodes
                .get_mut(&curr_vm)
                .unwrap()
                .eventually = true;
            path.pop();
        }
    }
    Ok((vec![], state_graph))
}

pub fn vec_vm_to_stalin<'a>(
    mut vec_vm: Vec<Rc<VM>>,
    state_graph: &StateGraph<'a>,
) -> AlthreadResult<Vec<StateLink<'a>>> {
    let mut ret_path = Vec::new();
    let mut back_node = vec_vm.pop().unwrap();

    while let Some(pred) = vec_vm.pop()
    {
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