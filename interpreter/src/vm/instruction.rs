use serde::{Deserialize, Serialize};
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
    Expression(ExpressionControl),
    GlobalReads(GlobalReadsControl),
    GlobalAssignment(GlobalAssignmentControl),
    LocalAssignment(LocalAssignmentControl),
    JumpIf(JumpIfControl),
    Jump(JumpControl),
    Break(BreakLoopControl),
    Unstack(UnstackControl),
    RunCall(RunCallControl),
    EndProgram,
    FnCall(FnCallControl),
    Declaration(DeclarationControl),
    ChannelPeek(String),
    ChannelPop(String),
    Destruct(usize),
    Exit,
    Push(Literal),
    WaitStart(WaitStartControl),
    Wait(WaitControl),
    Send(SendControl),
    SendWaiting,
    Connect(ConnectionControl),
    AtomicStart,
    AtomicEnd,
    //Receive,
    //Any
}
impl fmt::Display for InstructionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "EMPTY")?,
            Self::Expression(a) => write!(f, "{}", a)?,
            Self::GlobalReads(a) => write!(f, "{}", a)?,
            Self::GlobalAssignment(a) => write!(f, "{}", a)?,
            Self::LocalAssignment(a) => write!(f, "{}", a)?,
            Self::JumpIf(a) => write!(f, "{}", a)?,
            Self::Jump(a) => write!(f, "{}", a)?,
            Self::Unstack(a) => write!(f, "{}", a)?,
            Self::Destruct(d) => write!(f, "destruct {}", d)?,
            Self::RunCall(a) => write!(f, "{}", a)?,
            Self::Break(a) => write!(f, "{}", a)?,
            Self::EndProgram => write!(f, "end program")?,
            Self::FnCall(a) => write!(f, "{}", a)?,
            Self::Exit => write!(f, "exit")?,
            Self::Declaration(d) => write!(f, "{}", d)?,
            Self::Push(l) => write!(f, "push ({})", l)?,
            Self::WaitStart(w) => write!(f, "{}", w)?,
            Self::Wait(w) => write!(f, "{}", w)?,
            Self::Send(s) => write!(f, "{}", s)?,
            Self::SendWaiting => write!(f, "send waiting?")?,
            Self::ChannelPeek(s) => write!(f, "peek '{}'", s)?,
            Self::ChannelPop(s) => write!(f, "pop '{}'", s)?,
            Self::Connect(c) => write!(f, "{}", c)?,
            Self::AtomicStart => write!(f, "atomic start")?,
            Self::AtomicEnd => write!(f, "atomic end")?,
        }
        Ok(())
    }
}

impl InstructionType {
    pub fn is_local(&self) -> bool {
        match self {
              Self::GlobalAssignment(_)
            | Self::Send(_)
            | Self::ChannelPeek(_)
            | Self::AtomicStart // starts a block that surely contains a global operation
            | Self::WaitStart(_) => false, // wait starts an atomic block to evaluate the conditions

            Self::GlobalReads(r) => r.only_const, // a global read is local only if it reads constant variables

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

            Self::Connect(_) // connect is global only if a process was waiting
            | Self::RunCall(_)
            | Self::ChannelPop(_) // This is a local because it follows a peek
            | Self::Wait(_)
            | Self::Empty
            | Self::Expression(_)
            | Self::LocalAssignment(_)
            | Self::JumpIf(_)
            | Self::Jump(_)
            | Self::Break(_)
            | Self::Destruct(_)
            | Self::Unstack(_)
            | Self::EndProgram
            | Self::FnCall(_)
            | Self::Declaration(_)
            | Self::Exit
            | Self::AtomicEnd
            | Self::SendWaiting
            | Self::Push(_) => true,

        }
    }

    pub fn is_atomic_start(&self) -> bool {
        match self {
            Self::AtomicStart => true,
            Self::WaitStart(_) => true,
            _ => false,
        }
    }
    pub fn is_atomic_end(&self) -> bool {
        match self {
            Self::AtomicEnd => true,
            Self::Break(b) => b.stop_atomic,
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

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct JumpIfControl {
    pub jump_false: i64,
    pub unstack_len: usize,
}
impl fmt::Display for JumpIfControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "jumpIf {} (unstack {})",
            self.jump_false, self.unstack_len
        )?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct BreakLoopControl {
    pub jump: i64,
    pub unstack_len: usize,
    pub stop_atomic: bool,
}
impl fmt::Display for BreakLoopControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "break (unstack {})", self.unstack_len)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct JumpControl {
    pub jump: i64,
}
impl fmt::Display for JumpControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "jump {}", self.jump)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct WaitControl {
    pub jump: i64,
    pub unstack_len: usize,
}
impl fmt::Display for WaitControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "wait {} (unstack {})", self.jump, self.unstack_len)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct WaitStartControl {
    pub dependencies: WaitDependency,
    pub start_atomic: bool,
}
impl fmt::Display for WaitStartControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "wait start")?;
        if self.start_atomic {
            write!(f, " atomic")?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExpressionControl {
    pub root: LocalExpressionNode,
}
impl fmt::Display for ExpressionControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "eval {}", self.root)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct GlobalReadsControl {
    pub variables: Vec<String>,
    pub only_const: bool,
}
impl fmt::Display for GlobalReadsControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "global_read {}", self.variables.join(","))?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct UnstackControl {
    pub unstack_len: usize,
}
impl fmt::Display for UnstackControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unstack {}", self.unstack_len)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct DeclarationControl {
    pub unstack_len: usize,
}
impl fmt::Display for DeclarationControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "declare var with value (unstack {})", self.unstack_len)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct SendControl {
    pub channel_name: String,
    pub unstack_len: usize,
}
impl fmt::Display for SendControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "send to {} (unstack {})",
            self.channel_name, self.unstack_len
        )?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ConnectionControl {
    /// the index of the sender pid in the stack (none if the sender is the current process)
    pub sender_idx: Option<usize>,
    /// the index of the receiver pid in the stack (none if the receiver is the current process)
    pub receiver_idx: Option<usize>,
    pub sender_channel: String,
    pub receiver_channel: String,
}
impl fmt::Display for ConnectionControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "connect [&{}] {}->{} [&{}]",
            if self.sender_idx.is_none() {
                "self".to_string()
            } else {
                self.sender_idx.unwrap().to_string()
            },
            self.sender_channel,
            self.receiver_channel,
            if self.receiver_idx.is_none() {
                "self".to_string()
            } else {
                self.receiver_idx.unwrap().to_string()
            },
        )?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GlobalAssignmentControl {
    pub identifier: String,
    pub operator: BinaryAssignmentOperator,
    pub unstack_len: usize,
}
impl fmt::Display for GlobalAssignmentControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} (unstack {})",
            self.identifier, self.operator, self.unstack_len
        )?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct LocalAssignmentControl {
    pub index: usize,
    pub operator: BinaryAssignmentOperator,
    pub unstack_len: usize,
}
impl fmt::Display for LocalAssignmentControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{}] {}  (unstack {})",
            self.index, self.operator, self.unstack_len
        )?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct RunCallControl {
    pub name: String,
    pub unstack_len: usize,
}
impl fmt::Display for RunCallControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "run {}()", self.name)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct FnCallControl {
    pub name: String,
    pub unstack_len: usize,
    pub variable_idx: Option<usize>,
    pub arguments: Option<Vec<usize>>,
}
impl fmt::Display for FnCallControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}()  (unstack {})", self.name, self.unstack_len)?;
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
