use std::fmt;

use crate::{ast::{node::NodeBuilder}, error::AlthreadResult, no_rule, parser::Rule};
use ordered_float::OrderedFloat;
use pest::iterators::Pairs;
use serde::{Deserialize, Serialize};

use super::literal::Literal;

#[derive(Debug, PartialEq, Eq, Clone, Hash, Serialize, Deserialize)]
pub enum DataType {
    Void,
    Boolean,
    Integer,
    Float,
    String,
    Process(String),
    Tuple(Vec<DataType>),
    List(Box<DataType>),
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
            DataType::List(t) => Literal::List(t.as_ref().clone(), vec![]),
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
            DataType::Process(n) => format!("proc({})", n),
            DataType::Tuple(t) => format!(
                "({})",
                t.iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            DataType::List(t) => format!("list({})", t.to_string()),
        }
    }

    pub fn is_a_number(&self) -> bool {
        match self {
            Self::Integer | Self::Float => true,
            _ => false,
        }
    }
    pub fn is_integer(&self) -> bool {
        match self {
            Self::Integer => true,
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

    pub fn is_process(&self) -> (bool, String) {
        // println!("Checking if {:?} is a process", self);
        match self {
            Self::List(datatype) => datatype.is_process(),
            Self::Process(name) => (true, name.clone()),
            _ => (false, String::new()),
        }
    }

    pub fn tuple_unwrap(&self) -> Vec<DataType> {
        match self {
            Self::Tuple(v) => v.clone(),
            _ => panic!("Call tuple_unwrap on a type that is not a tuple"),
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
            Rule::PROCESS_TYPE => Ok(Self::Process(
                pair.into_inner().next().unwrap().as_str().to_string(),
            )),
            Rule::LIST_TYPE => {
                let mut pairs = pair.into_inner();
                let datatype = DataType::build(pairs.next().unwrap().into_inner())?;
                Ok(Self::List(Box::new(datatype)))
            }
            _ => Err(no_rule!(pair, "DataType")),
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
