use std::{collections::HashMap, fmt};

use fastrand::Rng;

use instruction::{Instruction, InstructionType, ProgramCode};

use crate::{ast::token::{binary_assignment_operator::BinaryAssignmentOperator, literal::Literal}, compiler::CompiledProject, error::{AlthreadError, AlthreadResult, ErrorType, Pos}};
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
    EndProgram,
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

                global_asgm.operator.apply(
                    &globals.get(&global_asgm.identifier).expect(format!("global variable '{}' not found", global_asgm.identifier).as_str()),
                    &lit)
                    .map_err(str_to_expr_error(cur_inst.pos))?;

                globals.insert(global_asgm.identifier.clone(), lit);
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
    pub running_programs: Vec<RunningProgramState<'a>>,
    pub programs_code: &'a HashMap<String, ProgramCode>,
    rng: Rng
}

impl<'a> VM<'a> {
    pub fn new(compiled_project: &'a CompiledProject) -> Self {
        Self {
            globals: compiled_project.global_memory.clone(),
            running_programs: Vec::new(),
            programs_code: &compiled_project.programs_code,
            rng: Rng::new(),
        }
    }

    fn run_program(&mut self, program_name: &str) {
        let new_id = match self.running_programs.last() {
            Some(p) => p.id + 1,
            None => 0
        };
        self.running_programs.push(RunningProgramState::new(
            new_id,
            program_name.to_string(), 
            &self.programs_code[program_name]
        ));
    }

    pub fn start(&mut self) {
        self.run_program("main");
    }

    pub fn next(&mut self) -> AlthreadResult<ExecutionStepInfo> {
        let program = self.rng.choice(self.running_programs.iter_mut()).expect("call next but no program is running");
        
        let mut exec_info = ExecutionStepInfo {
            prog_name: program.name.clone(),
            prog_id: program.id,
            instruction_count: 0
        };

        let (action, instruction_count) = program.next_global(&mut self.globals)?;
        match action {
            GlobalAction::Nothing => {unreachable!("next_global should not pause on a local instruction")}
            GlobalAction::Pause => {}
            GlobalAction::StartProgram(name) => {
                self.run_program(&name);
            }
            GlobalAction::EndProgram => {
                let remove_id = program.id;
                self.running_programs.retain(|f| f.id != remove_id)
            }
            GlobalAction::Exit => {
                self.running_programs.clear()
            }
        }
        exec_info.instruction_count = instruction_count;
        Ok(exec_info)
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
        for val in self.running_programs.get(0).expect("no program is not running, cannot print the VM").memory.iter() {
            writeln!(f," - {}", val)?;
        }
        Ok(())
    }
}