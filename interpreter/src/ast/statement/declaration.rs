use std::fmt;

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node, NodeBuilder},
        token::{
            datatype::DataType, declaration_keyword::DeclarationKeyword,
            object_identifier::ObjectIdentifier,
        },
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType},
    no_rule,
    parser::Rule,
    vm::instruction::{Instruction, InstructionType},
};

use super::expression::SideEffectExpression;

#[derive(Debug, Clone)]
pub struct Declaration {
    pub keyword: Node<DeclarationKeyword>,
    pub identifier: Node<ObjectIdentifier>,
    pub datatype: Option<Node<DataType>>,
    pub value: Option<Node<SideEffectExpression>>,
}

impl NodeBuilder for Declaration {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let keyword = Node::build(pairs.next().unwrap(), filepath)?;
        let identifier = Node::build(pairs.next().unwrap(), filepath)?;
        let mut datatype = None;
        let mut value = None;

        for pair in pairs {
            match pair.as_rule() {
                Rule::datatype => {
                    datatype = Some(Node::build(pair, filepath)?);
                }
                Rule::side_effect_expression => {
                    value = Some(Node::build(pair, filepath)?);
                }
                _ => return Err(no_rule!(pair, "declaration", filepath)),
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
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();
        let mut datatype = None;

        let full_var_name = self
            .identifier
            .value
            .parts
            .iter()
            .map(|p| p.value.value.as_str())
            .collect::<Vec<_>>()
            .join(".");

        // For declarations, we should only allow simple identifiers (single part)
        // Qualified identifiers like "fibo.N" should not be declared, only assigned to
        if self.identifier.value.parts.len() > 1 {
            return Err(AlthreadError::new(
                ErrorType::VariableError,
                Some(self.identifier.pos.clone()),
                format!("Cannot declare qualified variable '{}'. Use simple identifiers for declarations.", full_var_name),
            ));
        }

        // Get the simple variable name (first and only part)
        let var_name = &self.identifier.value.parts[0].value.value;

        if state.global_table().contains_key(&full_var_name) {
            return Err(AlthreadError::new(
                ErrorType::VariableError,
                Some(self.identifier.pos.clone()),
                format!("Variable {} already declared", full_var_name),
            ));
        }

        // Check if the variable starts with a capital letter (reserved for shared variables)
        if var_name.chars().next().unwrap().is_uppercase() {
            if !state.is_shared {
                return Err(AlthreadError::new(
                    ErrorType::VariableError,
                    Some(self.identifier.pos.clone()),
                    format!("Variable {} starts with a capital letter, which is reserved for shared variables", var_name)
                ));
            }
        } else {
            if state.is_shared && !state.in_function {
                return Err(AlthreadError::new(
                    ErrorType::VariableError,
                    Some(self.identifier.pos.clone()),
                    format!("Variable {} does not start with a capital letter, which is mandatory for shared variables", var_name)
                ));
            }
        }

        if let Some(d) = &self.datatype {
            datatype = Some(d.value.clone());
        }

        if let Some(value) = &self.value {
            state.current_stack_depth += 1;
            builder.extend(value.compile(state)?);
            let computed_datatype = state
                .program_stack
                .last()
                .expect("Error: Program stack is empty after compiling an expression")
                .datatype
                .clone();
            let unstack_len = state.unstack_current_depth();

            if let Some(declared_datatype) = datatype {
                // Special case: allow assignment of empty list (list(void)) to typed list
                let types_compatible = if let (DataType::List(declared_elem), DataType::List(computed_elem)) = (&declared_datatype, &computed_datatype) {
                    // Allow list(void) to be assigned to list(T) for any T (empty list case)
                    **computed_elem == DataType::Void || declared_elem == computed_elem
                } else {
                    declared_datatype == computed_datatype
                };

                if !types_compatible {
                    return Err(AlthreadError::new(
                        ErrorType::TypeError,
                        Some(self.datatype.as_ref().unwrap().pos.clone()),
                        format!(
                            "Declared type and assignment do not match (found :{} = {})",
                            declared_datatype, computed_datatype
                        ),
                    ));
                }
                
                // For empty list assignment, add conversion instruction
                if let (DataType::List(declared_elem), DataType::List(computed_elem)) = (&declared_datatype, &computed_datatype) {
                    if **computed_elem == DataType::Void {
                        // Add instruction to convert empty list to declared type
                        builder.instructions.push(Instruction {
                            control: InstructionType::ConvertEmptyListType {
                                to_element_type: (**declared_elem).clone(),
                            },
                            pos: Some(self.keyword.pos.clone()),
                        });
                        // Use declared type for the variable
                        datatype = Some(declared_datatype);
                    } else {
                        datatype = Some(computed_datatype);
                    }
                } else {
                    datatype = Some(computed_datatype);
                }
            } else {
                datatype = Some(computed_datatype);
            }

            builder.instructions.push(Instruction {
                control: InstructionType::Declaration { unstack_len },
                pos: Some(self.keyword.pos.clone()),
            });
        } else {
            if datatype.is_none() {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(self.identifier.pos.clone()),
                    "Declaration must have a datatype or a value".to_string(),
                ));
            }
            builder.instructions.push(Instruction {
                control: InstructionType::Push(datatype.as_ref().unwrap().default()),
                pos: Some(self.keyword.pos.clone()),
            });
        }

        let datatype = datatype.unwrap();

        state.program_stack.push(Variable {
            mutable: self.keyword.value == DeclarationKeyword::Let,
            name: var_name.clone(), // Use the simple variable name, not the full qualified name
            datatype,
            depth: state.current_stack_depth,
            declare_pos: Some(self.identifier.pos.clone()),
        });

        Ok(builder)
    }
}

impl AstDisplay for Declaration {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}decl")?;

        let prefix = &prefix.add_branch();
        writeln!(f, "{prefix}keyword: {}", self.keyword)?;

        // Get the display name for the identifier
        let identifier_name = self
            .identifier
            .value
            .parts
            .iter()
            .map(|p| p.value.value.as_str())
            .collect::<Vec<_>>()
            .join(".");

        match (&self.datatype, &self.value) {
            (Some(datatype), Some(value)) => {
                writeln!(f, "{prefix}ident: {}", identifier_name)?;
                writeln!(f, "{prefix}datatype: {datatype}")?;
                let prefix = prefix.switch();
                writeln!(f, "{prefix}value")?;
                value.ast_fmt(f, &prefix.add_leaf())?;
            }
            (Some(datatype), None) => {
                writeln!(f, "{prefix}ident: {}", identifier_name)?;
                let prefix = prefix.switch();
                writeln!(f, "{prefix}datatype: {datatype}")?;
            }
            (None, Some(value)) => {
                writeln!(f, "{prefix}ident: {}", identifier_name)?;
                let prefix = prefix.switch();
                writeln!(f, "{prefix}value")?;
                value.ast_fmt(f, &prefix.add_leaf())?;
            }
            (None, None) => {
                let prefix = prefix.switch();
                writeln!(f, "{prefix}ident: {}", identifier_name)?;
            }
        }

        Ok(())
    }
}
