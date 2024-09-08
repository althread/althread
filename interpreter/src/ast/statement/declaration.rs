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
pub struct Declaration {
    pub keyword: Node<DeclarationKeyword>,
    pub identifier: Node<Identifier>,
    pub datatype: Option<Node<DataType>>,
    pub value: Option<Node<Expression>>,
}

impl NodeBuilder for Declaration {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let keyword = Node::build(pairs.next().unwrap())?;
        let identifier = Node::build(pairs.next().unwrap())?;
        let mut datatype = None;
        let mut value = None;

        for pair in pairs {
            match pair.as_rule() {
                Rule::datatype => {
                    datatype = Some(Node::build(pair)?);
                }
                Rule::expression => {
                    value = Some(Node::build(pair)?);
                }
                _ => return Err(no_rule!(pair)),
            }
        }

        Ok(Self {
            keyword,
            identifier,
            datatype,
            value,
        })
    }
}


impl InstructionBuilder for Declaration {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {
        let mut instructions = Vec::new();
        let mut datatype = None;

        if let Some(d) = &self.datatype {
            datatype = Some(d.value.clone());
        }
        if let Some(value) = &self.value {
            state.current_stack_depth += 1;
            instructions.extend(value.compile(state)?);
            let computed_datatype = state.program_stack.last().expect("Error: Program stack is empty after compiling an expression").datatype.clone();
            let unstack_len = state.unstack_current_depth();

            if let Some(datatype) = datatype {
                if datatype != computed_datatype {
                    return Err(AlthreadError::new(
                        ErrorType::TypeError,
                        Some(self.datatype.as_ref().unwrap().pos),
                        format!("Declared type and assignment do not match (found :{} = {})", datatype, computed_datatype)
                    ))
                }
            }
            datatype = Some(computed_datatype);

            instructions.push(Instruction {
                control: InstructionType::Declaration(DeclarationControl{
                    unstack_len
                }),
                pos: Some(self.keyword.pos),
            });
        } else {
            if datatype.is_none() {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.identifier.pos),
                    "Declaration must have a datatype or a value".to_string()
                ));
            }
            instructions.push(Instruction {
                control: InstructionType::PushNull(datatype.as_ref().unwrap().clone()),
                pos: Some(self.keyword.pos),
            });
        } 

        let datatype = datatype.unwrap();
        
        state.program_stack.push(Variable {
            mutable: self.keyword.value == DeclarationKeyword::Let,
            name: self.identifier.value.value.clone(),
            datatype,
            depth: state.current_stack_depth
        });

        Ok(instructions)
    }
}

impl AstDisplay for Declaration {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}decl")?;

        let prefix = &prefix.add_branch();
        writeln!(f, "{prefix}keyword: {}", self.keyword)?;

        match (&self.datatype, &self.value) {
            (Some(datatype), Some(value)) => {
                writeln!(f, "{prefix}ident: {}", self.identifier)?;
                writeln!(f, "{prefix}datatype: {datatype}")?;
                let prefix = prefix.switch();
                writeln!(f, "{prefix}value")?;
                value.ast_fmt(f, &prefix.add_leaf())?;
            }
            (Some(datatype), None) => {
                writeln!(f, "{prefix}ident: {}", self.identifier)?;
                let prefix = prefix.switch();
                writeln!(f, "{prefix}datatype: {datatype}")?;
            }
            (None, Some(value)) => {
                writeln!(f, "{prefix}ident: {}", self.identifier)?;
                let prefix = prefix.switch();
                writeln!(f, "{prefix}value")?;
                value.ast_fmt(f, &prefix.add_leaf())?;
            }
            (None, None) => {
                let prefix = prefix.switch();
                writeln!(f, "{prefix}ident: {}", self.identifier)?;
            }
        }

        Ok(())
    }
}
