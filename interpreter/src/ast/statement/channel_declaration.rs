use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::{
            datatype::{self, DataType}, declaration_keyword::DeclarationKeyword, identifier::Identifier,
            literal::Literal,
        },
    }, compiler::{CompilerState, Variable}, error::{AlthreadError, AlthreadResult, ErrorType}, no_rule, parser::Rule, vm::instruction::{DeclarationControl, Instruction, InstructionType}
};

use super::expression::Expression;

#[derive(Debug, Clone)]
pub struct ChannelDeclaration {
    pub ch_left_prog: String,
    pub ch_left_name: String,
    pub ch_right_prog: String,
    pub ch_right_name: String,
    pub datatypes: Vec<DataType>,
    // todo: direction
}

impl NodeBuilder for ChannelDeclaration {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {

        let mut left_pairs = pairs.next().unwrap().into_inner();
        let left_prog = String::from(left_pairs.next().unwrap().as_str());
        let left_name = String::from(left_pairs.next().unwrap().as_str());

        // TODO parse types
        pairs.next();
        let mut datatypes: Vec<DataType> = vec![DataType::Integer];

        let mut right_pairs = pairs.next().unwrap().into_inner();
        let right_prog = String::from(right_pairs.next().unwrap().as_str());
        let right_name = String::from(right_pairs.next().unwrap().as_str());

        
        Ok(Self {
            ch_left_prog: left_prog,
            ch_left_name: left_name,
            ch_right_prog: right_prog,
            ch_right_name: right_name,
            datatypes,
        })
    }
}


impl InstructionBuilder for ChannelDeclaration {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {
        let mut instructions = Vec::new();
        
        Ok(instructions)
    }
}

impl AstDisplay for ChannelDeclaration {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}channel decl")?;

        Ok(())
    }
}
