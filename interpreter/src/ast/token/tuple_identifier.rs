use std::fmt::{self, Display};

use pest::iterators::{Pair, Pairs};

use crate::{
    ast::{
        display::{AstDisplay, Prefix}, node::{Node, NodeBuilder}, token::{identifier::Identifier,null_identifier::NullIdentifier,
        // object_identifier::ObjectIdentifier
        }
    }, error::{AlthreadResult, Pos}, no_rule, parser::Rule
};

#[derive(Debug, Clone, PartialEq)]
pub struct TupleIdentifier{
    pub value : Vec<Box<Lvalue>>
}


#[derive(Debug, Clone, PartialEq)]
pub enum Lvalue {
    Identifier(Node<Identifier>,),
    TupleIdentifier(Node<TupleIdentifier>),
    NullIdentifier(Node<NullIdentifier>),
    // Identifier(Node<Identifier>),
}

impl NodeBuilder for TupleIdentifier {
    fn build(mut pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        // This builder expects to be called from a non-atomic `identifier` rule,
        // which has one inner `IDENT` rule.
        let mut value : Vec<Box<Lvalue>> =  vec![];
        while let Some(pair) = pairs.next() {
            match pair.as_rule()
            {
                Rule::identifier => {
                   value.append(&mut vec![
                        Box::new(Lvalue::Identifier(Node::build(pair, filepath)?))
                    ]); 
                },
                Rule::identifier_tuple => {
                    value.append(&mut vec![
                        Box::new(Lvalue::TupleIdentifier(Node::build(pair, filepath)?))
                    ]);
                },
                Rule::identifier_null =>{
                    value.append(&mut vec![
                        Box::new(Lvalue::NullIdentifier(Node::build(pair, filepath)?))
                    ]);
                },
                _=> {
                    return Err(no_rule!(pair, "ImportBlock", filepath));}
            }
        }
        if value.is_empty()
        {
          return Err(crate::error::AlthreadError::new(
                crate::error::ErrorType::SyntaxError,
                None, // We don't have position info here.
                "Internal Compiler Error: TupleIdentifier::build called with empty pairs.".to_string(),
            ));
        }
        Ok(Self { value : value,})
    }
}

impl AstDisplay for TupleIdentifier {
    fn ast_fmt(&self, f: &mut fmt::Formatter, prefix: &Prefix) -> std::fmt::Result {
        let mut v = self.value.iter();
        writeln!(f, "{prefix}tuple:")?;
        let p = &prefix.add_leaf();
        while let Some(elt) = v.next()
        {
            let lvalue : Lvalue = (*(*elt).clone()).into();
            match lvalue {
                Lvalue::Identifier(node) => {
                    write!(f, "{p}ident: ")?;
                    node.value.fmt(f)?;
                }
                Lvalue::TupleIdentifier(node) => {
                    node.value.ast_fmt(f,&prefix.add_leaf())?;
                },
                Lvalue::NullIdentifier(node) => {
                    write!(f, "{p}ident ignored : ")?;
                    node.value.fmt(f)?;
                },
            }
        }
        Ok(())   
    }
}