use std::{collections::{BTreeSet, HashMap, HashSet}, fmt};

use channels::{Channels, ReceiverInfo};
use fastrand::Rng;

use instruction::{ExpressionControl, GlobalReadsControl, Instruction, InstructionType, ProgramCode};
use running_program::RunningProgramState;

use crate::{ast::{statement::waiting_case::WaitDependency, token::literal::Literal}, compiler::CompiledProject, error::{AlthreadError, AlthreadResult, ErrorType, Pos}};


pub mod instruction;
pub mod channels;
pub mod running_program;


pub type Memory = Vec<Literal>;
pub type GlobalMemory = HashMap<String, Literal>;

#[derive(Debug)]
pub struct ExecutionStepInfo {
    pub prog_name: String,
    pub prog_id: usize,
    pub instructions: Vec<Instruction>,
}


fn str_to_expr_error(pos: Option<Pos>) -> impl Fn(String) -> AlthreadError {
    return move |msg| AlthreadError::new(
        ErrorType::ExpressionError,
        pos,
        msg
    )
}

#[derive(Debug, PartialEq)]
pub enum GlobalAction {
    StartProgram(String, usize),
    Write(String),
    Send(String, Option<ReceiverInfo>),
    Connect(usize,String),
    EndProgram,
    Wait,
    Exit,
}


pub struct GlobalActions {
    pub actions: Vec<GlobalAction>,
    pub wait: bool,
}


pub struct VM<'a> {
    pub globals: GlobalMemory,
    pub channels: Channels,
    pub running_programs: HashMap<usize, RunningProgramState<'a>>,
    pub programs_code: &'a HashMap<String, ProgramCode>,
    pub executable_programs: BTreeSet<usize>, // needs to be sorted to have a deterministic behavior
    pub always_conditions: &'a Vec<(HashSet<String>, GlobalReadsControl, ExpressionControl, Pos)>,

    /// The programs that are waiting for a condition to be true
    /// The condition depends on the global variables that are in the HashSet
    waiting_programs: HashMap<usize, WaitDependency>,
    next_program_id: usize,
    global_state_id: u64,
    rng: Rng,
}

impl<'a> VM<'a> {
    pub fn new(compiled_project: &'a CompiledProject) -> Self {
        Self {
            globals: compiled_project.global_memory.clone(),
            channels: Channels::new(),
            running_programs: HashMap::new(),
            executable_programs: BTreeSet::new(),
            programs_code: &compiled_project.programs_code,
            always_conditions: &compiled_project.always_conditions,
            next_program_id: 0,
            waiting_programs: HashMap::new(),
            rng: Rng::new(),
            global_state_id: 0,
        }
    }

    fn run_program(&mut self, program_name: &str, pid: usize) {
        assert!(self.running_programs.get(&pid).is_none(), "program with id {} already exists", pid);
        self.running_programs.insert(pid,
            RunningProgramState::new(
                pid,
                program_name.to_string(), 
                &self.programs_code[program_name]
            ));
        self.executable_programs.insert(pid);
    }

    pub fn start(&mut self, seed: u64) {
        self.rng = Rng::with_seed(seed);
        self.global_state_id = 0;
        self.next_program_id = 1;
        self.run_program("main", 0);
    }

    pub fn next(&mut self) -> AlthreadResult<ExecutionStepInfo> {
        if self.running_programs.len() == 0 {
            return Err(AlthreadError::new(
                ErrorType::RuntimeError,
                None,
                "no program is running".to_string()
            ));
        }

        let program = self.rng.choice(self.executable_programs.iter()).ok_or(AlthreadError::new(
            ErrorType::RuntimeError,
            None,
            format!("All programs are waiting, deadlock:\n{}", self.waiting_programs.iter().map(|(id, dep)| 
            format!("-{}#{} at line {}: {:?}", 
                self.running_programs.get(id).unwrap().name,
                id, 
                self.running_programs.get(id).unwrap().current_instruction().unwrap().pos.unwrap().line,
                dep)).collect::<Vec<_>>().join("\n"))
        ))?;
        
        let program = self.running_programs.get_mut(program).expect("program is executable but not found in running programs");
        let program_id = program.id;
        
        let mut exec_info = ExecutionStepInfo {
            prog_name: program.name.clone(),
            prog_id: program_id,
            instructions: Vec::new(),
        };

        let (actions, executed_instructions) = program.next_global(&mut self.globals, &mut self.channels, &mut self.next_program_id, self.global_state_id)?;


        for action in actions.actions {
            match action {
                GlobalAction::Wait => {
                    unreachable!("wait action should not be in the list of actions");
                }
                GlobalAction::Send(sender_channel, receiver_info) => {
                    self.global_state_id += 1;
                    if let Some(receiver_info) = receiver_info {
                        if let Some(dependency) = self.waiting_programs.get(&receiver_info.program_id) {
                            if dependency.channels_state.contains(&receiver_info.channel_name) {
                                self.waiting_programs.remove(&receiver_info.program_id);
                                self.executable_programs.insert(receiver_info.program_id);
                            }
                        }
                    } else {
                        // the current process is waiting
                        self.executable_programs.remove(&program_id);
                        let dep = self.waiting_programs.entry(program_id).or_insert(WaitDependency::new());
                        dep.channels_connection.insert(sender_channel);
                    }
                }
                GlobalAction::Connect(sender_id, sender_channel) => {
                    self.global_state_id += 1;
                    if let Some(dependency) = self.waiting_programs.get(&sender_id) {
                        if dependency.channels_connection.contains(&sender_channel) {
                            self.waiting_programs.remove(&sender_id);
                            self.executable_programs.insert(sender_id);
                        }
                        else {
                            unreachable!("the sender program must be waiting for a connection, otherwise the channel connection is not a global action");
                        }
                    } else {
                        unreachable!("the sender program must be waiting, otherwise the channel connection is not a global action");
                    }
                }
                GlobalAction::Write(var_name) => {
                    self.global_state_id += 1;
                    //println!("program {} writes {}", program_id, var_name);
                    // Check if the variable appears in the conditions of a waiting program
                    self.waiting_programs.retain(|prog_id, dependencies| {
                        if dependencies.variables.contains(&var_name) {
                            self.executable_programs.insert(*prog_id);
                            return false;
                        }
                        true
                    });

                    for (dependencies, read, expr, pos) in self.always_conditions.iter() {
                        if dependencies.contains(&var_name) {
                            // Check if the condition is true
                            // create a small memory stack with the value of the variables
                            let mut memory = Vec::new();
                            for var_name in read.variables.iter() {
                                memory.push(self.globals.get(var_name).expect(format!("global variable '{}' not found", var_name).as_str()).clone());
                            }
                            let cond = expr.root.eval(&memory).map_err(|msg| AlthreadError::new(
                                ErrorType::ExpressionError,
                                Some(*pos),
                                msg
                            ))?;
                            if !cond.is_true() {
                                return Err(AlthreadError::new(
                                    ErrorType::RuntimeError,
                                    Some(*pos),
                                    format!("the condition is false")
                                ));
                            }
                        }
                    }
                }
                GlobalAction::StartProgram(name, pid) => {
                    self.global_state_id += 1;
                    self.run_program(&name, pid);
                }
                GlobalAction::EndProgram => {
                    let remove_id = program_id;
                    self.running_programs.remove(&remove_id);
                    self.executable_programs.remove(&remove_id);
                }
                GlobalAction::Exit => {
                    self.running_programs.clear()
                }
            }
        }

        if actions.wait {
            let program = self.running_programs.get_mut(&program_id).expect("program is waiting but not found in running programs");
            match &program.current_instruction().expect("waiting on no instruction").control {
                InstructionType::WaitStart(ctrl) => {
                    self.executable_programs.remove(&program_id);
                    self.waiting_programs.insert(program_id, ctrl.dependencies.clone());
                }
                _ => unreachable!("waiting on an instruction that is not a WaitStart instruction")
            }
        }

        exec_info.instructions = executed_instructions;


        
        Ok(exec_info)
    }

    pub fn is_finished(&self) -> bool {
        self.running_programs.is_empty()
    }

    pub fn new_memory() -> Memory {
        Vec::<Literal>::new()
    }
}



impl<'a> fmt::Display for VM<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f,"Globals:")?;
        for (name, val) in self.globals.iter() {
            writeln!(f,"  {}: {}", name, val)?;
        }
        writeln!(f,"'main' stack:")?;
        /*for val in self.running_programs.get(&0).expect("no program is not running, cannot print the VM").memory.iter() {
            writeln!(f," - {}", val)?;
        }*/
        Ok(())
    }
}