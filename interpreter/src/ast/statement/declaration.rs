use std::fmt::{self, Debug};

use pest::iterators::Pairs;

use crate::{
    ast::{
        display::{AstDisplay, Prefix}, 
        node::{InstructionBuilder, Node, NodeBuilder}, 
        statement::{declaration, expression::tuple_expression::TupleExpression}, 
        token::{
            self, datatype::{self, DataType}, declaration_keyword::DeclarationKeyword, identifier::{self, Identifier}, object_identifier::ObjectIdentifier, tuple_identifier::{self, Lvalue, TupleIdentifier}
        }
    },
    compiler::{CompilerState, InstructionBuilderOk, Variable},
    error::{AlthreadError, AlthreadResult, ErrorType,Pos},
    no_rule,
    parser::Rule,
    vm::instruction::{Instruction, InstructionType},
};
use super::expression::SideEffectExpression;

#[derive(Debug, Clone)]
pub struct Declaration {
    pub keyword: Node<DeclarationKeyword>,
    pub identifier: Lvalue,
    pub datatype: Option<Node<DataType>>,
    pub value: Option<Node<SideEffectExpression>>,
}

impl NodeBuilder for Declaration {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let keyword = Node::build(pairs.next().unwrap(), filepath)?;
        let identifier : Lvalue;
        let lvalue = pairs.next().unwrap(); 
        match lvalue.as_rule()  {
            Rule::identifier_tuple => {identifier = Lvalue::TupleIdentifier(Node::build(lvalue, filepath)?);}
            Rule::identifier => {
            //    identifier = Lvalue::Identifier(Node::build(lvalue, filepath)?);
                identifier = Lvalue::Identifier(Node::build(lvalue, filepath)?);
            }
            _ => {
                return Err(no_rule!(lvalue, "ImportBlock", filepath));
            }
        }
        let mut datatype = None;
        let mut value = None;
        print!("pas ouf du tout : \n\n -> id : {:?}\n\n -> pairs {:?} \n\n\n",identifier.clone(),pairs.clone().next());

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

fn compile_identifier(declaration : &Declaration , state: &mut CompilerState, node : &Node<Identifier>, builder : &mut InstructionBuilderOk, datatype : DataType ,stack_index : usize,scope_start_ip : usize) ->
    AlthreadResult<InstructionBuilderOk>
{
    let full_var_name = &node.value.value;
    // Get the simple variable name (first and only part)
    let var_name = &node.value.value;

    if state.global_table().contains_key(full_var_name) {
        return Err(AlthreadError::new(
            ErrorType::VariableError,
            Some(node.pos.clone()),
            format!("Variable {} already declared", full_var_name),
        ));
    }

    // Check if the variable starts with a capital letter (reserved for shared variables)
    if var_name.chars().next().unwrap().is_uppercase() {
        if !state.is_shared {
            return Err(AlthreadError::new(
                ErrorType::VariableError,
                Some(node.pos.clone()),
                format!("Variable {} starts with a capital letter, which is reserved for shared variables", var_name)
            ));
        }
    } else {
        if state.is_shared && !state.in_function {
            return Err(AlthreadError::new(
                ErrorType::VariableError,
                Some(node.pos.clone()),
                format!("Variable {} does not start with a capital letter, which is mandatory for shared variables", var_name)
            ));
        }
    }
    state.program_stack.push(Variable {
        mutable: true,
        name: node.value.value.clone(), // Use the simple variable name, not the full qualified name
        datatype: datatype.clone(),
        depth: state.current_stack_depth,
        declare_pos: Some(node.pos.clone()),
    });
    // builder.instructions.push(Instruction {
    //     control: InstructionType::Declaration { unstack_len },
    //     pos: Some(declaration.keyword.pos.clone()),
    // });
    Ok((*builder).clone())
}



fn compile_tupleidentifier(declaration : &Declaration , state: &mut CompilerState, node : &Node<TupleIdentifier>, builder : &mut InstructionBuilderOk, datatype : DataType ,stack_index : usize,scope_start_ip : usize) ->
    AlthreadResult<InstructionBuilderOk>
{
    
    match datatype {
        DataType::Tuple(v) => {
            
            let vec = node.value.value.clone();
            if v.len() != vec.len()
            {
                return Err(AlthreadError::new(
                    ErrorType::VariableError,
                    Some(node.pos.clone()),
                    format!("Tuple not well desfined")
                ));
            }
            let mut veciter = vec.iter().enumerate();
            if let Some(value) = &declaration.value {
                print!("Est un SideEffectExpression\n");
                state.current_stack_depth += 1;
                builder.extend(value.compile(state)?);
            }
            print!("Value ! : \n");
            while let Some((i,elt)) = veciter.next()
            {
                let value : Lvalue = (*(*elt).clone()).into();
                let r: Result<InstructionBuilderOk, AlthreadError>;
                match value {
                    Lvalue::Identifier(node) => {
                        r = compile_identifier(&declaration, state, &node,builder,v[i].clone(),stack_index,scope_start_ip);
                        
                        let r : Option<DataType> = std::option::Option::Some(v[i].clone());
                        builder.instructions.push(Instruction {
                            control: InstructionType::Push(r.as_ref().unwrap().default()),
                            pos: Some(declaration.keyword.pos.clone()),
                        });
                    },
                    Lvalue::TupleIdentifier(node) => {
                        r = compile_tupleidentifier(&declaration, state, &node,builder,v[i].clone(),stack_index,scope_start_ip);
                    },
                }
                if r.is_err() {return r;}
            }
        }
        _=> {}
    }
    Ok((*builder).clone())
}

impl InstructionBuilder for Declaration {

    fn compile(&self, state: &mut CompilerState) -> AlthreadResult<InstructionBuilderOk> {
        let mut builder = InstructionBuilderOk::new();
        match &self.identifier {
            Lvalue::Identifier(node) => {
                let mut datatype = None;
                let full_var_name = &node.value.value;
                // Get the simple variable name (first and only part)
                let var_name = &node.value.value;

                if state.global_table().contains_key(full_var_name) {
                    return Err(AlthreadError::new(
                        ErrorType::VariableError,
                        Some(node.pos.clone()),
                        format!("Variable {} already declared", full_var_name),
                    ));
                }

                // Check if the variable starts with a capital letter (reserved for shared variables)
                if var_name.chars().next().unwrap().is_uppercase() {
                    if !state.is_shared {
                        return Err(AlthreadError::new(
                            ErrorType::VariableError,
                            Some(node.pos.clone()),
                            format!("Variable {} starts with a capital letter, which is reserved for shared variables", var_name)
                        ));
                    }
                } else {
                    if state.is_shared && !state.in_function {
                        return Err(AlthreadError::new(
                            ErrorType::VariableError,
                            Some(node.pos.clone()),
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
                        let types_compatible =
                            if let (DataType::List(declared_elem), DataType::List(computed_elem)) =
                                (&declared_datatype, &computed_datatype)
                            {
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
                        if let (DataType::List(declared_elem), DataType::List(computed_elem)) =
                            (&declared_datatype, &computed_datatype)
                        {
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
                            Some(node.pos.clone()),
                            "Declaration must have a datatype or a value".to_string(),
                        ));
                    }
                    builder.instructions.push(Instruction {
                        control: InstructionType::Push(datatype.as_ref().unwrap().default()),
                        pos: Some(self.keyword.pos.clone()),
                    });
                }

                let datatype = datatype.unwrap();

                let stack_index = state.program_stack.len();
                // Variable becomes valid at the next instruction (after the declaration instruction)
                let scope_start_ip = builder.instructions.len();
                
                state.program_stack.push(Variable {
                    mutable: self.keyword.value == DeclarationKeyword::Let,
                    name: var_name.clone(), // Use the simple variable name, not the full qualified name
                    datatype: datatype.clone(),
                    depth: state.current_stack_depth,
                    declare_pos: Some(node.pos.clone()),
                });
                
                // Add debug info to the builder (will be adjusted when builders are extended)
                builder.debug_variables.push(crate::compiler::LocalVariableDebugInfo {
                    name: var_name.clone(),
                    datatype,
                    stack_index,
                    scope_start_ip,
                    scope_end_ip: None,
                    declare_pos: Some(node.pos.clone()),
                });
            },
            Lvalue::TupleIdentifier(node) => {
                let mut datatype: Option<DataType> = None;

                if let Some(d) = &self.datatype {
                    datatype = Some(d.value.clone());
                }

                if let Some(value) = &self.value {
                    print!("est une sideexpression ? : value -> {:?}\n",value);
                    return Err(AlthreadError::new(
                            ErrorType::TypeError,
                            Some(node.pos.clone()),
                            "Pas d'instantiatition".to_string(),
                        ));
                } else {
                    if datatype.is_none() {
                        return Err(AlthreadError::new(
                            ErrorType::TypeError,
                            Some(node.pos.clone()),
                            "Pas d'instantiatition".to_string(),
                        ));
                    }
                    
                }
                let r = compile_tupleidentifier(&self, state, &node, &mut builder,datatype.unwrap(),0,0);
                if r.is_err() {return r;}
            },
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
                // Get the display name for the identifier
                let identifier_name = &node
                    .value.value;

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
                                for i in 0..v.len()
                                {
                                   write!(f, "{p}datatype: ")?; 
                                    v[i].fmt(f)?;
                                    writeln!(f,"")?;
                                } 
                            }
                            _ => {writeln!(f, "{prefix}datatype: {datatype}")?; }
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
                                for i in 0..v.len()
                                {
                                   write!(f, "{p}datatype: ")?; 
                                    v[i].fmt(f)?;
                                    writeln!(f,"")?;
                                } 
                            }
                            _ => {writeln!(f, "{prefix}datatype: {datatype}")?; }
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
        }
        Ok(())
    }
}
