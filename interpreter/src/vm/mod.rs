use core::panic;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet}, fmt, hash::{Hash, Hasher}, rc::Rc
};

use channels::{Channels, ChannelsState, ReceiverInfo};
use fastrand::Rng;

use instruction::{Instruction, InstructionType, ProgramCode};
use running_program::RunningProgramState;

use crate::{
    ast::{
        statement::{expression::LocalExpressionNode, waiting_case::WaitDependency},
        token::literal::Literal,
    },
    compiler::{stdlib::Stdlib, CompiledProject, FunctionDefinition},
    error::{AlthreadError, AlthreadResult, ErrorType, Pos},
};

pub mod channels;
pub mod instruction;
pub mod running_program;

pub type Memory = Vec<Literal>;
pub type GlobalMemory = BTreeMap<String, Literal>;

#[derive(Debug)]
pub struct ExecutionStepInfo {
    pub prog_name: String,
    pub prog_id: usize,
    pub instructions: Vec<Instruction>,
    pub invariant_error: AlthreadResult<i32>,
    pub actions: Vec<GlobalAction>,
}

fn str_to_expr_error(pos: Option<Pos>) -> impl Fn(String) -> AlthreadError {
    return move |msg| AlthreadError::new(ErrorType::ExpressionError, pos, msg);
}

#[derive(Debug, PartialEq, Clone)]
pub enum GlobalAction {
    StartProgram(String, usize, Literal),
    Print(String),
    Write(String),
    Send(String, Option<ReceiverInfo>),
    Connect(usize, String),
    EndProgram,
    Wait,
    Exit,
}

#[derive(Debug, PartialEq, Clone)]
pub struct GlobalActions {
    pub actions: Vec<GlobalAction>,
    pub wait: bool,
    pub end: bool,
}

#[derive(Debug, Clone)]
pub struct VM<'a> {
    pub globals: GlobalMemory,
    pub channels: Channels,
    pub running_programs: Vec<RunningProgramState<'a>>,
    pub programs_code: &'a HashMap<String, ProgramCode>,
    pub user_funcs: &'a HashMap<String, FunctionDefinition>,
    pub executable_programs: BTreeSet<usize>, // needs to be sorted to have a deterministic behavior
    pub always_conditions: &'a Vec<(HashSet<String>, Vec<String>, LocalExpressionNode, Pos)>,
    pub eventually_conditions : &'a Vec<(HashSet<String>, Vec<String>, LocalExpressionNode, Pos)>, // adding a eventually conditions structure

    /// The programs that are waiting for a condition to be true
    /// The condition depends on the global variables that are in the HashSet
    waiting_programs: HashMap<usize, WaitDependency>,
    next_program_id: usize,
    rng: Rng,

    pub stdlib: Rc<Stdlib>,
}

impl<'a> VM<'a> {
    pub fn new(compiled_project: &'a CompiledProject) -> Self {
        Self {
            globals: compiled_project.global_memory.clone(),
            channels: Channels::new(),
            running_programs: Vec::new(),
            executable_programs: BTreeSet::new(),
            programs_code: &compiled_project.programs_code,
            user_funcs: &compiled_project.user_functions,
            always_conditions: &compiled_project.always_conditions,
            eventually_conditions: &compiled_project.eventually_conditions,
            next_program_id: 0,
            waiting_programs: HashMap::new(),
            rng: Rng::new(),
            stdlib: compiled_project.stdlib.clone(),
        }
    }

    fn run_program(&mut self, program_name: &str, pid: usize, args: Literal) {
        assert!(
            self.running_programs.get(pid).is_none(),
            "program with id {} already exists",
            pid
        );

        self.running_programs.insert(
            pid,
            RunningProgramState::new(
                pid,
                program_name.to_string(),
                &self.programs_code[program_name],
                self.user_funcs,
                args,
                self.stdlib.clone(),
            ),
        );
        self.executable_programs.insert(pid);
    }

    pub fn start(&mut self, seed: u64) {
        self.rng = Rng::with_seed(seed);
        self.next_program_id = 1;
        self.run_program("main", 0, Literal::empty_tuple());
    }

    pub fn next_random(&mut self) -> AlthreadResult<ExecutionStepInfo> {
        let program =
            self.rng
                .choice(self.executable_programs.iter())
                .ok_or(AlthreadError::new(
                    ErrorType::RuntimeError,
                    None,
                    format!(
                        "All programs are waiting, deadlock:\n{}",
                        self.waiting_programs
                            .iter()
                            .map(|(id, dep)| format!(
                                "-{}#{} at line {}: {:?}",
                                self.running_programs.get(*id).unwrap().name,
                                id,
                                self.running_programs
                                    .get(*id)
                                    .unwrap()
                                    .current_instruction()
                                    .unwrap()
                                    .pos
                                    .unwrap()
                                    .line,
                                dep
                            ))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ),
                ))?;

        let program = self
            .running_programs
            .get_mut(*program)
            .expect("program is executable but not found in running programs");
        let program_id = program.id;
        
       
        let mut exec_info = ExecutionStepInfo {
            prog_name: program.name.clone(),
            prog_id: program_id,
            instructions: Vec::new(),
            actions: Vec::new(),
            invariant_error: Ok(0),
        };

        let (actions, executed_instructions) = program.next_global(
            &mut self.globals,
            &mut self.channels,
            &mut self.next_program_id,
        )?;
        // maybe should be replace to avoid recurrent calls
        if actions.wait {
            // actually nothing happened
            assert!(
                actions.actions.is_empty(),
                "a process returning await should means that no actions have been performed..."
            );

            let program = self
                .running_programs
                .get_mut(program_id)
                .expect("program is waiting but not found in running programs");
            match &program
                .current_instruction()
                .expect("waiting on no instruction")
                .control
            {
                InstructionType::WaitStart { dependencies, .. } => {
                    self.executable_programs.remove(&program_id);
                    self.waiting_programs
                        .insert(program_id, dependencies.clone());
                }
                _ => unreachable!("waiting on an instruction that is not a WaitStart instruction"),
            }
            return self.next_random();
        }

        let mut need_to_check_invariants = false;

        for action in actions.actions.iter() {
            match action {
                GlobalAction::Wait => {
                    unreachable!("await action should not be in the list of actions");
                }
                GlobalAction::Send(_sender_channel, receiver_info) => {
                    if let Some(receiver_info) = receiver_info {
                        if let Some(dependency) =
                            self.waiting_programs.get(&receiver_info.program_id)
                        {
                            if dependency
                                .channels_state
                                .contains(&receiver_info.channel_name)
                            {
                                self.waiting_programs.remove(&receiver_info.program_id);
                                self.executable_programs.insert(receiver_info.program_id);
                            }
                        }
                    } else {
                        // the current process is waiting but this  will be catched up by the await instruction
                    }
                }
                GlobalAction::Connect(sender_id, sender_channel) => {
                    if let Some(dependency) = self.waiting_programs.get(&sender_id) {
                        if dependency.channels_connection.contains(sender_channel) {
                            self.waiting_programs.remove(sender_id);
                            self.executable_programs.insert(sender_id.clone());
                        } else {
                            unreachable!("the sender program must be waiting for a connection, otherwise the channel connection is not a global action");
                        }
                    } else {
                        unreachable!("the sender program must be waiting, otherwise the channel connection is not a global action");
                    }
                }
                GlobalAction::Write(var_name) => {
                    //println!("program {} writes {}", program_id, var_name);
                    // Check if the variable appears in the conditions of a waiting program
                    self.waiting_programs.retain(|prog_id, dependencies| {
                        if dependencies.variables.contains(var_name) {
                            self.executable_programs.insert(*prog_id);
                            return false;
                        }
                        true
                    });

                    need_to_check_invariants = true;
                }
                GlobalAction::StartProgram(name, pid, args) => {
                    self.run_program(name, *pid, args.clone());
                }
                GlobalAction::EndProgram => {
                    panic!("EndProgram action should not be in the list of actions");
                }
                GlobalAction::Exit => self.running_programs.clear(),
                GlobalAction::Print(_) => {} // do nothing, this is just a print action
            }
        }
        if actions.end {
            let remove_id = program_id;
            self.executable_programs.remove(&remove_id);
            self.waiting_programs.remove(&remove_id);
        }

        // TODO this method should be modified so eventually violation generate an error, 
        // for example by having a encounterd eventually counter, if the final VM's counter is == 0 no block validated eventually and path is wrong
        if need_to_check_invariants {
            exec_info.invariant_error = self.check_invariants();
        }

        exec_info.instructions = executed_instructions;
        exec_info.actions = actions.actions;

        Ok(exec_info)
    }

    pub fn next_step_pid(&mut self, pid: usize) -> AlthreadResult<Option<ExecutionStepInfo>> {
        let program = self
            .running_programs
            .get_mut(pid)
            .expect("program is executable but not found in running programs");

        if program.has_terminated() {
            return Ok(None);
        }

        let mut exec_info = ExecutionStepInfo {
            prog_name: program.name.clone(),
            prog_id: pid,
            instructions: Vec::new(),
            actions: Vec::new(),
            invariant_error: Ok(0),
        };

        let (actions, executed_instructions) = program.next_global(
            &mut self.globals,
            &mut self.channels,
            &mut self.next_program_id,
        )?;
        // maybe should be replace to avoid recurrent calls
        if actions.wait {
            // actually nothing happened
            assert!(
                actions.actions.is_empty(),
                "a process returning await should means that no actions have been performed..."
            );

            let program = self
                .running_programs
                .get_mut(pid)
                .expect("program is waiting but not found in running programs");
            match &program
                .current_instruction()
                .expect("waiting on no instruction")
                .control
            {
                InstructionType::WaitStart { dependencies, .. } => {
                    self.executable_programs.remove(&pid);
                    self.waiting_programs.insert(pid, dependencies.clone());
                }
                _ => unreachable!("waiting on an instruction that is not a WaitStart instruction"),
            }
            return Ok(None);
        }

        for action in actions.actions {
            match action {
                GlobalAction::Wait => {
                    unreachable!("await action should not be in the list of actions");
                }
                GlobalAction::Send(_sender_channel, receiver_info) => {
                    if let Some(receiver_info) = receiver_info {
                        if let Some(dependency) =
                            self.waiting_programs.get(&receiver_info.program_id)
                        {
                            if dependency
                                .channels_state
                                .contains(&receiver_info.channel_name)
                            {
                                self.waiting_programs.remove(&receiver_info.program_id);
                                self.executable_programs.insert(receiver_info.program_id);
                            }
                        }
                    } else {
                        // the current process is waiting but this  will be catched up by the await instruction
                    }
                }
                GlobalAction::Connect(sender_id, sender_channel) => {
                    if let Some(dependency) = self.waiting_programs.get(&sender_id) {
                        if dependency.channels_connection.contains(&sender_channel) {
                            self.waiting_programs.remove(&sender_id);
                            self.executable_programs.insert(sender_id);
                        } else {
                            unreachable!("the sender program must be waiting for a connection, otherwise the channel connection is not a global action");
                        }
                    } else {
                        unreachable!("the sender program must be waiting, otherwise the channel connection is not a global action");
                    }
                }
                GlobalAction::Write(var_name) => {
                    //println!("program {} writes {}", program_id, var_name);
                    // Check if the variable appears in the conditions of a waiting program
                    self.waiting_programs.retain(|prog_id, dependencies| {
                        if dependencies.variables.contains(&var_name) {
                            self.executable_programs.insert(*prog_id);
                            return false;
                        }
                        true
                    });
                }
                GlobalAction::StartProgram(name, pid, args) => {
                    self.run_program(&name, pid, args);
                }
                GlobalAction::EndProgram => {
                    panic!("EndProgram action should not be in the list of actions");
                }
                GlobalAction::Exit => self.running_programs.clear(),
                GlobalAction::Print(_) => {} // do nothing, this is just a print action
            }
        }
        if actions.end {
            let remove_id = pid;
            self.executable_programs.remove(&remove_id);
            self.waiting_programs.remove(&remove_id);
        }

        exec_info.instructions = executed_instructions;

        Ok(Some(exec_info))
    }

    pub fn get_program(&self, pid: usize) -> &RunningProgramState {
        self.running_programs.get(pid).expect("program not found")
    }

    /**
     * List all the next possible state of the VM
     */
    pub fn next(&self) -> AlthreadResult<Vec<(String, usize, Vec<Instruction>, VM<'a>)>> {
        if self.running_programs.len() == 0 {
            return Ok(Vec::new());
        }

        let mut next_states = Vec::new();

        // for each non-waiting program, execute the next instruction and store the result
        for program_id in self.executable_programs.iter() {
            let program = self.running_programs.get(*program_id).unwrap();

            if self.waiting_programs.contains_key(&program.id) {
                continue;
            }

            let mut vm = self.clone();
            if let Some(result) = vm.next_step_pid(program.id)? {
                next_states.push((program.name.clone(), program.id, result.instructions, vm));
            }
        }

        Ok(next_states)
    }

    pub fn is_finished(&self) -> bool {
        self.executable_programs.is_empty()
    }

    pub fn new_memory() -> Memory {
        Vec::<Literal>::new()
    }

    pub fn current_state(&self) -> (&GlobalMemory, &ChannelsState, Vec<(&Vec<Literal>, usize)>) {
        let local_states = self
            .running_programs
            .iter()
            .map(|prog| prog.current_state())
            .collect();

        (&self.globals, self.channels.state(), local_states)
    }

    //42 this check invariants, actually it only check always digging in to either expand it to take in account eventually or do a special one for eventually
    // return OK(0) if only always is verified
    // return OK(1) if eventually is also true
    pub fn check_invariants(&self) -> AlthreadResult<i32> {
        for (_deps, read_vars, expr, pos) in self.always_conditions.iter() {
            //if _deps.contains(&var_name) { //TODO improve by checking if the variable is in the dependencies
            // Check if the condition is true
            // create a small memory stack with the value of the variables
            let mut memory = Vec::new();
            for var_name in read_vars.iter() {
                memory.push(
                    self.globals
                        .get(var_name)
                        .expect(format!("global variable '{}' not found", var_name).as_str())
                        .clone(),
                );
            }
            match expr.eval(&memory) {
                Ok(cond) => {
                    if !cond.is_true() {
                        return Err(AlthreadError::new(
                            ErrorType::InvariantError,
                            Some(*pos),
                            "The invariant is not respected".to_string(),
                        ));
                    }
                }
                Err(e) => {
                    return Err(AlthreadError::new(
                        ErrorType::ExpressionError,
                        Some(*pos),
                        e,
                    ));
                }
            }

            //}
        }

        // now checking eventually
        for (_deps, read_vars, expr, pos) in self.eventually_conditions.iter() {
            //if _deps.contains(&var_name) { //TODO improve by checking if the variable is in the dependencies
            // Check if the eventually condition is true
            // create a small memory stack with the value of the variables
            let mut memory = Vec::new();
            for var_name in read_vars.iter() {
                memory.push(
                    self.globals
                        .get(var_name)
                        .expect(format!("global variable '{}' not found", var_name).as_str())
                        .clone(),
                );
            }
            match expr.eval(&memory) {
                Ok(cond) => {
                    if !cond.is_true() {
                        return Ok(0); // eventually not checking on a specific state isn't an error
                    } 
                }
                Err(e) => {
                    return Err(AlthreadError::new(
                        ErrorType::ExpressionError,
                        Some(*pos),
                        e,
                    ));
                }
            }

            //}
        }
        Ok(1) // if the eventually is valid we say it in the return
    }
}

impl<'a> fmt::Display for VM<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Globals:")?;
        for (name, val) in self.globals.iter() {
            writeln!(f, "  {}: {}", name, val)?;
        }
        writeln!(f, "'main' stack:")?;
        /*for val in self.running_programs.get(&0).expect("no program is not running, cannot print the VM").memory.iter() {
            writeln!(f," - {}", val)?;
        }*/
        Ok(())
    }
}

impl<'a> Hash for VM<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.globals.hash(state);
        self.channels.state().hash(state);
        self.running_programs.hash(state);
    }

    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        for piece in data {
            piece.hash(state)
        }
    }
}

impl std::cmp::PartialEq for VM<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.globals == other.globals
            && self.channels.state() == other.channels.state()
            && self.running_programs == other.running_programs
            && self.programs_code == other.programs_code
    }
}

impl std::cmp::Eq for VM<'_> {}

impl<'a> Serialize for VM<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (globals, channels, locals) = self.current_state();
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("VM", 3)?;
        state.serialize_field("globals", globals)?;
        state.serialize_field("channels", channels)?;
        state.serialize_field("locals", &locals)?;
        state.end()
    }
}
