use std::fmt;

use pest::iterators::Pairs;

use crate::{ast::node::NodeBuilder, error::AlthreadResult, no_rule, parser::Rule};

use super::literal::Literal;

#[derive(Debug, PartialEq, Clone)]
pub enum DataType {
    Void,
    Boolean,
    Integer,
    Float,
    String,
    Process(String),
}

impl DataType {
    pub fn default(&self) -> Literal {
        match self {
            DataType::Void => Literal::Null,
            DataType::Boolean => Literal::Bool(false),
            DataType::Integer =>  Literal::Int(0),
            DataType::Float =>  Literal::Float(0.0),
            DataType::String =>  Literal::String("".to_string()),
            DataType::Process(_) => Literal::Null,
        }
    }
    pub fn from_str(value: &str) -> Self {
        match value {
            "bool" => Self::Boolean,
            "int" => Self::Integer,
            "float" => Self::Float,
            "string" => Self::String,
            _ => Self::Void,
        }
    }

    /*     pub fn from_value(val: &Value) -> Self {
        match val {
            Value::Null => Self::Void,
            Value::Bool(_) => Self::Bool,
            Value::Int(_) => Self::Int,
            Value::Float(_) => Self::Float,
            Value::String(_) => Self::String,
        }
    } */

    pub fn as_str(&self) -> &str {
        match self {
            DataType::Void => "void",
            DataType::Boolean => "bool",
            DataType::Integer => "int",
            DataType::Float => "float",
            DataType::String => "string",
            DataType::Process(n) => n,
        }
    }

    pub fn is_a_number(&self) -> bool {
        match self {
            Self::Integer | Self::Float => true,
            _ => false,
        }
    }
    pub fn is_boolean(&self) -> bool {
        match self {
            Self::Boolean => true,
            _ => false,
        }
    }
    pub fn is_process_of(&self, name: &str) -> bool {
        match self {
            Self::Process(n) => n == name,
            _ => false,
        }
    }

} 

impl NodeBuilder for DataType {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();
        match pair.as_rule() {
            Rule::BOOL_TYPE => Ok(Self::Boolean),
            Rule::INT_TYPE => Ok(Self::Integer),
            Rule::FLOAT_TYPE => Ok(Self::Float),
            Rule::STR_TYPE => Ok(Self::String),
            Rule::VOID_TYPE => Ok(Self::Void),
            _ => Err(no_rule!(pair)),
        }
    }
}

impl DataType {
    pub fn get_literal(&self) -> Literal {
        match self {
            DataType::Void => Literal::Null,
            DataType::Boolean => Literal::Bool(false),
            DataType::Integer => Literal::Int(0),
            DataType::Float => Literal::Float(0.0),
            DataType::String => Literal::String("".to_string()),
            DataType::Process(_) => Literal::Null,
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let datatype = match self {
            DataType::Void => "void",
            DataType::Boolean => "bool",
            DataType::Integer => "int",
            DataType::Float => "float",
            DataType::String => "string",
            DataType::Process(n) => { write!(f, "process({})", n); "" },
        };

        write!(f, "{}", datatype)
    }
}
