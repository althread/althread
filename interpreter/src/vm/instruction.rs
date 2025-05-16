use std::fmt;

use crate::{
    ast::{
        statement::{expression::LocalExpressionNode, waiting_case::WaitDependency},
        token::{binary_assignment_operator::BinaryAssignmentOperator, literal::Literal},
    },
    error::Pos,
};

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionType {
    Empty,
    Expression(LocalExpressionNode),
    Push(Literal),
    Unstack {
        unstack_len: usize,
    },
    Destruct,
    GlobalReads {
        variables: Vec<String>,
        only_const: bool,
    },
    GlobalAssignment {
        identifier: String,
        operator: BinaryAssignmentOperator,
        unstack_len: usize,
    },
    LocalAssignment {
        index: usize,
        operator: BinaryAssignmentOperator,
        unstack_len: usize,
    },

    Declaration {
        unstack_len: usize,
    },
    RunCall {
        name: String,
        unstack_len: usize,
    },
    FnCall {
        name: String,
        unstack_len: usize,
        variable_idx: Option<usize>,
        arguments: Option<Vec<usize>>,
    },
    JumpIf {
        jump_false: i64,
        unstack_len: usize,
    },
    Jump(i64),
    Break {
        jump: i64,
        unstack_len: usize,
        stop_atomic: bool,
    },
    ChannelPeek(String),
    ChannelPop(String),
    
    WaitStart {
        dependencies: WaitDependency,
        start_atomic: bool,
    },
    Wait {
        jump: i64,
        unstack_len: usize,
    },
    Send {
        channel_name: String,
        unstack_len: usize,
    },
    Connect {
        /// the index of the sender pid in the stack (none if the sender is the current process)
        sender_pid: Option<usize>,
        /// the index of the receiver pid in the stack (none if the receiver is the current process)
        receiver_pid: Option<usize>,
        sender_channel: String,
        receiver_channel: String,
    },
    AtomicStart,
    AtomicEnd,
    EndProgram,
    Exit,
}
impl fmt::Display for InstructionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "EMPTY")?,
            Self::Expression(a) => write!(f, "eval {}", a)?,
            Self::GlobalReads { variables, .. } => {
                write!(f, "global_read {}", variables.join(","))?
            }
            Self::GlobalAssignment {
                identifier,
                operator,
                unstack_len,
            } => write!(f, "{} {} (unstack {})", identifier, operator, unstack_len)?,
            Self::LocalAssignment {
                index,
                operator,
                unstack_len,
            } => write!(f, "[{}] {} (unstack {})", index, operator, unstack_len)?,
            Self::JumpIf {
                jump_false,
                unstack_len,
            } => write!(f, "jumpIf {} (unstack {})", jump_false, unstack_len)?,
            Self::Jump(a) => write!(f, "jump {}", a)?,
            Self::Unstack { unstack_len } => write!(f, "unstack {}", unstack_len)?,
            Self::Destruct => write!(f, "destruct tuple", )?,
            Self::RunCall { name, .. } => write!(f, "run {}()", name)?,
            Self::Break { unstack_len, .. } => write!(f, "break (unstack {})", unstack_len)?,
            Self::EndProgram => write!(f, "end program")?,
            Self::FnCall {
                name, unstack_len, ..
            } => write!(f, "{}()  (unstack {})", name, unstack_len)?,
            Self::Exit => write!(f, "exit")?,
            Self::Declaration { unstack_len } => {
                write!(f, "declare var with value (unstack {})", unstack_len)?
            }
            Self::Push(l) => write!(f, "push ({})", l)?,
            Self::WaitStart { start_atomic, .. } => {
                write!(f, "await start")?;
                if *start_atomic {
                    write!(f, " atomic")?;
                }
                ()
            }
            Self::Wait { jump, unstack_len } => {
                write!(f, "await {} (unstack {})", jump, unstack_len)?
            }
            Self::Send {
                channel_name,
                unstack_len,
            } => write!(f, "send to {} (unstack {})", channel_name, unstack_len)?,
            Self::ChannelPeek(s) => write!(f, "peek '{}'", s)?,
            Self::ChannelPop(s) => write!(f, "pop '{}'", s)?,
            Self::Connect {
                sender_pid,
                receiver_pid,
                sender_channel,
                receiver_channel,
            } => {
                write!(
                    f,
                    "connect [&{}] {}->{} [&{}]",
                    if sender_pid.is_none() {
                        "self".to_string()
                    } else {
                        sender_pid.unwrap().to_string()
                    },
                    sender_channel,
                    receiver_channel,
                    if receiver_pid.is_none() {
                        "self".to_string()
                    } else {
                        receiver_pid.unwrap().to_string()
                    },
                )?;
                ()
            }
            Self::AtomicStart => write!(f, "atomic start")?,
            Self::AtomicEnd => write!(f, "atomic end")?,
        }
        Ok(())
    }
}

impl InstructionType {
    pub fn is_local(&self) -> bool {
        match self {
              Self::GlobalAssignment {..}
            | Self::Send {..}
            | Self::ChannelPeek(_)
            | Self::AtomicStart // starts a block that surely contains a global operation
            | Self::WaitStart {..} => false, // await starts an atomic block to evaluate the conditions

            Self::GlobalReads {only_const, ..} => *only_const, // a global read is local only if it reads constant variables

            // This should be checked. I think the following are not global because
            // they do not write or read any global variable or channel
            // Indeed, starting a process do not write anything. The process itself is the one that will write
            // Similarly, connected a channel that was waiting to send or receive is not a global operation
            // because the processes that wait are different and their operation are global so they will 
            // not be done atomically
            // They are global actions but that do not require the process to pause

            // Channel peek is a little bit different because it might not be followed in the case the read is not completed (if the guard was false for instance)
            // In that case, I am not sure that the peek should be considered as a local operation
            // Anyway, it is hard to know in advance (in the case we want to stop *before* global 
            // instructions instead of after)

            Self::Connect {..} // connect is global only if a process was waiting
            | Self::RunCall {..}
            | Self::ChannelPop(_) // This is a local because it follows a peek
            | Self::Wait {..}
            | Self::Empty
            | Self::Expression(_)
            | Self::LocalAssignment {..}
            | Self::JumpIf {..}
            | Self::Jump(_)
            | Self::Break {..}
            | Self::Destruct
            | Self::Unstack {..}
            | Self::FnCall {..}
            | Self::Declaration {..}
            | Self::AtomicEnd
            | Self::EndProgram
            | Self::Exit
            | Self::Push(_) => true,

        }
    }

    pub fn is_atomic_start(&self) -> bool {
        match self {
            Self::AtomicStart => true,
            Self::WaitStart { .. } => true,
            _ => false,
        }
    }
    pub fn is_atomic_end(&self) -> bool {
        match self {
            Self::AtomicEnd => true,
            Self::Break { stop_atomic, .. } => *stop_atomic,
            Self::EndProgram => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Instruction {
    pub pos: Option<Pos>,
    pub control: InstructionType,
}

impl Instruction {
    pub fn is_atomic_start(&self) -> bool {
        self.control.is_atomic_start()
    }
    pub fn is_atomic_end(&self) -> bool {
        self.control.is_atomic_end()
    }
    pub fn is_end(&self) -> bool {
        match self.control {
            InstructionType::EndProgram => true,
            _ => false,
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.pos {
            Some(pos) => write!(f, "{}", pos.line)?,
            None => {}
        };
        write!(f, ": {}", self.control)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProgramCode {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

// impl display for ProcessCode
impl fmt::Display for ProgramCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.name)?;
        for i in self.instructions.iter() {
            writeln!(f, "  - {}", i.control)?;
        }
        Ok(())
    }
}
