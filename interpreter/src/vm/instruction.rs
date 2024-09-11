use std::fmt;

use crate::{ast::{node::Node, statement::{expression::{binary_expression::LocalBinaryExpressionNode, primary_expression::LocalPrimaryExpressionNode, unary_expression::LocalUnaryExpressionNode, LocalExpressionNode}, Statement}, token::{binary_assignment_operator::BinaryAssignmentOperator, binary_operator::BinaryOperator, literal::Literal, unary_operator::UnaryOperator}}, error::Pos};

use super::Memory;


#[derive(Debug, Clone)]
pub enum InstructionType {
    Empty,
    Atomic(AtomicControl),
    Expression(ExpressionControl),
    GlobalReads(GlobalReadsControl),
    GlobalAssignment(GlobalAssignmentControl),
    LocalAssignment(LocalAssignmentControl),
    JumpIf(JumpIfControl),
    Jump(JumpControl),
    Unstack(UnstackControl),
    RunCall(RunCallControl),
    EndProgram,
    FnCall(FnCallControl),
    Declaration(DeclarationControl),
    ChannelPeek(String),
    ChannelPop(String),
    Exit,
    Push(Literal),
    Wait(WaitControl),
    Send(SendControl),
    Connect(ConnectionControl),
    //Receive,
    //Any
}
impl fmt::Display for InstructionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Empty => {write!(f, "EMPTY")?},
            Self::Atomic(a) => {write!(f, "{}", a)?},
            Self::Expression(a) => {write!(f, "{}", a)?},
            Self::GlobalReads(a) => {write!(f, "{}", a)?},
            Self::GlobalAssignment(a) => {write!(f, "{}", a)?},
            Self::LocalAssignment(a) => {write!(f, "{}", a)?},
            Self::JumpIf(a) => {write!(f, "{}", a)?},
            Self::Jump(a) => {write!(f, "{}", a)?},
            Self::Unstack(a) => {write!(f, "{}", a)?},
            Self::RunCall(a) => {write!(f, "{}", a)?},
            Self::EndProgram => {write!(f, "end program")?},
            Self::FnCall(a) => {write!(f, "{}", a)?},
            Self::Exit => {write!(f, "exit")?},
            Self::Declaration(d) => {write!(f, "{}", d)?},
            Self::Push(l) => {write!(f, "push ({})", l)?},
            Self::Wait(w) => {write!(f, "{}", w)?},
            Self::Send(s) => {write!(f, "{}", s)?},
            Self::ChannelPeek(s) => {write!(f, "peek '{}'", s)?},
            Self::ChannelPop(s) => {write!(f, "pop '{}'", s)?},
            Self::Connect(c) => {write!(f, "{}", c)?},
        }
        Ok(())
    }
}

impl InstructionType {
    pub fn is_local(&self) -> bool {
        match self {
            Self::GlobalAssignment(_)
            | Self::Send(_)
            | Self::RunCall(_)
            | Self::ChannelPop(_)
            | Self::GlobalReads(_) => false,

            Self::Connect(_) // connect is global only if a process was waiting
            | Self::Wait(_) // wait is global only if the condition is false
            | Self::ChannelPeek(_) // This is a local peek that should be directly followed by a pop is the read occurs
            | Self::Empty
            | Self::Expression(_)
            | Self::LocalAssignment(_)
            | Self::JumpIf(_)
            | Self::Jump(_)
            | Self::Unstack(_)
            | Self::EndProgram
            | Self::FnCall(_)
            | Self::Declaration(_)
            | Self::Exit
            | Self::Push(_) => true,

            Self::Atomic(_) => todo!(),

        }
    }
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub pos: Option<Pos>,
    pub control: InstructionType,
}
impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.pos {
            Some(pos) => write!(f, "{}", pos.line)?,
            None => { },
        };
        write!(f, ": {}", self.control)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct AtomicControl {
    pub node: Node<Statement>,
}
impl fmt::Display for AtomicControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "atomic")?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct JumpIfControl {
    pub jump_false: i64,
    pub unstack_len: usize,
}
impl fmt::Display for JumpIfControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "jumpIf {} (unstack {})", self.jump_false, self.unstack_len)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct JumpControl {
    pub jump: i64,
}
impl fmt::Display for JumpControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "jump {}", self.jump)?;
        Ok(())
    }
}


#[derive(Debug, Clone)]
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



#[derive(Debug, Clone)]
pub struct ExpressionControl {
    pub root: LocalExpressionNode,
}
impl fmt::Display for ExpressionControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "eval {}", self.root)?;
        Ok(())
    }
}


#[derive(Debug, Clone)]
pub struct GlobalReadsControl {
    pub variables: Vec<String>,
}
impl fmt::Display for GlobalReadsControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "global_read {}", self.variables.join(","))?;
        Ok(())
    }
}


#[derive(Debug, Clone)]
pub struct UnstackControl {
    pub unstack_len: usize,
}
impl fmt::Display for UnstackControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unstack {}", self.unstack_len)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DeclarationControl {
    pub unstack_len: usize,
}
impl fmt::Display for DeclarationControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "declare var with value (unstack {})", self.unstack_len)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SendControl {
    pub channel_name: String,
    pub nb_values: usize,
    pub unstack_len: usize,
}
impl fmt::Display for SendControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "send {} {}-tuple (unstack {})", self.channel_name, self.nb_values, self.unstack_len)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
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
        write!(f, "connect [&{}] {}->{} [&{}]", 
        if self.sender_idx.is_none() { "self".to_string() } else { self.sender_idx.unwrap().to_string() }, 
        self.sender_channel, self.receiver_channel, 
        if self.receiver_idx.is_none() { "self".to_string() } else { self.receiver_idx.unwrap().to_string() }, 
    )?;
        Ok(())
    }
}


#[derive(Debug, Clone)]
pub struct GlobalAssignmentControl {
    pub identifier: String,
    pub operator: BinaryAssignmentOperator,
    pub unstack_len: usize,
}
impl fmt::Display for GlobalAssignmentControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} (unstack {})", self.identifier, self.operator, self.unstack_len)?;
        Ok(())
    }
}


#[derive(Debug, Clone)]
pub struct LocalAssignmentControl {
    pub index: usize,
    pub operator: BinaryAssignmentOperator,
    pub unstack_len: usize,
}
impl fmt::Display for LocalAssignmentControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] {}  (unstack {})", self.index, self.operator, self.unstack_len)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RunCallControl {
    pub name: String
}
impl fmt::Display for RunCallControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "run {}()", self.name)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FnCallControl {
    pub name: String,
    pub unstack_len: usize,
}
impl fmt::Display for FnCallControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}()  (unstack {})", self.name, self.unstack_len)?;
        Ok(())
    }
}


#[derive(Debug)]
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


impl LocalExpressionNode {
    pub fn eval(&self, mem: &Memory) -> Result<Literal, String> {
        match self {
            LocalExpressionNode::Binary(binary_exp) => {
                binary_exp.eval(mem)
            },
            LocalExpressionNode::Unary(unary_exp) => {
                unary_exp.eval(mem)
            },
            LocalExpressionNode::Primary(primary_exp) => {
                match primary_exp {
                    LocalPrimaryExpressionNode::Literal(literal) => {
                        Ok(literal.value.clone())
                    },
                    LocalPrimaryExpressionNode::Var(local_var) => {
                        let lit = mem.get(mem.len() - 1 - local_var.index).ok_or("local variable index does not exist in memory".to_string())?;
                        Ok(lit.clone())
                    },
                    LocalPrimaryExpressionNode::Expression(expr) => {
                        expr.as_ref().eval(mem)
                    },
                }
            },
        }
    }
}

impl LocalBinaryExpressionNode {
    pub fn eval(&self, mem: &Memory) -> Result<Literal, String> {
        let left = self.left.eval(mem)?;
        let right = self.right.eval(mem)?;

        match self.operator {
            BinaryOperator::Add => left.add(&right),
            BinaryOperator::Subtract => left.subtract(&right),
            BinaryOperator::Multiply => left.multiply(&right),
            BinaryOperator::Divide => left.divide(&right),
            BinaryOperator::Modulo => left.modulo(&right),
            BinaryOperator::Equals => left.equals(&right),
            BinaryOperator::NotEquals => left.not_equals(&right),
            BinaryOperator::LessThan => left.less_than(&right),
            BinaryOperator::LessThanOrEqual => left.less_than_or_equal(&right),
            BinaryOperator::GreaterThan => left.greater_than(&right),
            BinaryOperator::GreaterThanOrEqual => right.greater_than_or_equal(&right),
            BinaryOperator::And => left.and(&right),
            BinaryOperator::Or => left.or(&right),
        }
    }
}
impl LocalUnaryExpressionNode {
    pub fn eval(&self, mem: &Memory) -> Result<Literal, String> {
        let operand = self.operand.eval(mem)?;
        match self.operator {
            UnaryOperator::Positive => operand.positive(),
            UnaryOperator::Negative => operand.negative(),
            UnaryOperator::Not => operand.not(),
        }
    }
}