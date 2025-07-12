use std::fmt::{self};

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        statement::expression::SideEffectExpression,
        token::{binary_assignment_operator::BinaryAssignmentOperator, object_identifier::ObjectIdentifier},
    },
    compiler::{CompilerState, InstructionBuilderOk},
    error::{AlthreadError, AlthreadResult, ErrorType},
    parser::Rule,
    vm::instruction::{Instruction, InstructionType},
};

#[derive(Debug, Clone)]
pub struct BinaryAssignment {
    pub identifier: Node<ObjectIdentifier>,
    pub operator: Node<BinaryAssignmentOperator>,
    pub value: Node<SideEffectExpression>,
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
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();

        // Get the full variable name (e.g., "fibo.N")
        let full_var_name = self.value.identifier
            .value
            .parts
            .iter()
            .map(|p| p.value.value.as_str())
            .collect::<Vec<_>>()
            .join(".");

        state.current_stack_depth += 1;
        builder.extend(self.value.value.compile(state)?);
        let rdatatype = state
            .program_stack
            .last()
            .expect("empty stack after expression")
            .datatype
            .clone();
        let unstack_len = state.unstack_current_depth();

        if let Some(g_val) = state.global_table().get(&full_var_name) {
            if g_val.datatype != rdatatype {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos),
                    format!(
                        "Cannot assign value of type {} to variable of type {}",
                        rdatatype, g_val.datatype
                    ),
                ));
            }
            if !g_val.mutable {
                return Err(AlthreadError::new(
                    ErrorType::VariableError,
                    Some(self.pos),
                    format!(
                        "Cannot assign value to the immutable global variable {}",
                        full_var_name
                    ),
                ));
            }
            builder.instructions.push(Instruction {
                pos: Some(self.value.identifier.pos),
                control: InstructionType::GlobalAssignment {
                    identifier: full_var_name,
                    operator: self.value.operator.value.clone(),
                    unstack_len,
                },
            });
        } else {
            let mut var_idx = 0;
            let mut l_var = None;
            for var in state.program_stack.iter().rev() {
                if var.name == full_var_name {
                    l_var = Some(var);
                    break;
                }
                var_idx += 1;
            }
            if l_var.is_none() {
                return Err(AlthreadError::new(
                    ErrorType::VariableError,
                    Some(self.pos),
                    format!(
                        "Variable '{}' is undefined",
                        full_var_name
                    ),
                ));
            }
            let l_var = l_var.unwrap();
            if l_var.datatype != rdatatype {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.pos),
                    format!(
                        "Cannot assign value of type {} to variable of type {}",
                        rdatatype, l_var.datatype
                    ),
                ));
            }
            if !l_var.mutable {
                return Err(AlthreadError::new(
                    ErrorType::VariableError,
                    Some(self.pos),
                    format!(
                        "Cannot assign value to the immutable local variable {}",
                        full_var_name
                    ),
                ));
            }

            builder.instructions.push(Instruction {
                pos: Some(self.value.identifier.pos),
                control: InstructionType::LocalAssignment {
                    index: var_idx,
                    operator: self.value.operator.value.clone(),
                    unstack_len,
                },
            });
        }

        Ok(builder)
    }
}

impl AstDisplay for BinaryAssignment {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{}binary_assign", prefix)?;

        let prefix = prefix.add_branch();
        let full_name = self.identifier
            .value
            .parts
            .iter()
            .map(|p| p.value.value.as_str())
            .collect::<Vec<_>>()
            .join(".");
        writeln!(f, "{}ident: {}", &prefix, full_name)?;
        writeln!(f, "{}op: {}", &prefix, self.operator)?;

        let prefix = prefix.switch();
        writeln!(f, "{}value:", &prefix)?;
        let prefix = prefix.add_leaf();
        self.value.ast_fmt(f, &prefix)?;
        Ok(())
    }
}
