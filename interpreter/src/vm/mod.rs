use std::{collections::{BTreeSet, HashMap, HashSet}, fmt};

use channels::{Channels, ReceiverInfo};
use fastrand::Rng;

use instruction::{ExpressionControl, GlobalReadsControl, Instruction, InstructionType, ProgramCode};

use crate::{ast::{statement::waiting_case::WaitDependency, token::literal::Literal}, compiler::CompiledProject, error::{AlthreadError, AlthreadResult, ErrorType, Pos}};
pub mod instruction;

pub mod channels;


pub type Memory = Vec<Literal>;
pub type GlobalMemory = HashMap<String, Literal>;

#[derive(Debug)]
pub struct ExecutionStepInfo {
    pub prog_name: String,
    pub prog_id: usize,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug)]
pub struct RunningProgramState<'a> {
    name: String,
    memory: Memory,
    code: &'a ProgramCode,
    instruction_pointer: usize,
    
    /// keeps track of the global state when the waitstart instruction was executed to see if it has changed when the wait instruction is executed
    global_state_stack: Vec<u64>,
    pub id: usize,
}


fn str_to_expr_error(pos: Option<Pos>) -> impl Fn(String) -> AlthreadError {
    return move |msg| AlthreadError::new(
        ErrorType::ExpressionError,
        pos,
        msg
    )
}

#[derive(Debug)]
pub enum GlobalAction {
    Nothing,
    Pause,
    StartProgram(String, usize),
    Write(String),
    Send(String, Option<ReceiverInfo>),
    Connect(usize,String),
    EndProgram,
    Wait,
    Exit,
}
impl GlobalAction {
    pub fn is_local(&self) -> bool {
        match self {
            Self::Nothing => true,
            _ => false,
        }
    }
}

impl<'a> RunningProgramState<'a> {

    fn new(id: usize, name:String, code: &'a ProgramCode) -> Self {
        Self {
            memory: Vec::new(),
            code,
            instruction_pointer: 0,
            name,
            global_state_stack: Vec::new(),
            id,
        }
    }

    pub fn current_instruction(&self) -> Option<&Instruction> {
        self.code.instructions.get(self.instruction_pointer)
    }
    fn next_global(&mut self, globals: &mut GlobalMemory, channels: &mut Channels, next_pid: usize, global_state_id: u64) -> AlthreadResult<(GlobalAction, Vec<Instruction>)> {

        let mut instructions = Vec::new();
        loop {
            if let Some(inst) = self.current_instruction() { instructions.push(inst.clone()); }
            
            let action = self.next(globals, channels, next_pid, global_state_id)?;
            
            if !action.is_local() {
                return Ok((action, instructions));
            }
        }
    }

    fn next(&mut self, globals: &mut GlobalMemory, channels: &mut Channels, mut next_pid: usize, global_state_id: u64) -> AlthreadResult<GlobalAction> {
        let cur_inst = self.code.instructions.get(self.instruction_pointer).ok_or(AlthreadError::new(
            ErrorType::InstructionNotAllowed,
            None,
            "the current instruction pointer points to no instruction".to_string()
        ))?;

        //println!("{} running instruction {}", self.id, self.current_instruction().unwrap());

        let mut action = if cur_inst.control.is_local() {
            GlobalAction::Nothing
        } else {
            GlobalAction::Pause
        };
        let pos_inc = match &cur_inst.control {
            InstructionType::JumpIf(c) => {
                let cond = self.memory.last().unwrap().is_true();
                for _ in 0..c.unstack_len { self.memory.pop(); }
                if cond {
                    1
                } else {
                    c.jump_false
                }
            },
            InstructionType::Jump(c) => c.jump,
            InstructionType::Expression(exp) => {
                let lit = exp.root.eval(&mut self.memory).map_err(|msg| AlthreadError::new(
                    ErrorType::ExpressionError,
                    cur_inst.pos,
                    msg,
                ))?;
                self.memory.push(lit);
                1
            },
            InstructionType::GlobalReads(global_read) => {
                for var_name in global_read.variables.iter() {
                    self.memory.push(globals.get(var_name).expect(format!("global variable '{}' not found", var_name).as_str()).clone());
                };
                1
            },
            InstructionType::GlobalAssignment(global_asgm) => {
                let lit = self.memory.last().expect("Panic: stack is empty, cannot perform assignment").clone();
                for _ in 0..global_asgm.unstack_len { self.memory.pop(); }

                let lit = global_asgm.operator.apply(
                    &globals.get(&global_asgm.identifier).expect(format!("global variable '{}' not found", global_asgm.identifier).as_str()),
                    &lit)
                    .map_err(str_to_expr_error(cur_inst.pos))?;

                globals.insert(global_asgm.identifier.clone(), lit);
                action = GlobalAction::Write(global_asgm.identifier.clone());
                1
            },
            InstructionType::LocalAssignment(local_asgm) => {
                let lit = self.memory.last().expect("Panic: stack is empty, cannot perform assignment").clone();
                for _ in 0..local_asgm.unstack_len { self.memory.pop(); }
                
                let len = self.memory.len();

                self.memory[len - 1 - local_asgm.index] = local_asgm.operator.apply(
                    &self.memory[len - 1 - local_asgm.index], 
                    &lit)
                    .map_err(str_to_expr_error(cur_inst.pos))?;
                1
            },
            InstructionType::Unstack(unstack_ctrl) => {
                for _ in 0..unstack_ctrl.unstack_len { self.memory.pop(); }
                1
            },
            InstructionType::Declaration(dec) => {
                let lit = self.memory.last().expect("Panic: stack is empty, cannot perform declaration with value").clone();
                for _ in 0..dec.unstack_len { self.memory.pop(); }
                self.memory.push(lit);
                1
            },
            InstructionType::RunCall(call) => {
                self.memory.push(Literal::Process(call.name.clone(), next_pid));
                action = GlobalAction::StartProgram(call.name.clone(), next_pid);
                next_pid += 1; // TODO: will be used when several global actions are implemented
                1
            }
            InstructionType::EndProgram => {
                action = GlobalAction::EndProgram;
                1
            }
            InstructionType::Empty => 1,
            InstructionType::FnCall(f) => {
                // currently, only the print function is implemented
                if f.name != "print" {
                    panic!("implement a proper function call in the VM");
                }
                let lit = self.memory.last().expect("Panic: stack is empty, cannot perform function call").clone();
                for _ in 0..f.unstack_len { self.memory.pop(); }

                let str = lit.into_tuple().unwrap().iter().map(|lit| lit.to_string()).collect::<Vec<_>>().join(",");
                println!("{}", str);
                1
            }
            InstructionType::WaitStart(_) => { // this instruction is not executed, it is used to create a dependency in case of a wait
                self.global_state_stack.push(global_state_id);
                1
            }
            InstructionType::Wait(wait_ctrl) => {
                let cond = self.memory.last().unwrap().is_true();
                for _ in 0..wait_ctrl.unstack_len { self.memory.pop(); }
                if cond {
                    1
                } else {
                    if global_state_id == self.global_state_stack.pop().expect("Panic: global_state_stack is empty, cannot pop") {
                        action = GlobalAction::Wait;
                    }
                    // otherwise we do not wait since the state has changed
                    wait_ctrl.jump
                }
            }
            InstructionType::Destruct(_) => {
                // The values are in a tuple on the top of the stack
                let tuple = self.memory.pop().expect("Panic: stack is empty, cannot destruct").into_tuple().expect("Panic: cannot convert to tuple");
                for val in tuple.into_iter() {
                    self.memory.push(val);
                };
                1
            }
            InstructionType::Push(literal) => {
                self.memory.push(literal.clone());
                1
            }
            InstructionType::Send(send_ctrl) => {

                let value = self.memory.last().expect("Panic: stack is empty, cannot send").clone();
                
                for _ in 0..send_ctrl.unstack_len { self.memory.pop(); }
                
                let receiver = channels.send(self.id, send_ctrl.channel_name.clone(), value);

                action = GlobalAction::Send(send_ctrl.channel_name.clone(), receiver);
                1
            }
            InstructionType::ChannelPeek(channel_name) => {
                let values = channels.peek(self.id, channel_name.clone());
                match values {
                    Some(value) => {
                        self.memory.push(value.clone());
                        self.memory.push(Literal::Bool(true));
                    }
                    None => {
                        self.memory.push(Literal::Bool(false));
                    }
                }
                1
            }
            InstructionType::ChannelPop(channel_name) => {
                let _ = channels.pop(self.id, channel_name.clone());
                1
            }
            InstructionType::Connect(connect_ctrl) => {
                let sender_pid = match connect_ctrl.sender_idx {
                    None => self.id,
                    Some(idx) => self.memory.get(self.memory.len() - 1 - idx).expect("Panic: stack is empty, cannot connect").clone().to_pid().expect("Panic: cannot convert to pid")
                };
                let receiver_pid = match connect_ctrl.receiver_idx {
                    None => self.id,
                    Some(idx) => self.memory.get(self.memory.len() - 1 - idx).expect("Panic: stack is empty, cannot connect").clone().to_pid().expect("Panic: cannot convert to pid")
                };

                let sender_info = channels.connect(sender_pid, connect_ctrl.sender_channel.clone(), receiver_pid, connect_ctrl.receiver_channel.clone()).map_err(|msg| AlthreadError::new(
                    ErrorType::RuntimeError,
                    cur_inst.pos,
                    msg
                ))?;
                if let Some(sender_info) = sender_info {
                    action = GlobalAction::Connect(sender_info.0, sender_info.1);
                }
                1
            }
            _ => panic!("Instruction '{:?}' not implemented", cur_inst.control),
        };
        let new_pos = (self.instruction_pointer as i64) + pos_inc;
        if new_pos < 0 {
            return Err(AlthreadError::new(
                ErrorType::RuntimeError, 
                None,
                "instruction pointer is becomming negative".to_string()))
        }
        self.instruction_pointer = new_pos as usize;
        Ok(action)
    }
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

    fn run_program(&mut self, program_name: &str) {
        self.running_programs.insert(self.next_program_id,
            RunningProgramState::new(
                self.next_program_id,
                program_name.to_string(), 
                &self.programs_code[program_name]
            ));
        self.executable_programs.insert(self.next_program_id);
        self.next_program_id += 1;
    }

    pub fn start(&mut self, seed: u64) {
        self.rng = Rng::with_seed(seed);
        self.global_state_id = 0;
        self.run_program("main");
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
        
        let mut exec_info = ExecutionStepInfo {
            prog_name: program.name.clone(),
            prog_id: program.id,
            instructions: Vec::new(),
        };

        let (action, executed_instructions) = program.next_global(&mut self.globals, &mut self.channels, self.next_program_id, self.global_state_id)?;

        match action {
            GlobalAction::Nothing => {unreachable!("next_global should not pause on a local instruction")}
            GlobalAction::Pause => {},
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
                    self.executable_programs.remove(&program.id);
                    let dep = self.waiting_programs.entry(program.id).or_insert(WaitDependency::new());
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
                //println!("program {} writes {}", program.id, var_name);
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
            GlobalAction::StartProgram(name, next_pid) => {
                self.global_state_id += 1;
                assert!(next_pid == self.next_program_id);
                self.run_program(&name);
            }
            GlobalAction::EndProgram => {
                let remove_id = program.id;
                self.running_programs.remove(&remove_id);
                self.executable_programs.remove(&remove_id);
            }
            GlobalAction::Wait => {
                match &program.current_instruction().expect("waiting on no instruction").control {
                    InstructionType::WaitStart(ctrl) => {
                        self.executable_programs.remove(&program.id);
                        self.waiting_programs.insert(program.id, ctrl.dependencies.clone());
                    }
                    _ => unreachable!("waiting on an instruction that is not a WaitStart instruction")
                }
            }
            GlobalAction::Exit => {
                self.running_programs.clear()
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
        for val in self.running_programs.get(&0).expect("no program is not running, cannot print the VM").memory.iter() {
            writeln!(f," - {}", val)?;
        }
        Ok(())
    }
}