use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder, NodeExecutor},
        token::literal::Literal,
    }, compiler::State, env::{instruction::{Instruction, InstructionType, JumpControl, JumpIfControl}, process_env::ProcessEnv}, error::AlthreadResult, parser::Rule
};

use super::{expression::Expression, Statement};

#[derive(Debug)]
pub struct IfControl {
    pub condition: Node<Expression>,
    pub then_block: Box<Node<Statement>>,
    pub else_block: Option<Box<Node<Statement>>>,
}

impl NodeBuilder for IfControl {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let condition = Node::build(pairs.next().unwrap())?;
        let then_block = Node::build(pairs.next().unwrap())?;
        let else_block = pairs.next().map(|pair| Node::build(pair)).transpose()?;

        let else_block = match else_block {
            Some(else_block) => Some(Box::new(else_block)),
            None => None,
        };

        Ok(Self {
            condition,
            then_block: Box::new(then_block),
            else_block: else_block,
        })
    }
}

impl NodeExecutor for IfControl {
    fn eval(&self, env: &mut ProcessEnv) -> AlthreadResult<Option<Literal>> {
        match env.position {
            0 => {
                let condition = self.condition.eval(env.get_child())?.unwrap();
                env.position = if condition.is_true() {
                    1
                } else if self.else_block.is_some() {
                    2
                } else {
                    return Ok(Some(Literal::Null));
                };
                Ok(None)
            }
            1 => Ok(self
                .then_block
                .eval(env.get_child())?
                .map(|_| Literal::Null)),
            2 => Ok(self
                .else_block
                .as_ref()
                .unwrap()
                .eval(env.get_child())?
                .map(|_| Literal::Null)),
            _ => unreachable!(),
        }
    }
}



impl InstructionBuilder for IfControl {
    fn compile(&self, state: &mut State) -> Vec<Instruction> {
        
        let mut instructions = Vec::new();
        let condition = self.condition.compile(state);
        let then_block = self.then_block.compile(state);
        let else_block = self.else_block.as_ref().map(|block| block.compile(state)).unwrap_or_default();


        instructions.extend(condition);
        instructions.push(Instruction {
            span: 0,
            control: InstructionType::JumpIf(JumpIfControl { 
                jump_false: then_block.len() + 2,
            }),
        });
        instructions.extend(then_block);
        instructions.push(Instruction {
            span: 0,
            control: InstructionType::Jump(JumpControl { 
                jump: else_block.len() + 1,
            }),
        });
        instructions.extend(else_block);

        instructions
    }
}

impl AstDisplay for IfControl {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}if_control")?;

        let prefix = prefix.add_branch();
        writeln!(f, "{prefix}condition")?;
        {
            let prefix = prefix.add_leaf();
            self.condition.ast_fmt(f, &prefix)?;
        }
        if let Some(else_block) = &self.else_block {
            writeln!(f, "{prefix}then")?;
            {
                let prefix = prefix.add_leaf();
                self.then_block.ast_fmt(f, &prefix)?;
            }

            let prefix = prefix.switch();
            writeln!(f, "{prefix}else")?;
            {
                let prefix = prefix.add_leaf();
                else_block.ast_fmt(f, &prefix)?;
            }
        } else {
            let prefix = prefix.switch();
            writeln!(f, "{prefix}then")?;
            {
                let prefix = prefix.add_leaf();
                self.then_block.ast_fmt(f, &prefix)?;
            }
        }

        Ok(())
    }
}
