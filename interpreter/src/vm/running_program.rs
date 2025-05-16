use std::{
    hash::{Hash, Hasher},
    rc::Rc,
};

use crate::{
    ast::token::literal::Literal,
    compiler::stdlib::Stdlib,
    error::{AlthreadError, AlthreadResult, ErrorType},
};

use super::{
    channels::{Channels, ReceiverInfo},
    instruction::{Instruction, InstructionType, ProgramCode},
    str_to_expr_error, GlobalAction, GlobalActions, GlobalMemory, Memory,
};

#[derive(Debug, Clone)]
pub struct RunningProgramState<'a> {
    pub name: String,

    memory: Memory,
    code: &'a ProgramCode,
    instruction_pointer: usize,
    pub id: usize,

    pub stdlib: Rc<Stdlib>,
}

impl PartialEq for RunningProgramState<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.memory == other.memory
            && self.name == other.name
            && self.instruction_pointer == other.instruction_pointer
    }
}

impl Hash for RunningProgramState<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.memory.hash(state);
        self.instruction_pointer.hash(state);
    }
}

impl<'a> RunningProgramState<'a> {
    pub fn new(
        id: usize,
        name: String,
        code: &'a ProgramCode,
        args: Literal,
        stdlib: Rc<Stdlib>,
    ) -> Self {
        let arg_len = if let Literal::Tuple(v) = &args {
            v.len()
        } else {
            panic!("args should be a tuple")
        };

        let memory = if arg_len > 0 { vec![args] } else { Vec::new() };

        Self {
            memory,
            code,
            instruction_pointer: 0,
            name,
            id,
            stdlib,
        }
    }

    pub fn current_state(&self) -> (&Memory, usize) {
        (&self.memory, self.instruction_pointer)
    }

    pub fn current_instruction(&self) -> AlthreadResult<&Instruction> {
        self.code
            .instructions
            .get(self.instruction_pointer)
            .ok_or(AlthreadError::new(
                ErrorType::InstructionNotAllowed,
                None,
                format!(
                "the current instruction pointer points to no instruction (pointer:{}, program:{})",
                self.instruction_pointer, self.name
            ),
            ))
    }

    pub fn has_terminated(&self) -> bool {
        if let Some(inst) = self.current_instruction().ok() {
            inst.is_end()
        } else {
            true
        }
    }

    pub fn next_global(
        &mut self,
        globals: &mut GlobalMemory,
        channels: &mut Channels,
        next_pid: &mut usize,
    ) -> AlthreadResult<(GlobalActions, Vec<Instruction>)> {
        let mut instructions = Vec::new();
        let mut actions = Vec::new();
        let mut wait = false;
        let mut end = false;
        loop {
            let (at_actions, at_instructions) = self.next_atomic(globals, channels, next_pid)?;

            actions.extend(at_actions.actions);
            instructions.extend(at_instructions);

            if at_actions.wait {
                wait = true;
                break;
            }
            if at_actions.end {
                end = true;
                break;
            }
            if self.is_next_instruction_global() {
                break;
            }
        }
        Ok((GlobalActions { actions, wait, end }, instructions))
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
    ) -> AlthreadResult<(GlobalActions, Vec<Instruction>)> {
        let mut instructions = Vec::new();

        let mut result = GlobalActions {
            actions: Vec::new(),
            wait: false,
            end: false,
        };
        // if the next instruction is not the start of an atomic block, we execute the next instruction
        if !self.current_instruction()?.is_atomic_start() {
            instructions.push(self.current_instruction()?.clone());
            let action = self.next(globals, channels, next_pid)?;
            if let Some(action) = action {
                if action == GlobalAction::Wait {
                    result.wait = true;
                } else if action == GlobalAction::EndProgram {
                    result.end = true;
                } else {
                    result.actions.push(action);
                }
            }
            return Ok((result, instructions));
        }
        // else we execute all the instructions until the end of the atomic block
        loop {
            instructions.push(self.current_instruction()?.clone());
            let action = self.next(globals, channels, next_pid)?;
            if let Some(action) = action {
                if action == GlobalAction::Wait {
                    result.wait = true;
                    break;
                } else {
                    result.actions.push(action);
                }
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
    ) -> AlthreadResult<Option<GlobalAction>> {
        let cur_inst =
            self.code
                .instructions
                .get(self.instruction_pointer)
                .ok_or(AlthreadError::new(
                    ErrorType::InstructionNotAllowed,
                    None,
                    format!(
                        "the current instruction pointer points to no instruction (pointer {}",
                        self.instruction_pointer
                    ),
                ))?;

        //println!("{} current memory:\n{}", self.id, self.memory.iter().map(|lit| format!("{:?}", lit)).collect::<Vec<_>>().join("\n"));
        //println!("{} running instruction {}", self.id, self.current_instruction().unwrap());

        let mut action = None;

        let pos_inc = match &cur_inst.control {
            InstructionType::Empty => 1,
            InstructionType::AtomicStart => 1,
            InstructionType::AtomicEnd => 1,
            InstructionType::Break {
                unstack_len, jump, ..
            } => {
                for _ in 0..*unstack_len {
                    self.memory.pop();
                }
                *jump
            }
            InstructionType::JumpIf {
                jump_false,
                unstack_len,
            } => {
                let cond = self.memory.last().unwrap().is_true();
                for _ in 0..*unstack_len {
                    self.memory.pop();
                }
                if cond {
                    1
                } else {
                    *jump_false
                }
            }
            InstructionType::Jump(jump) => *jump,
            InstructionType::Expression(exp) => {
                let lit = exp.eval(&mut self.memory).map_err(|msg| {
                    AlthreadError::new(ErrorType::ExpressionError, cur_inst.pos, msg)
                })?;
                self.memory.push(lit);
                1
            }
            InstructionType::GlobalReads { variables, .. } => {
                for var_name in variables.iter() {
                    self.memory.push(
                        globals
                            .get(var_name)
                            .expect(format!("global variable '{}' not found", var_name).as_str())
                            .clone(),
                    );
                }
                1
            }
            InstructionType::GlobalAssignment {
                identifier,
                operator,
                unstack_len,
            } => {
                let lit = self
                    .memory
                    .last()
                    .expect("Panic: stack is empty, cannot perform assignment")
                    .clone();
                for _ in 0..*unstack_len {
                    self.memory.pop();
                }

                let lit = operator
                    .apply(
                        &globals
                            .get(identifier)
                            .expect(format!("global variable '{}' not found", identifier).as_str()),
                        &lit,
                    )
                    .map_err(str_to_expr_error(cur_inst.pos))?;

                globals.insert(identifier.clone(), lit);
                action = Some(GlobalAction::Write(identifier.clone()));
                1
            }
            InstructionType::LocalAssignment {
                index,
                unstack_len,
                operator,
            } => {
                let lit = self
                    .memory
                    .last()
                    .expect("Panic: stack is empty, cannot perform assignment")
                    .clone();
                for _ in 0..*unstack_len {
                    self.memory.pop();
                }

                let len = self.memory.len();

                self.memory[len - 1 - index] = operator
                    .apply(&self.memory[len - 1 - *index], &lit)
                    .map_err(str_to_expr_error(cur_inst.pos))?;
                1
            }
            InstructionType::Unstack { unstack_len } => {
                for _ in 0..*unstack_len {
                    self.memory.pop();
                }
                1
            }
            InstructionType::Declaration { unstack_len } => {
                let lit = self
                    .memory
                    .last()
                    .expect("Panic: stack is empty, cannot perform declaration with value")
                    .clone();
                for _ in 0..*unstack_len {
                    self.memory.pop();
                }
                self.memory.push(lit);
                1
            }
            InstructionType::RunCall { name, unstack_len } => {
                let args = self
                    .memory
                    .last()
                    .expect("Panic: stack is empty, cannot run call")
                    .clone();
                for _ in 0..*unstack_len {
                    self.memory.pop();
                }
                self.memory.push(Literal::Process(name.clone(), *next_pid));
                action = Some(GlobalAction::StartProgram(name.clone(), *next_pid, args));
                *next_pid += 1;
                1
            }
            InstructionType::EndProgram => {
                action = Some(GlobalAction::EndProgram);
                0
            }
            InstructionType::FnCall {
                variable_idx,
                name,
                arguments,
                unstack_len,
            } => {
                if let Some(v_idx) = variable_idx {
                    //println!("f: {:?} on v_idx {}", f.name, v_idx);
                    //println!("current instruction: {:?}", cur_inst);
                    let v_idx = self.memory.len() - 1 - v_idx;
                    let mut lit = self
                        .memory
                        .get(v_idx)
                        .expect("Panic: stack is empty, cannot perform function call")
                        .clone();

                    let interfaces = self.stdlib.get_interfaces(&lit.get_datatype()).ok_or(
                        AlthreadError::new(
                            ErrorType::UndefinedFunction,
                            cur_inst.pos,
                            format!("Type {:?} has no interface available", lit.get_datatype()),
                        ),
                    )?;

                    let fn_idx = interfaces.iter().position(|i| i.name == *name);
                    if fn_idx.is_none() {
                        return Err(AlthreadError::new(
                            ErrorType::UndefinedFunction,
                            cur_inst.pos,
                            format!("undefined function {}", name),
                        ));
                    }
                    let fn_idx = fn_idx.unwrap();
                    let interface = interfaces.get(fn_idx).unwrap();
                    let mut args = match &arguments {
                        None => self.memory.last().unwrap().clone(),
                        Some(v) => {
                            let mut args = Vec::new();
                            for i in 0..v.len() {
                                let idx = self.memory.len() - 1 - v[i];
                                args.push(self.memory.get(idx).unwrap().clone());
                            }
                            Literal::Tuple(args)
                        }
                    };
                    let ret = interface.f.as_ref()(&mut lit, &mut args);

                    //update the memory with object literal
                    self.memory[v_idx] = lit;

                    for _ in 0..*unstack_len {
                        self.memory.pop();
                    }

                    self.memory.push(ret);
                    1
                } else {
                    // currently, only the print function is implemented
                    if *name != "print" {
                        panic!("implement a proper function call in the VM");
                    }
                    let lit = self
                        .memory
                        .last()
                        .expect("Panic: stack is empty, cannot perform function call")
                        .clone();
                    for _ in 0..*unstack_len {
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
                    action = Some(GlobalAction::Print(str));
                    self.memory.push(Literal::Null);
                    1
                }
            }
            InstructionType::WaitStart { .. } => 1,
            InstructionType::Wait {
                unstack_len, jump, ..
            } => {
                let cond = self.memory.last().unwrap().is_true();
                for _ in 0..*unstack_len {
                    self.memory.pop();
                }
                if cond {
                    1
                } else {
                    action = Some(GlobalAction::Wait);
                    *jump
                }
            }
            InstructionType::Destruct => {
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
            InstructionType::Send {
                channel_name,
                unstack_len,
            } => {
                let value = self
                    .memory
                    .last()
                    .expect("Panic: stack is empty, cannot send")
                    .clone();

                for _ in 0..*unstack_len {
                    self.memory.pop();
                }

                let receiver = channels.send(self.id, channel_name.clone(), value);

                action = Some(GlobalAction::Send(channel_name.clone(), receiver));
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
            InstructionType::Connect {
                sender_pid,
                sender_channel,
                receiver_pid,
                receiver_channel,
            } => {
                let sender_pid = match *sender_pid {
                    None => self.id,
                    Some(idx) => self
                        .memory
                        .get(self.memory.len() - 1 - idx)
                        .expect("Panic: stack is empty, cannot connect")
                        .clone()
                        .to_pid()
                        .expect("Panic: cannot convert to pid"),
                };
                let receiver_pid = match receiver_pid {
                    None => self.id,
                    Some(idx) => self
                        .memory
                        .get(self.memory.len() - 1 - idx)
                        .expect("Panic: stack is empty, cannot connect")
                        .clone()
                        .to_pid()
                        .expect("Panic: cannot convert to pid"),
                };

                let is_data_waiting = channels
                    .connect(
                        sender_pid,
                        sender_channel.clone(),
                        receiver_pid,
                        receiver_channel.clone(),
                    )
                    .map_err(|msg| {
                        AlthreadError::new(ErrorType::RuntimeError, cur_inst.pos, msg)
                    })?;
                // A connection has the same effect as a send globally, if some data was waiting to be sent
                if is_data_waiting {
                    action = Some(GlobalAction::Send(
                        sender_channel.clone(),
                        Some(ReceiverInfo {
                            program_id: receiver_pid,
                            channel_name: receiver_channel.clone(),
                        }),
                    ));
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
