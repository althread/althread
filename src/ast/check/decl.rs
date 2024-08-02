use pest::iterators::Pair;

use crate::{
    env::{datatype::DataType, value::Value, Environment},
    error::AlthreadResult,
    no_rule,
    parser::Rule,
};

use super::expr::check_expr;

pub fn check_decl(pair: Pair<Rule>, env: &mut Environment) -> AlthreadResult<()> {
    let mut pairs = pair.into_inner();
    let mutable = pairs.next().unwrap().as_str() == "let";
    let identifier: Pair<Rule> = pairs.next().unwrap();
    let mut datatype = None;
    let mut value = None;
    for pair in pairs {
        match pair.as_rule() {
            Rule::DATATYPE => datatype = Some(DataType::from_str(pair.as_str())),
            Rule::expr => value = Some(Value::from_datatype(&check_expr(pair, env)?)),
            _ => return Err(no_rule!(pair)),
        }
    }

    env.insert_symbol(mutable, &identifier, datatype, value)
}
