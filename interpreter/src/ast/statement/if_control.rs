use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        block::Block, display::{AstDisplay, Prefix}, node::{InstructionBuilder, Node, NodeBuilder}, statement::Statement, token::datatype::DataType
    }, compiler::{CompilerState, InstructionBuilderOk}, error::{AlthreadError, AlthreadResult, ErrorType}, no_rule, parser::Rule, vm::instruction::{Instruction, InstructionType}
};

use super::expression::Expression;

#[derive(Debug, Clone)]
pub struct IfControl {
    pub condition: Node<Expression>,
    pub then_block: Box<Node<Block>>,
    pub else_block: Option<Box<Node<Block>>>,
}

impl NodeBuilder for IfControl {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let condition = Node::build(pairs.next().unwrap())?;
        let then_block = Node::build(pairs.next().unwrap())?;
        
        // The else block is optional and could be
        // a if statement, in this case we need to wrap it in a block node
        let else_block = match pairs.next() {
            Some(else_block_pair) => match else_block_pair.as_rule() {
                Rule::if_control => {
                    // wrape the if controle in a block node
                    let if_statement: Node<IfControl> = Node::build(else_block_pair)?;
                    let common_position= if_statement.pos.clone();
                    let v = vec![Node { 
                        pos: common_position.clone(), 
                        value: Statement::If(if_statement)
                    }];
                    Some(Node {
                        pos: common_position,
                        value: Block {
                            children: v,
                        }
                    })
                }
                Rule::code_block => {
                    Some(Node::build(else_block_pair)?)
                }
                _ => return Err(no_rule!(else_block_pair, "For else expression")),
            },
            None => None,
        };


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

impl InstructionBuilder for IfControl {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();
        state.current_stack_depth += 1;

        let condition = self.condition.compile(state)?;
        // Check if the top of the stack is a boolean
        if state
            .program_stack
            .last()
            .expect("stack should contain a value after an expression is compiled")
            .datatype
            != DataType::Boolean
        {
            return Err(AlthreadError::new(
                ErrorType::TypeError,
                Some(self.condition.pos),
                "if condition must be a boolean".to_string(),
            ));
        }
        // pop all variables from the stack at the given depth
        let unstack_len = state.unstack_current_depth();

        let then_block = self.then_block.compile(state)?;

        let else_block = match self.else_block.as_ref() {
            Some(block) => block.compile(state)?,
            None => InstructionBuilderOk::new(),
        };

        builder.extend(condition);
        builder.instructions.push(Instruction {
            pos: Some(self.condition.pos),
            control: InstructionType::JumpIf {
                jump_false: (then_block.instructions.len() + 2) as i64,
                unstack_len,
            },
        });
        builder.extend(then_block);
        if let Some(else_node) = &self.else_block {
            builder.instructions.push(Instruction {
                pos: Some(else_node.pos),
                control: InstructionType::Jump((else_block.instructions.len() + 1) as i64),
            });
            builder.extend(else_block);
        } else {
            builder.instructions.push(Instruction {
                pos: Some(self.then_block.pos),
                control: InstructionType::Empty,
            });
        }

        Ok(builder)
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
