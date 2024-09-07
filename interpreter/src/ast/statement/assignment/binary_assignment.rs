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

#[derive(Debug)]
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



impl InstructionBuilder for BinaryAssignment {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<Vec<Instruction>> {
        let mut instructions = Vec::new();
        
        state.current_stack_depth += 1;
        instructions.append(&mut self.value.compile(state)?);
        let unstack_len = state.unstack_current_depth();

        if let Some(_) = state.global_table.get(&self.identifier.value.value) {
            instructions.push(Instruction {
                line: self.identifier.line,
                column: self.identifier.column,
                control: InstructionType::GlobalAssignment(GlobalAssignmentControl{
                    identifier: self.identifier.value.value.clone(),
                    operator: self.operator.value.clone(),
                    unstack_len
                })
            });
        } else {
            let mut var_idx = 0;
            for var in state.program_stack.iter().rev() {
                if var.name == self.identifier.value.value {
                    break;
                }
                var_idx += 1;
            }
            instructions.push(Instruction {
                line: self.identifier.line,
                column: self.identifier.column,
                control: InstructionType::LocalAssignment(LocalAssignmentControl{
                    index: var_idx,
                    operator: self.operator.value.clone(),
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
