use crate::{
    ast::token::literal::Literal,
    error::{AlthreadError, AlthreadResult, ErrorType},
};

use super::{
    channels::Channels,
    instruction::{Instruction, InstructionType, ProgramCode},
    str_to_expr_error, GlobalAction, GlobalActions, GlobalMemory, Memory,
};

#[derive(Debug)]
pub struct RunningProgramState<'a> {
    pub name: String,

    memory: Memory,
    code: &'a ProgramCode,
    instruction_pointer: usize,
    /// keeps track of the global state when the waitstart instruction was executed to see if it has changed when the wait instruction is executed
    global_state_stack: Vec<u64>,
    pub id: usize,
}

impl<'a> RunningProgramState<'a> {
    pub fn new(id: usize, name: String, code: &'a ProgramCode) -> Self {
        Self {
            memory: Vec::new(),
            code,
            instruction_pointer: 0,
            name,
            global_state_stack: Vec::new(),
            id,
        }
    }

    pub fn current_instruction(&self) -> AlthreadResult<&Instruction> {
        self.code.instructions.get(self.instruction_pointer).ok_or(AlthreadError::new(
            ErrorType::InstructionNotAllowed,
            None,
            "the current instruction pointer points to no instruction".to_string(),
        ))
    }
    pub fn next_global(
        &mut self,
        globals: &mut GlobalMemory,
        channels: &mut Channels,
        next_pid: &mut usize,
        global_state_id: u64,
    ) -> AlthreadResult<(GlobalActions, Vec<Instruction>)> {
        let mut instructions = Vec::new();
        let mut actions = Vec::new();
        let mut wait = false;
        loop {

            let (at_actions, at_instructions)  = self.next_atomic(globals, channels, next_pid, global_state_id)?;

            actions.extend(at_actions.actions);
            instructions.extend(at_instructions);
            
            if at_actions.wait {
                break;
            }

            if self.is_next_instruction_global() {
                break;
            }
        }
        Ok((GlobalActions { actions, wait }, instructions))
    }

    pub fn is_next_instruction_global(&mut self) -> bool {
        self.current_instruction()
            .map_or(true, |inst| !inst.control.is_local())
    }

    pub fn next_atomic(
        &mut self,
        globals: &mut GlobalMemory,
        channels: &mut Channels,
        next_pid: &mut usize,
        global_state_id: u64,
    ) -> AlthreadResult<(GlobalActions, Vec<Instruction>)> {
        let mut instructions = Vec::new();
        
        let mut result = GlobalActions { 
            actions: Vec::new(),
            wait: false 
        };
        // if the next instruction is not the start of an atomic block, we execute the next instruction
        if !self.current_instruction()?.is_atomic_start() {
            instructions.push(self.current_instruction()?.clone());
            let action = self.next(globals, channels, next_pid, global_state_id)?;
            if let Some(action) = action {
                if action == GlobalAction::Wait {
                    result.wait = true;
                }
                result.actions.push(action);
            }
            return Ok((result, instructions));
        }
        // else we execute all the instructions until the end of the atomic block
        loop {
            
            instructions.push(self.current_instruction()?.clone());
            let action = self.next(globals, channels, next_pid, global_state_id)?;
            if let Some(action) = action {
                if action == GlobalAction::Wait {
                    result.wait = true;
                }
                result.actions.push(action);
            }
            if self.current_instruction()?.is_atomic_end() {
                break;
            }
        }
        Ok((result, instructions))
    }

    fn next(
        &mut self,
        globals: &mut GlobalMemory,
        channels: &mut Channels,
        next_pid: &mut usize,
        global_state_id: u64,
    ) -> AlthreadResult<Option<GlobalAction>> {
        let cur_inst =
            self.code
                .instructions
                .get(self.instruction_pointer)
                .ok_or(AlthreadError::new(
                    ErrorType::InstructionNotAllowed,
                    None,
                    "the current instruction pointer points to no instruction".to_string(),
                ))?;

        //println!("{} running instruction {}", self.id, self.current_instruction().unwrap());

        let mut action = None;

        let pos_inc = match &cur_inst.control {
            InstructionType::Empty => 1,
            InstructionType::AtomicStart => 1,
            InstructionType::AtomicEnd => 1,
            InstructionType::JumpIf(c) => {
                let cond = self.memory.last().unwrap().is_true();
                for _ in 0..c.unstack_len {
                    self.memory.pop();
                }
                if cond {
                    1
                } else {
                    c.jump_false
                }
            }
            InstructionType::Jump(c) => c.jump,
            InstructionType::Expression(exp) => {
                let lit = exp.root.eval(&mut self.memory).map_err(|msg| {
                    AlthreadError::new(ErrorType::ExpressionError, cur_inst.pos, msg)
                })?;
                self.memory.push(lit);
                1
            }
            InstructionType::GlobalReads(global_read) => {
                for var_name in global_read.variables.iter() {
                    self.memory.push(
                        globals
                            .get(var_name)
                            .expect(format!("global variable '{}' not found", var_name).as_str())
                            .clone(),
                    );
                }
                1
            }
            InstructionType::GlobalAssignment(global_asgm) => {
                let lit = self
                    .memory
                    .last()
                    .expect("Panic: stack is empty, cannot perform assignment")
                    .clone();
                for _ in 0..global_asgm.unstack_len {
                    self.memory.pop();
                }

                let lit = global_asgm
                    .operator
                    .apply(
                        &globals.get(&global_asgm.identifier).expect(
                            format!("global variable '{}' not found", global_asgm.identifier)
                                .as_str(),
                        ),
                        &lit,
                    )
                    .map_err(str_to_expr_error(cur_inst.pos))?;

                globals.insert(global_asgm.identifier.clone(), lit);
                action = Some(GlobalAction::Write(global_asgm.identifier.clone()));
                1
            }
            InstructionType::LocalAssignment(local_asgm) => {
                let lit = self
                    .memory
                    .last()
                    .expect("Panic: stack is empty, cannot perform assignment")
                    .clone();
                for _ in 0..local_asgm.unstack_len {
                    self.memory.pop();
                }

                let len = self.memory.len();

                self.memory[len - 1 - local_asgm.index] = local_asgm
                    .operator
                    .apply(&self.memory[len - 1 - local_asgm.index], &lit)
                    .map_err(str_to_expr_error(cur_inst.pos))?;
                1
            }
            InstructionType::Unstack(unstack_ctrl) => {
                for _ in 0..unstack_ctrl.unstack_len {
                    self.memory.pop();
                }
                1
            }
            InstructionType::Declaration(dec) => {
                let lit = self
                    .memory
                    .last()
                    .expect("Panic: stack is empty, cannot perform declaration with value")
                    .clone();
                for _ in 0..dec.unstack_len {
                    self.memory.pop();
                }
                self.memory.push(lit);
                1
            }
            InstructionType::RunCall(call) => {
                self.memory
                    .push(Literal::Process(call.name.clone(), *next_pid));
                action = Some(GlobalAction::StartProgram(call.name.clone(), *next_pid));
                *next_pid += 1;
                1
            }
            InstructionType::EndProgram => {
                action = Some(GlobalAction::EndProgram);
                1
            }
            InstructionType::FnCall(f) => {
                // currently, only the print function is implemented
                if f.name != "print" {
                    panic!("implement a proper function call in the VM");
                }
                let lit = self
                    .memory
                    .last()
                    .expect("Panic: stack is empty, cannot perform function call")
                    .clone();
                for _ in 0..f.unstack_len {
                    self.memory.pop();
                }

                let str = lit
                    .into_tuple()
                    .unwrap()
                    .iter()
                    .map(|lit| lit.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                println!("{}", str);
                1
            }
            InstructionType::WaitStart(_) => {
                // this instruction is not executed, it is used to create a dependency in case of a wait
                self.global_state_stack.push(global_state_id);
                1
            }
            InstructionType::Wait(wait_ctrl) => {
                let cond = self.memory.last().unwrap().is_true();
                for _ in 0..wait_ctrl.unstack_len {
                    self.memory.pop();
                }
                if cond {
                    1
                } else {
                    if global_state_id
                        == self
                            .global_state_stack
                            .pop()
                            .expect("Panic: global_state_stack is empty, cannot pop")
                    {
                        action = Some(GlobalAction::Wait);
                    }
                    // otherwise we do not wait since the state has changed
                    wait_ctrl.jump
                }
            }
            InstructionType::Destruct(_) => {
                // The values are in a tuple on the top of the stack
                let tuple = self
                    .memory
                    .pop()
                    .expect("Panic: stack is empty, cannot destruct")
                    .into_tuple()
                    .expect("Panic: cannot convert to tuple");
                for val in tuple.into_iter() {
                    self.memory.push(val);
                }
                1
            }
            InstructionType::Push(literal) => {
                self.memory.push(literal.clone());
                1
            }
            InstructionType::Send(send_ctrl) => {
                let value = self
                    .memory
                    .last()
                    .expect("Panic: stack is empty, cannot send")
                    .clone();

                for _ in 0..send_ctrl.unstack_len {
                    self.memory.pop();
                }

                let receiver = channels.send(self.id, send_ctrl.channel_name.clone(), value);

                action = Some(GlobalAction::Send(send_ctrl.channel_name.clone(), receiver));
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
                    Some(idx) => self
                        .memory
                        .get(self.memory.len() - 1 - idx)
                        .expect("Panic: stack is empty, cannot connect")
                        .clone()
                        .to_pid()
                        .expect("Panic: cannot convert to pid"),
                };
                let receiver_pid = match connect_ctrl.receiver_idx {
                    None => self.id,
                    Some(idx) => self
                        .memory
                        .get(self.memory.len() - 1 - idx)
                        .expect("Panic: stack is empty, cannot connect")
                        .clone()
                        .to_pid()
                        .expect("Panic: cannot convert to pid"),
                };

                let sender_info = channels
                    .connect(
                        sender_pid,
                        connect_ctrl.sender_channel.clone(),
                        receiver_pid,
                        connect_ctrl.receiver_channel.clone(),
                    )
                    .map_err(|msg| {
                        AlthreadError::new(ErrorType::RuntimeError, cur_inst.pos, msg)
                    })?;
                if let Some(sender_info) = sender_info {
                    action = Some(GlobalAction::Connect(sender_info.0, sender_info.1));
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
                "instruction pointer is becomming negative".to_string(),
            ));
        }
        self.instruction_pointer = new_pos as usize;
        Ok(action)
    }
}
