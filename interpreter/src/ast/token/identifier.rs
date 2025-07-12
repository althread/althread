use pest::iterators::Pairs;

use crate::{
    ast::node::{NodeBuilder},
    error::{AlthreadResult},
    no_rule,
    parser::Rule,
};

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Identifier {
    pub value: String,
}

impl NodeBuilder for Identifier {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        // This builder expects to be called from a non-atomic `identifier` rule,
        // which has one inner `IDENT` rule.
        if let Some(pair) = pairs.next() {
            // Ensure there's only one inner pair, as expected.
            if pairs.next().is_some() {
                return Err(crate::error::AlthreadError::new(
                    crate::error::ErrorType::SyntaxError,
                    None,
                    "Identifier builder expected only one inner pair.".to_string(),
                ));
            }

            match pair.as_rule() {
                Rule::IDENT => Ok(Self {
                    value: pair.as_str().to_string(),
                }),
                _ => Err(no_rule!(pair, "Identifier")),
            }
        } else {
            // This error means Node::build was called on an atomic rule (like IDENT)
            // instead of a wrapper rule (like identifier). This is a bug in the calling code.
            Err(crate::error::AlthreadError::new(
                crate::error::ErrorType::SyntaxError,
                None, // We don't have position info here.
                "Internal Compiler Error: Identifier::build called with empty pairs. Check for Node::build calls on atomic IDENT rules.".to_string(),
            ))
        }
    }
}
