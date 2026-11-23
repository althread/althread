use std::fmt;

use crate::ast::{
    display::{AstDisplay, Prefix},
    node::Node,
    token::{identifier::Identifier, null_identifier::NullIdentifier},
};

#[derive(Debug, Clone, PartialEq)]
pub struct TupleIdentifier {
    pub value: Vec<Box<Lvalue>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Lvalue {
    Identifier(Node<Identifier>),
    TupleIdentifier(Node<TupleIdentifier>),
    NullIdentifier(Node<NullIdentifier>),
}

impl AstDisplay for TupleIdentifier {
    fn ast_fmt(&self, f: &mut fmt::Formatter<'_>, prefix: &Prefix) -> fmt::Result {
        writeln!(f, "{prefix}tuple:")?;
        let child_prefix = prefix.add_leaf();
        for value in &self.value {
            match value.as_ref() {
                Lvalue::Identifier(node) => {
                    writeln!(f, "{child_prefix}ident: {}", node.value)?;
                }
                Lvalue::TupleIdentifier(node) => {
                    node.value.ast_fmt(f, &child_prefix)?;
                }
                Lvalue::NullIdentifier(node) => {
                    writeln!(f, "{child_prefix}ident ignored: {}", node.value)?;
                }
            }
        }
        Ok(())
    }
}
