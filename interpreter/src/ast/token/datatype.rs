use std::fmt;

use pest::iterators::Pairs;
use ordered_float::OrderedFloat;
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
    Tuple(Vec<DataType>),
}

impl DataType {
    pub fn default(&self) -> Literal {
        match self {
            DataType::Void => Literal::Null,
            DataType::Boolean => Literal::Bool(false),
            DataType::Integer => Literal::Int(0),
            DataType::Float => Literal::Float(OrderedFloat(0.0)),
            DataType::String => Literal::String("".to_string()),
            DataType::Process(_) => Literal::Null,
            DataType::Tuple(v) => Literal::Tuple(v.iter().map(|d| d.default()).collect()),
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

    pub fn to_string(&self) -> String {
        match self {
            DataType::Void => "void".to_string(),
            DataType::Boolean => "bool".to_string(),
            DataType::Integer => "int".to_string(),
            DataType::Float => "float".to_string(),
            DataType::String => "string".to_string(),
            DataType::Process(n) => format!("program({})", n),
            DataType::Tuple(t) => format!(
                "({})",
                t.iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
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
            _ => Err(no_rule!(pair, "DataType")),
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
