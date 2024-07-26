use pest::iterators::Pair;

use crate::{
    env::{datatype::DataType, Environment},
    error::AlthreadResult,
    no_rule,
    parser::Rule,
};

pub fn check_decl(pair: Pair<Rule>, env: &mut Environment) -> AlthreadResult<()> {
    let mut pairs = pair.into_inner();
    let mutable = pairs.next().unwrap().as_str() == "let";
    let identifier = pairs.next().unwrap();
    let mut datatype = None;
    for pair in pairs {
        match pair.as_rule() {
            Rule::DATATYPE => datatype = Some(DataType::from_str(pair.as_str())),
            Rule::expr => {}
            _ => return Err(no_rule!(pair)),
        }
    }

    env.insert_symbol(mutable, &identifier, datatype, None)
}
