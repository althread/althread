use std::{collections::{HashMap, HashSet}, fmt};

use fastrand::Rng;

use instruction::{ExpressionControl, GlobalReadsControl, Instruction, InstructionType, ProgramCode};

use crate::{ast::{statement::wait, token::{binary_assignment_operator::BinaryAssignmentOperator, literal::Literal}}, compiler::CompiledProject, error::{AlthreadError, AlthreadResult, ErrorType, Pos}};
pub mod instruction;




type Memory = Vec<Literal>;
type GlobalMemory = HashMap<String, Literal>;

#[derive(Debug)]
pub struct ExecutionStepInfo {
    pub prog_name: String,
    pub prog_id: usize,
    pub instruction_count: usize,
}

#[derive(Debug)]
pub struct RunningProgramState<'a> {
    name: String,
    memory: Memory,
    code: &'a ProgramCode,
    instruction_pointer: usize,
    pub id: usize,
}


fn str_to_expr_error(pos: Option<Pos>) -> impl Fn(String) -> AlthreadError {
    return move |msg| AlthreadError::new(
        ErrorType::ExpressionError,
        pos,
        msg
    )
}

pub enum GlobalAction {
    Nothing,
    Pause,
    StartProgram(String),
    Write(String),
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
            id,
        }
    }

    pub fn current_instruction(&self) -> Option<&Instruction> {
        self.code.instructions.get(self.instruction_pointer)
    }
    fn next_global(&mut self, globals: &mut GlobalMemory) -> AlthreadResult<(GlobalAction, usize)> {
        let mut n = 0;
        while true {
            let action = self.next(globals)?;
            n += 1;
            if !action.is_local() {
                return Ok((action, n));
            }
        }
        unreachable!()
    }

    fn next(&mut self, globals: &mut GlobalMemory) -> AlthreadResult<GlobalAction> {
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
                action = GlobalAction::StartProgram(call.name.clone());
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

                println!("{}", lit);
                1
            }
            InstructionType::Wait(wait_ctrl) => {
                let cond = self.memory.last().unwrap().is_true();
                for _ in 0..wait_ctrl.unstack_len { self.memory.pop(); }
                if cond {
                    1
                } else {
                    action = GlobalAction::Wait;
                    wait_ctrl.jump
                }
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
    pub running_programs: HashMap<usize, RunningProgramState<'a>>,
    pub programs_code: &'a HashMap<String, ProgramCode>,
    pub executable_programs: HashSet<usize>,
    pub always_conditions: &'a Vec<(HashSet<String>, GlobalReadsControl, ExpressionControl, Pos)>,

    /// The programs that are waiting for a condition to be true
    /// The condition depends on the global variables that are in the HashSet
    pub waiting_programs: HashMap<usize, HashSet<String>>,
    next_program_id: usize,
    rng: Rng,
    seed: u64,
}

impl<'a> VM<'a> {
    pub fn new(compiled_project: &'a CompiledProject) -> Self {
        Self {
            globals: compiled_project.global_memory.clone(),
            running_programs: HashMap::new(),
            executable_programs: HashSet::new(),
            programs_code: &compiled_project.programs_code,
            always_conditions: &compiled_project.always_conditions,
            next_program_id: 0,
            waiting_programs: HashMap::new(),
            rng: Rng::new(),
            seed: 0,
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
        self.run_program("main");
    }

    pub fn next(&mut self) -> AlthreadResult<ExecutionStepInfo> {
        
        let program = self.rng.choice(self.executable_programs.iter()).expect("call next but no program is executable");

        let program = self.running_programs.get_mut(program).expect("program is executable but not found in running programs");
        
        let mut exec_info = ExecutionStepInfo {
            prog_name: program.name.clone(),
            prog_id: program.id,
            instruction_count: 0
        };

        let (action, instruction_count) = program.next_global(&mut self.globals)?;
        match action {
            GlobalAction::Nothing => {unreachable!("next_global should not pause on a local instruction")}
            GlobalAction::Pause => {},
            GlobalAction::Write(var_name) => {
                //println!("program {} writes {}", program.id, var_name);
                // Check if the variable appears in the conditions of a waiting program
                self.waiting_programs.retain(|prog_id, dependencies| {
                    if dependencies.contains(&var_name) {
                        self.executable_programs.insert(*prog_id);
                        //println!("program {} is woken up", prog_id);
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
            GlobalAction::StartProgram(name) => {
                self.run_program(&name);
            }
            GlobalAction::EndProgram => {
                let remove_id = program.id;
                self.running_programs.remove(&remove_id);
                self.executable_programs.remove(&remove_id);
            }
            GlobalAction::Wait => {
                self.executable_programs.remove(&program.id);
                let mut dependencies = HashSet::new();
                match &program.current_instruction().expect("waiting on no instruction").control {
                    InstructionType::GlobalReads(global_read) => {
                        for var_name in global_read.variables.iter() {
                            dependencies.insert(var_name.clone());
                        }
                    }
                    _ => unreachable!("waiting on an instruction that is not a global read")
                }
                //println!("process {} is waiting for {:?}", program.id, dependencies);
                self.waiting_programs.insert(program.id, dependencies);
            }
            GlobalAction::Exit => {
                self.running_programs.clear()
            }
        }
        exec_info.instruction_count = instruction_count;


        
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
            writeln!(f,"  {}: {}", name, val);
        }
        writeln!(f,"'main' stack:")?;
        for val in self.running_programs.get(&0).expect("no program is not running, cannot print the VM").memory.iter() {
            writeln!(f," - {}", val)?;
        }
        Ok(())
    }
}