use std::fmt::{self, Debug};

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{InstructionBuilder, Node},
        token::{
            datatype::DataType,
            declaration_keyword::DeclarationKeyword,
            identifier::Identifier,
            tuple_identifier::{Lvalue, TupleIdentifier},
        },
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType, Pos},
    vm::instruction::{Instruction, InstructionType},
};

use super::expression::Expression;

#[derive(Debug, Clone)]
pub struct Declaration {
    pub keyword: Node<DeclarationKeyword>,
    pub identifier: Lvalue,
    pub datatype: Option<Node<DataType>>,
    pub value: Option<Node<Expression>>,
}

fn push_default_for_binding(
    declaration: &Declaration,
    builder: &mut InstructionBuilderOk,
    datatype: &DataType,
) {
    builder.instructions.push(Instruction {
        control: InstructionType::Push(datatype.default()),
        pos: Some(declaration.keyword.pos.clone()),
    });
}

fn register_binding(
    state: &mut CompilerState,
    builder: &mut InstructionBuilderOk,
    name: String,
    datatype: DataType,
    mutable: bool,
    declare_pos: Pos,
    stack_index: usize,
    scope_start_ip: usize,
) {
    state.program_stack.push(Variable {
        mutable,
        name: name.clone(),
        datatype: datatype.clone(),
        depth: state.current_stack_depth,
        declare_pos: Some(declare_pos.clone()),
    });
    builder
        .debug_variables
        .push(crate::compiler::LocalVariableDebugInfo {
            name,
            datatype,
            stack_index,
            scope_start_ip,
            scope_end_ip: None,
            declare_pos: Some(declare_pos),
        });
}

fn validate_identifier_rules(
    state: &CompilerState,
    declaration: &Declaration,
    node: &Node<Identifier>,
) -> AlthreadResult<()> {
    let var_name = &node.value.value;

    if state.global_table().contains_key(var_name) {
        return Err(AlthreadError::new(
            ErrorType::VariableError,
            Some(node.pos.clone()),
            format!("Variable {} already declared", var_name),
        ));
    }

    if var_name.chars().next().unwrap().is_uppercase() {
        if !state.is_shared {
            return Err(AlthreadError::new(
                ErrorType::VariableError,
                Some(node.pos.clone()),
                format!(
                    "Variable {} starts with a capital letter, which is reserved for shared variables",
                    var_name
                ),
            ));
        }
    } else if state.is_shared && !state.in_function {
        return Err(AlthreadError::new(
            ErrorType::VariableError,
            Some(node.pos.clone()),
            format!(
                "Variable {} does not start with a capital letter, which is mandatory for shared variables",
                var_name
            ),
        ));
    }

    if declaration.keyword.value == DeclarationKeyword::Const && declaration.value.is_none() {
        return Err(AlthreadError::new(
            ErrorType::TypeError,
            Some(node.pos.clone()),
            "Const declarations must have a value".to_string(),
        ));
    }

    Ok(())
}

fn emit_tuple_destructure(
    builder: &mut InstructionBuilderOk,
    tuple_offset: usize,
    pos: &Pos,
) {
    builder.instructions.push(Instruction {
        control: InstructionType::DestructureTuple { tuple_offset },
        pos: Some(pos.clone()),
    });
}

fn register_identifier_binding(
    declaration: &Declaration,
    state: &mut CompilerState,
    node: &Node<Identifier>,
    builder: &mut InstructionBuilderOk,
    datatype: DataType,
    stack_index: usize,
    scope_start_ip: usize,
    has_runtime_source: bool,
) -> AlthreadResult<()> {
    validate_identifier_rules(state, declaration, node)?;

    if !has_runtime_source {
        push_default_for_binding(declaration, builder, &datatype);
    }

    register_binding(
        state,
        builder,
        node.value.value.clone(),
        datatype,
        declaration.keyword.value == DeclarationKeyword::Let,
        node.pos.clone(),
        stack_index,
        scope_start_ip,
    );
    Ok(())
}

fn register_ignored_binding(
    declaration: &Declaration,
    state: &mut CompilerState,
    pos: &Pos,
    builder: &mut InstructionBuilderOk,
    datatype: DataType,
    stack_index: usize,
    scope_start_ip: usize,
    has_runtime_source: bool,
) {
    if !has_runtime_source {
        push_default_for_binding(declaration, builder, &datatype);
    }

    register_binding(
        state,
        builder,
        "_".to_string(),
        datatype,
        false,
        pos.clone(),
        stack_index,
        scope_start_ip,
    );
}

fn resolve_declared_datatype(
    declaration: &Declaration,
    state: &mut CompilerState,
    builder: &mut InstructionBuilderOk,
    error_pos: &Pos,
) -> AlthreadResult<(DataType, bool)> {
    let mut datatype = declaration.datatype.as_ref().map(|node| node.value.clone());

    if let Some(value) = &declaration.value {
        state.current_stack_depth += 1;
        builder.extend(value.compile(state)?);

        let computed_datatype = state
            .program_stack
            .last()
            .expect("Program stack should contain the compiled declaration value")
            .datatype
            .clone();
        let unstack_len = state.unstack_current_depth();

        if let Some(declared_datatype) = datatype.clone() {
            let types_compatible =
                if let (DataType::List(declared_elem), DataType::List(computed_elem)) =
                    (&declared_datatype, &computed_datatype)
                {
                    **computed_elem == DataType::Void || declared_elem == computed_elem
                } else {
                    declared_datatype == computed_datatype
                };

            if !types_compatible {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    declaration.datatype.as_ref().map(|node| node.pos.clone()),
                    format!(
                        "Declared type and assignment do not match (found :{} = {})",
                        declared_datatype, computed_datatype
                    ),
                ));
            }

            if let (DataType::List(declared_elem), DataType::List(computed_elem)) =
                (&declared_datatype, &computed_datatype)
            {
                if **computed_elem == DataType::Void {
                    builder.instructions.push(Instruction {
                        control: InstructionType::ConvertEmptyListType {
                            to_element_type: (**declared_elem).clone(),
                        },
                        pos: Some(declaration.keyword.pos.clone()),
                    });
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
            pos: Some(declaration.keyword.pos.clone()),
        });

        return Ok((datatype.expect("datatype should be inferred"), true));
    }

    let Some(datatype) = datatype else {
        return Err(AlthreadError::new(
            ErrorType::TypeError,
            Some(error_pos.clone()),
            "Declaration must have a datatype or a value".to_string(),
        ));
    };

    Ok((datatype, false))
}

fn bind_tuple_pattern(
    declaration: &Declaration,
    state: &mut CompilerState,
    node: &Node<TupleIdentifier>,
    builder: &mut InstructionBuilderOk,
    datatype: DataType,
    stack_index: usize,
    scope_start_ip: usize,
    has_runtime_source: bool,
    runtime_tuple_offset: usize,
) -> AlthreadResult<()> {
    if node.value.value.len() < 2 {
        return Err(AlthreadError::new(
            ErrorType::VariableError,
            Some(node.pos.clone()),
            "A tuple pattern must contain at least two elements".to_string(),
        ));
    }

    let DataType::Tuple(item_types) = datatype else {
        return Err(AlthreadError::new(
            ErrorType::VariableError,
            Some(node.pos.clone()),
            format!(
                "Cannot destructure {} into a tuple pattern of size {}",
                datatype,
                node.value.value.len()
            ),
        ));
    };

    if item_types.len() != node.value.value.len() {
        return Err(AlthreadError::new(
            ErrorType::VariableError,
            Some(node.pos.clone()),
            format!(
                "Tuple pattern expects {} elements but the value has {}",
                node.value.value.len(),
                item_types.len()
            ),
        ));
    }

    if has_runtime_source {
        emit_tuple_destructure(builder, runtime_tuple_offset, &node.pos);
    }

    let tuple_len = item_types.len();
    for (index, pattern) in node.value.value.iter().enumerate() {
        let item_type = item_types[index].clone();
        let child_runtime_offset = runtime_tuple_offset + (tuple_len - 1 - index);

        match pattern.as_ref() {
            Lvalue::Identifier(identifier) => register_identifier_binding(
                declaration,
                state,
                identifier,
                builder,
                item_type,
                stack_index,
                scope_start_ip,
                has_runtime_source,
            )?,
            Lvalue::NullIdentifier(null_identifier) => register_ignored_binding(
                declaration,
                state,
                &null_identifier.pos,
                builder,
                item_type,
                stack_index,
                scope_start_ip,
                has_runtime_source,
            ),
            Lvalue::TupleIdentifier(tuple_pattern) => bind_tuple_pattern(
                declaration,
                state,
                tuple_pattern,
                builder,
                item_type,
                stack_index,
                scope_start_ip,
                has_runtime_source,
                child_runtime_offset,
            )?,
        }
    }

    Ok(())
}

impl InstructionBuilder for Declaration {
    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();

        match &self.identifier {
            Lvalue::Identifier(node) => {
                let (datatype, has_runtime_source) =
                    resolve_declared_datatype(self, state, &mut builder, &node.pos)?;
                let stack_index = state.program_stack.len();
                let scope_start_ip = builder.instructions.len();
                register_identifier_binding(
                    self,
                    state,
                    node,
                    &mut builder,
                    datatype,
                    stack_index,
                    scope_start_ip,
                    has_runtime_source,
                )?;
            }
            Lvalue::TupleIdentifier(node) => {
                let (datatype, has_runtime_source) =
                    resolve_declared_datatype(self, state, &mut builder, &node.pos)?;
                let stack_index = state.program_stack.len();
                let scope_start_ip = builder.instructions.len();
                bind_tuple_pattern(
                    self,
                    state,
                    node,
                    &mut builder,
                    datatype,
                    stack_index,
                    scope_start_ip,
                    has_runtime_source,
                    0,
                )?;
            }
            Lvalue::NullIdentifier(node) => {
                return Err(AlthreadError::new(
                    ErrorType::TypeError,
                    Some(node.pos.clone()),
                    "Declaration of variable cannot be a standalone '_'".to_string(),
                ));
            }
        }

        Ok(builder)
    }
}

impl AstDisplay for Declaration {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}decl")?;

        let prefix = &prefix.add_branch();
        writeln!(f, "{prefix}keyword: {}", self.keyword)?;
        match &self.identifier {
            Lvalue::Identifier(node) => {
                let identifier_name = &node.value.value;

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
            }
            Lvalue::TupleIdentifier(node) => {
                writeln!(f, "{prefix}ident:")?;
                node.ast_fmt(f, &prefix.add_leaf())?;
                match (&self.datatype, &self.value) {
                    (Some(datatype), Some(value)) => {
                        writeln!(f, "{prefix}datatype: ")?;
                        let p1 = &prefix.add_leaf();
                        match &datatype.value {
                            DataType::Tuple(v) => {
                                writeln!(f, "{p1}tuple: ")?;
                                let p = &p1.add_leaf();
                                for item in v {
                                    write!(f, "{p}datatype: ")?;
                                    item.fmt(f)?;
                                    writeln!(f, "")?;
                                }
                            }
                            _ => {
                                writeln!(f, "{prefix}datatype: {datatype}")?;
                            }
                        }
                        let prefix = prefix.switch();
                        writeln!(f, "{prefix}value")?;
                        value.ast_fmt(f, &prefix.add_leaf())?;
                    }
                    (Some(datatype), None) => {
                        writeln!(f, "{prefix}datatype: ")?;
                        let p1 = &prefix.add_leaf();
                        match &datatype.value {
                            DataType::Tuple(v) => {
                                writeln!(f, "{p1}tuple: ")?;
                                let p = &p1.add_leaf();
                                for item in v {
                                    write!(f, "{p}datatype: ")?;
                                    item.fmt(f)?;
                                    writeln!(f, "")?;
                                }
                            }
                            _ => {
                                writeln!(f, "{prefix}datatype: {datatype}")?;
                            }
                        }
                    }
                    (None, Some(value)) => {
                        let prefix = prefix.switch();
                        writeln!(f, "{prefix}value")?;
                        value.ast_fmt(f, &prefix.add_leaf())?;
                    }
                    (None, None) => {}
                }
            }
            Lvalue::NullIdentifier(_node) => {}
        }
        Ok(())
    }
}
