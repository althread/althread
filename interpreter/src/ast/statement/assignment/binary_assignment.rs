use std::fmt::{self};

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        statement::expression::Expression,
        token::{
            binary_assignment_operator::BinaryAssignmentOperator, identifier::Identifier,
            literal::Literal,
        },
    }, compiler::CompilerState, error::{AlthreadError, AlthreadResult, ErrorType}, parser::Rule, vm::instruction::{GlobalAssignmentControl, Instruction, InstructionType, LocalAssignmentControl}
};

#[derive(Debug, Clone)]
pub struct BinaryAssignment {
    pub identifier: Node<Identifier>,
    pub operator: Node<BinaryAssignmentOperator>,
    pub value: Node<Expression>,
}

impl NodeBuilder for BinaryAssignment {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let identifier = Node::build(pairs.next().unwrap())?;
        let operator = Node::build(pairs.next().unwrap())?;
        let value = Node::build(pairs.next().unwrap())?;

        Ok(Self {
            identifier,
            operator,
            value,
        })
    }
}



impl InstructionBuilder for Node<BinaryAssignment> {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {
        let mut instructions = Vec::new();
        
        state.current_stack_depth += 1;
        instructions.append(&mut self.value.value.compile(state)?);
        let rdatatype = state.program_stack.last().expect("empty stack after expression").datatype.clone();
        let unstack_len = state.unstack_current_depth();

        if let Some(g_val) = state.global_table.get(&self.value.identifier.value.value) {
            if g_val.datatype != rdatatype {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos),
                    format!("Cannot assign value of type {} to variable of type {}", rdatatype, g_val.datatype)
                ))
            }
            if !g_val.mutable {
                return Err(AlthreadError::new(
                    ErrorType::VariableError,
                    Some(self.pos),
                    format!("Cannot assign value to the immutable global variable {}", self.value.identifier.value.value)
                ))
            }
            instructions.push(Instruction {
                pos: Some(self.value.identifier.pos),
                control: InstructionType::GlobalAssignment(GlobalAssignmentControl{
                    identifier: self.value.identifier.value.value.clone(),
                    operator: self.value.operator.value.clone(),
                    unstack_len
                })
            });
        } else {
            let mut var_idx = 0;
            let mut l_var = None;
            for var in state.program_stack.iter().rev() {
                if var.name == self.value.identifier.value.value {
                    l_var = Some(var);
                    break;
                }
                var_idx += 1;
            }
            if l_var.is_none() {
                return Err(AlthreadError::new(
                    ErrorType::VariableError,
                    Some(self.pos),
                    format!("Variable '{}' is undefined", self.value.identifier.value.value)
                )) 
            }
            let l_var = l_var.unwrap();
            if l_var.datatype != rdatatype {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos),
                    format!("Cannot assign value of type {} to variable of type {}", rdatatype, l_var.datatype)
                ))
            }
            if !l_var.mutable {
                return Err(AlthreadError::new(
                    ErrorType::VariableError,
                    Some(self.pos),
                    format!("Cannot assign value to the immutable local variable {}", self.value.identifier.value.value)
                ))
            }

            instructions.push(Instruction {
                pos: Some(self.value.identifier.pos),
                control: InstructionType::LocalAssignment(LocalAssignmentControl{
                    index: var_idx,
                    operator: self.value.operator.value.clone(),
                    unstack_len
                })
            });
        }

        Ok(instructions)
    }
}




impl AstDisplay for BinaryAssignment {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{}binary_assign", prefix)?;

        let prefix = prefix.add_branch();
        writeln!(f, "{}ident: {}", &prefix, self.identifier)?;
        writeln!(f, "{}op: {}", &prefix, self.operator)?;

        let prefix = prefix.switch();
        writeln!(f, "{}value:", &prefix)?;
        let prefix = prefix.add_leaf();
        self.value.ast_fmt(f, &prefix)?;
        Ok(())
    }
}
