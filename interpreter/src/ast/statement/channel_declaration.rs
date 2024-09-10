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


impl InstructionBuilder for Node<ChannelDeclaration> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {
        let dec = &self.value;
        // check if a channel with the same name already exists on this program
        if let Some(datatypes) = state.channels.get(&(dec.ch_left_prog.clone(), dec.ch_left_name.clone())) {
            // check if the datatypes are the same
            if datatypes != &dec.datatypes {
                return Err(AlthreadError::new(
                    ErrorType::TypeError, 
                    Some(self.pos),
                    format!("Channel {} already attached with different types", dec.ch_left_name)));
            }
        }
        state.channels.insert((dec.ch_left_prog.clone(), dec.ch_left_name.clone()), dec.datatypes.clone());
        state.channels.insert((dec.ch_right_prog.clone(), dec.ch_right_name.clone()), dec.datatypes.clone());
        // No instructions because it's just a declaration
        Ok(vec![])
    }
}

impl AstDisplay for ChannelDeclaration {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}channel decl")?;

        Ok(())
    }
}
