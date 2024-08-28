use crate::ast::{node::Node, statement::{expression::Expression, Statement}, token::literal::Literal};

use super::Env;

struct Span {
    start: usize,
    end: usize,
}

pub struct ProcessEnv2 {
    registers: Vec<Literal>,
}

pub enum InstructionType {
    Empty,
    Atomic(AtomicControl),
    //While(WhileControl),
    If(IfControl),
    //Wait(WaitControl),
    //Receive,
    //Any
}

pub struct Instruction {
    pub dependencies: Vec<usize>,
    pub span: usize,
    pub control: InstructionType,
}

pub struct AtomicControl {
    pub node: Node<Statement>,
}

pub struct IfControl {
    pub condition: Node<Expression>,
    pub jump_true: usize,
    pub jump_false: usize,
}

impl Instruction {
    fn eval(&self, env: &mut ProcessEnv2) -> Option<usize> {
        let next = match self.control {
            InstructionType::Atomic(c) => { c.node.eval(env); 1 },
            InstructionType::If(c) => { 
                if c.condition.eval(env).unwrap().unwrap().is_true() {
                    c.jump_true
                } else {
                    c.jump_false
                }
            },
            InstructionType::Empty => 1,
            _ => panic!("Not implemented"),
        };
        Ok(next)
    }
}


pub struct ProcessCode {
    pub name: String,
    pub instructions: Vec<Instruction>,
}