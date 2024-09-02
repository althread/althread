use std::fmt;

use crate::ast::{node::Node, statement::{expression::{Expression, LocalExpression, LocalExpressionNode}, Statement}, token::{binary_assignment_operator::BinaryAssignmentOperator, literal::Literal}};


struct Span {
    start: usize,
    end: usize,
}

type Memory = Vec<Literal>;

#[derive(Debug)]
pub struct ProgramState {
    memory: Memory,
    instructions: Vec<Instruction>,
    instruction_pointer: usize,
}

#[derive(Debug)]
pub enum InstructionType {
    Empty,
    Atomic(AtomicControl),
    Expression(ExpressionControl),
    GlobalReads(GlobalReadsControl),
    Write(WriteControl),
    GlobalAssignment(GlobalAssignmentControl),
    LocalAssignment(LocalAssignmentControl),
    //While(WhileControl),
    JumpIf(JumpIfControl),
    Jump(JumpControl),
    PushNull,
    //Wait(WaitControl),
    //Receive,
    //Any
}

#[derive(Debug)]
pub struct Instruction {
    pub span: usize,
    pub control: InstructionType,
}

#[derive(Debug)]
pub struct AtomicControl {
    pub node: Node<Statement>,
}

#[derive(Debug)]
pub struct JumpIfControl {
    pub jump_false: usize,
}
#[derive(Debug)]
pub struct JumpControl {
    pub jump: usize,
}

#[derive(Debug)]
pub struct ExpressionControl {
    pub root: LocalExpressionNode,
}
#[derive(Debug)]
pub struct GlobalReadsControl {
    pub variables: Vec<String>,
}
#[derive(Debug)]
pub struct WriteControl {
    pub variable: usize,
}

#[derive(Debug)]
pub struct GlobalAssignmentControl {
    pub identifier: String,
    pub operator: BinaryAssignmentOperator,
}
#[derive(Debug)]
pub struct LocalAssignmentControl {
    pub index: usize,
    pub operator: BinaryAssignmentOperator,
}


impl ProgramState {
    fn next(&self, globals: &mut Memory) {
        /*let inc = match self.instructions[self.instruction_pointer].control {
            InstructionType::Atomic(c) => { 
                c.node.exec(self.memory); 
                1 
            },
            InstructionType::JumpIf(c) => {
                if self.memory.pop().unwrap().is_true() {
                    1
                } else {
                    c.jump_false
                }
            },
            InstructionType::Jump(c) => c.jump,
            InstructionType::Read(c) => { 
                for i in c.variables.iter() {
                    self.memory.push(globals[*i].clone());
                }
                1
            },
            InstructionType::Local(c) => { 
                c.node.exec(self.memory);
                1 
            },
            InstructionType::Write(c) => { 
                let v = self.memory.pop().unwrap();
                globals[c.variable] = v;
                1
            },
            InstructionType::Empty => 1,
            _ => panic!("Not implemented"),
        };
        
        self.instruction_pointer += inc;*/
    }
}


pub struct ProcessCode {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

// impl debug for ProcessCode
impl fmt::Display for ProcessCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.name)?;
        for i in self.instructions.iter() {
            writeln!(f, "  - {:?}", i.control)?;
        }
        Ok(())
    }
}
