pub mod assignment;
pub mod atomic;
pub mod break_loop;
pub mod channel_declaration;
pub mod declaration;
pub mod expression;
pub mod fn_call;
pub mod fn_return;
pub mod for_control;
pub mod if_control;
pub mod label;
pub mod loop_control;
pub mod receive;
pub mod run_call;
pub mod send;
pub mod wait;
pub mod waiting_case;
pub mod while_control;

use std::fmt;

use assignment::Assignment;
use break_loop::BreakLoopControl;
use channel_declaration::ChannelDeclaration;
use declaration::Declaration;
use fn_call::FnCall;
use fn_return::FnReturn;
use for_control::ForControl;
use if_control::IfControl;
use label::LabelStatement;
use loop_control::LoopControl;
use run_call::RunCall;
use send::SendStatement;
use wait::Wait;
use while_control::WhileControl;

use crate::{
    compiler::{CompilerState, InstructionBuilderOk},
    error::AlthreadResult,
    vm::instruction::{Instruction, InstructionType},
};

use super::{
    block::Block,
    display::{AstDisplay, Prefix},
    node::{InstructionBuilder, Node},
};

#[derive(Debug, Clone)]
pub enum Statement {
    Assignment(Node<Assignment>),
    Declaration(Node<Declaration>),
    Send(Node<SendStatement>),
    ChannelDeclaration(Node<ChannelDeclaration>),
    Run(Node<RunCall>),
    FnCall(Node<FnCall>),
    FnReturn(Node<FnReturn>),
    If(Node<IfControl>),
    While(Node<WhileControl>),
    Loop(Node<LoopControl>),
    For(Node<ForControl>),
    BreakLoop(Node<BreakLoopControl>),
    Label(Node<LabelStatement>),
    Atomic(Node<atomic::Atomic>),
    Wait(Node<Wait>),
    Block(Node<Block>),
}

impl InstructionBuilder for Statement {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        match self {
            Self::If(node) => node.compile(state),
            Self::Assignment(node) => node.compile(state),
            Self::Declaration(node) => node.compile(state),
            Self::ChannelDeclaration(node) => node.compile(state),
            Self::While(node) => node.compile(state),
            Self::Loop(node) => node.compile(state),
            Self::For(node) => node.compile(state),
            Self::Atomic(node) => node.compile(state),
            Self::Wait(node) => node.compile(state),
            Self::Block(node) => node.compile(state),
            Self::Send(node) => node.compile(state),
            Self::BreakLoop(node) => node.compile(state),
            Self::Label(node) => node.compile(state),
            Self::Run(node) => {
                // a run call returns a value, so we have to ustack it
                let mut builder = node.compile(state)?;
                builder.instructions.push(Instruction {
                    pos: Some(node.pos.clone()),
                    control: InstructionType::Unstack { unstack_len: 1 },
                });
                state.program_stack.pop();
                Ok(builder)
            }
            Self::FnCall(node) => {
                let mut builder = node.compile(state)?;
                builder.instructions.push(Instruction {
                    pos: Some(node.pos.clone()),
                    control: InstructionType::Unstack { unstack_len: 1 },
                });
                state.program_stack.pop();
                Ok(builder)
            }
            Self::FnReturn(node) => node.compile(state),
        }
    }
}

impl Statement {
    pub fn is_atomic(&self) -> bool {
        todo!("Check this implementation");
    }
}

impl AstDisplay for Statement {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        match self {
            Statement::Assignment(node) => node.ast_fmt(f, prefix),
            Statement::Declaration(node) => node.ast_fmt(f, prefix),
            Statement::Send(node) => node.ast_fmt(f, prefix),
            Statement::ChannelDeclaration(node) => node.ast_fmt(f, prefix),
            Statement::Wait(node) => node.ast_fmt(f, prefix),
            Statement::FnCall(node) => node.ast_fmt(f, prefix),
            Statement::FnReturn(node) => node.ast_fmt(f, prefix),
            Statement::Run(node) => node.ast_fmt(f, prefix),
            Statement::If(node) => node.ast_fmt(f, prefix),
            Statement::While(node) => node.ast_fmt(f, prefix),
            Statement::Loop(node) => node.ast_fmt(f, prefix),
            Statement::For(node) => node.ast_fmt(f, prefix),
            Statement::BreakLoop(node) => node.ast_fmt(f, prefix),
            Statement::Label(node) => node.ast_fmt(f, prefix),
            Statement::Atomic(node) => node.ast_fmt(f, prefix),
            Statement::Block(node) => node.ast_fmt(f, prefix),
        }
    }
}
