use std::fmt;

use crate::{ast::{node::Node, statement::{expression::{binary_expression::LocalBinaryExpressionNode, primary_expression::LocalPrimaryExpressionNode, unary_expression::LocalUnaryExpressionNode, Expression, LocalExpression, LocalExpressionNode}, Statement}, token::{binary_assignment_operator::BinaryAssignmentOperator, binary_operator::BinaryOperator, datatype::DataType, literal::Literal, unary_operator::UnaryOperator}}, error::Pos};

use super::Memory;


struct Span {
    start: usize,
    end: usize,
}


#[derive(Debug)]
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
    Exit,
    PushNull(DataType),
    Wait(WaitControl),
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
            Self::PushNull(d) => {write!(f, "push null ({})", d)?},
            Self::Wait(w) => {write!(f, "{}", w)?},
        }
        Ok(())
    }
}

impl InstructionType {
    pub fn is_local(&self) -> bool {
        match self {
            Self::GlobalAssignment(_)
            | Self::RunCall(_)
            | Self::GlobalReads(_) => false,

            | Self::Wait(_) // wait is global only if the condition is false
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
            | Self::PushNull(_) => true,

            Self::Atomic(_) => todo!(),

        }
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct AtomicControl {
    pub node: Node<Statement>,
}
impl fmt::Display for AtomicControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "atomic")?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct JumpIfControl {
    pub jump_false: i64,
    pub unstack_len: usize,
}
impl fmt::Display for JumpIfControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "jumpIf {}", self.jump_false)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct JumpControl {
    pub jump: i64,
}
impl fmt::Display for JumpControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "jump {}", self.jump)?;
        Ok(())
    }
}


#[derive(Debug)]
pub struct WaitControl {
    pub jump: i64,
    pub unstack_len: usize,
}
impl fmt::Display for WaitControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "wait {}", self.jump)?;
        Ok(())
    }
}



#[derive(Debug)]
pub struct ExpressionControl {
    pub root: LocalExpressionNode,
}
impl fmt::Display for ExpressionControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "eval {}", self.root)?;
        Ok(())
    }
}


#[derive(Debug)]
pub struct GlobalReadsControl {
    pub variables: Vec<String>,
}
impl fmt::Display for GlobalReadsControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "global_read {}", self.variables.join(","))?;
        Ok(())
    }
}


#[derive(Debug)]
pub struct UnstackControl {
    pub unstack_len: usize,
}
impl fmt::Display for UnstackControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unstack {}", self.unstack_len)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct DeclarationControl {
    pub unstack_len: usize,
}
impl fmt::Display for DeclarationControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "declare var with value")?;
        Ok(())
    }
}


#[derive(Debug)]
pub struct GlobalAssignmentControl {
    pub identifier: String,
    pub operator: BinaryAssignmentOperator,
    pub unstack_len: usize,
}
impl fmt::Display for GlobalAssignmentControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} ", self.identifier, self.operator)?;
        Ok(())
    }
}


#[derive(Debug)]
pub struct LocalAssignmentControl {
    pub index: usize,
    pub operator: BinaryAssignmentOperator,
    pub unstack_len: usize,
}
impl fmt::Display for LocalAssignmentControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] {} ", self.index, self.operator)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct RunCallControl {
    pub name: String
}
impl fmt::Display for RunCallControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "run {}()", self.name)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct FnCallControl {
    pub name: String,
    pub unstack_len: usize,
}
impl fmt::Display for FnCallControl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}()", self.name)?;
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
            writeln!(f, "  - {:?}", i.control)?;
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