use core::fmt;
use ordered_float::OrderedFloat;
use pest::iterators::{Pair, Pairs};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::{
    fmt::Formatter,
    hash::Hash,
    str::FromStr,
};

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::NodeBuilder,
    },
    error::{AlthreadError, AlthreadResult, ErrorType, Pos},
    no_rule,
    parser::Rule,
};

use super::datatype::DataType;

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(OrderedFloat<f64>),
    String(String),
    Process(String, usize),
    Tuple(Vec<Literal>),
    List(DataType, Vec<Literal>),
}

impl<'a> Serialize for Literal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Literal", 1)?;
        match self {
            Self::Null => state.serialize_field("null", &true)?,
            Self::Bool(value) => state.serialize_field("bool", value)?,
            Self::Int(value) => state.serialize_field("int", value)?,
            Self::Float(value) => state.serialize_field("float", value.as_ref())?,
            Self::String(value) => state.serialize_field("string", value)?,
            Self::Process(name, pid) => {
                state.serialize_field("program", name)?;
                state.serialize_field("pid", pid)?;
            }
            Self::Tuple(values) => state.serialize_field("tuple", values)?,
            Self::List(datatype, values) => {
                state.serialize_field("list_datatype", datatype)?;
                state.serialize_field("list", values)?;
            }
        }
        state.end()
    }
}

impl NodeBuilder for Literal {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();

        fn safe_parse<T: FromStr>(pair: &Pair<Rule>) -> Result<T, AlthreadError> {
            pair.as_str().parse::<T>().map_err(|_| {
                AlthreadError::new(
                    ErrorType::SyntaxError,
                    Some(Pos {
                        start: pair.as_span().start(),
                        end: pair.as_span().end(),
                        line: pair.line_col().0,
                        col: pair.line_col().1,
                    }),
                    format!("Cannot parse {}", pair.as_str()),
                )
            })
        }

        Ok(match pair.as_rule() {
            Rule::NULL => Self::Null,
            Rule::BOOL => Self::Bool(safe_parse(&pair)?),
            Rule::INT => Self::Int(safe_parse(&pair)?),
            Rule::FLOAT => Self::Float(safe_parse(&pair)?),
            Rule::STR => Self::String(pair.as_str().to_string()),
            _ => return Err(no_rule!(pair, "Literal")),
        })
    }
}

impl Literal {
    pub fn get_datatype(&self) -> DataType {
        match self {
            Self::Null => DataType::Void,
            Self::Bool(_) => DataType::Boolean,
            Self::Int(_) => DataType::Integer,
            Self::Float(_) => DataType::Float,
            Self::String(_) => DataType::String,
            Self::Process(n, _) => DataType::Process(n.to_string()),
            Self::Tuple(t) => DataType::Tuple(t.iter().map(|l| l.get_datatype()).collect()),
            Self::List(d, _) => DataType::List(Box::new(d.clone())),
        }
    }

    pub fn empty_tuple() -> Self {
        Self::Tuple(Vec::new())
    }
    pub fn to_integer(&self) -> Result<i64, String> {
        match self {
            Self::Int(i) => Ok(*i),
            i => Err(format!("Cannot convert {} to integer", i.get_datatype())),
        }
    }
    pub fn to_pid(&self) -> Result<usize, String> {
        match self {
            Self::Process(_, pid) => Ok(*pid),
            i => Err(format!("Cannot convert {} to pid", i.get_datatype())),
        }
    }
    pub fn to_tuple(&self) -> Result<&Vec<Literal>, String> {
        match self {
            Self::Tuple(t) => Ok(t),
            i => Err(format!("Cannot convert {} to tuple", i.get_datatype())),
        }
    }
    pub fn into_tuple(self) -> Result<Vec<Literal>, String> {
        match self {
            Self::Tuple(t) => Ok(t),
            i => Err(format!("Cannot convert {} to tuple", i.get_datatype())),
        }
    }

    pub fn is_true(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Int(i) if *i == 0 => false,
            Self::Float(f) if *f == 0.0 => false,
            Self::Bool(b) if !*b => false,
            Self::String(s) if s.is_empty() => false,
            _ => true,
        }
    }

    pub fn positive(&self) -> Result<Self, String> {
        match self {
            Self::Int(i) => Ok(Self::Int(*i)),
            Self::Float(f) => Ok(Self::Float(*f)),
            i => Err(format!("Cannot make {} positive", i.get_datatype())),
        }
    }

    pub fn negative(&self) -> Result<Self, String> {
        match self {
            Self::Int(i) => Ok(Self::Int(-i)),
            Self::Float(f) => Ok(Self::Float(-f)),
            i => Err(format!("Cannot negate {}", i.get_datatype())),
        }
    }

    pub fn not(&self) -> Result<Self, String> {
        match self {
            Self::Bool(b) => Ok(Self::Bool(!b)),
            i => Err(format!("Cannot negate {}", i.get_datatype())),
        }
    }

    pub fn add(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Int(i + j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Float(i + *j)),
            (Self::String(i), Self::String(j)) => Ok(Self::String(format!("{}{}", i, j))),
            (i, j) => Err(format!(
                "Cannot add {} and {}",
                i.get_datatype(),
                j.get_datatype()
            )),
        }
    }

    pub fn subtract(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Int(i - j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Float(i - *j)),
            (i, j) => Err(format!(
                "Cannot subtract {} by {}",
                i.get_datatype(),
                j.get_datatype()
            )),
        }
    }

    pub fn multiply(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Int(i * j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Float(i * *j)),
            (i, j) => Err(format!(
                "Cannot multiply {} by {}",
                i.get_datatype(),
                j.get_datatype()
            )),
        }
    }

    pub fn divide(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (_, Self::Int(0)) | (_, Self::Float(OrderedFloat(0.0))) => {
                Err("Cannot divide by zero".to_string())
            }
            (Self::Int(i), Self::Int(j)) => Ok(Self::Int(i / j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Float(i / *j)),
            (i, j) => Err(format!(
                "Cannot divide {} by {}",
                i.get_datatype(),
                j.get_datatype()
            )),
        }
    }

    pub fn modulo(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) if *j != 0 => Ok(Self::Int(i % j)),
            (Self::Float(i), Self::Float(j)) if *j != 0.0 => Ok(Self::Float(i % *j)),
            (i, j) => Err(format!(
                "No modulo between {} and {}",
                i.get_datatype(),
                j.get_datatype()
            )),
        }
    }

    pub fn equals(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Null, Self::Null) => Ok(Self::Bool(true)),
            (Self::Int(i), Self::Int(j)) => Ok(Self::Bool(i == j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Bool(i == j)),
            (Self::Bool(i), Self::Bool(j)) => Ok(Self::Bool(i == j)),
            (Self::String(i), Self::String(j)) => Ok(Self::Bool(i == j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                i.get_datatype(),
                j.get_datatype()
            )),
        }
    }

    pub fn not_equals(&self, other: &Self) -> Result<Self, String> {
        Ok(Self::Bool(!self.equals(other)?.is_true()))
    }

    pub fn less_than(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Bool(i < j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Bool(i < j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                i.get_datatype(),
                j.get_datatype()
            )),
        }
    }

    pub fn less_than_or_equal(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Bool(i <= j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Bool(i <= j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                i.get_datatype(),
                j.get_datatype()
            )),
        }
    }

    pub fn greater_than(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Bool(i > j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Bool(i > j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                i.get_datatype(),
                j.get_datatype()
            )),
        }
    }

    pub fn greater_than_or_equal(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Bool(i >= j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Bool(i >= j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                i.get_datatype(),
                j.get_datatype()
            )),
        }
    }

    pub fn and(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Bool(i), Self::Bool(j)) => Ok(Self::Bool(*i && *j)),
            (i, j) => Err(format!(
                "Cannot perform AND operation between {} and {}",
                i.get_datatype(),
                j.get_datatype()
            )),
        }
    }

    pub fn or(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Bool(i), Self::Bool(j)) => Ok(Self::Bool(*i || *j)),
            (i, j) => Err(format!(
                "Cannot perform OR operation between {} and {}",
                i.get_datatype(),
                j.get_datatype()
            )),
        }
    }

    pub fn increment(&self) -> Result<Self, String> {
        match self {
            Self::Int(i) => Ok(Self::Int(i + 1)),
            Self::Float(f) => Ok(Self::Float(f + 1.0)),
            i => Err(format!("Cannot increment {}", i.get_datatype())),
        }
    }

    pub fn decrement(&self) -> Result<Self, String> {
        match self {
            Self::Int(i) => Ok(Self::Int(i - 1)),
            Self::Float(f) => Ok(Self::Float(f - 1.0)),
            i => Err(format!("Cannot decrement {}", i.get_datatype())),
        }
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(value) => write!(f, "{}", value),
            Self::Int(value) => write!(f, "{}", value),
            Self::Float(value) => write!(f, "{}", value),
            Self::String(value) => write!(f, "{}", value),
            Self::Process(name, pid) => write!(f, "{}#{}", name, pid),
            Self::Tuple(values) => write!(
                f,
                "({})",
                values
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Self::List(_datatype, values) => write!(
                f,
                "list({})",
                values
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl AstDisplay for Literal {
    fn ast_fmt(&self, f: &mut Formatter, prefix: &Prefix) -> std::fmt::Result {
        match self {
            Self::Null => writeln!(f, "{prefix}null"),
            Self::Bool(value) => writeln!(f, "{prefix}bool: {value}"),
            Self::Int(value) => writeln!(f, "{prefix}int: {value}"),
            Self::Float(value) => writeln!(f, "{prefix}float: {value}"),
            Self::String(value) => writeln!(f, "{prefix}string: \"{value}\""),
            Self::Process(name, pid) => write!(f, "{prefix}pid {} instance of {}", pid, name),
            Self::Tuple(values) => {
                writeln!(f, "{prefix}tuple")?;
                for value in values {
                    value.ast_fmt(f, &prefix.add_leaf())?;
                }
                Ok(())
            }
            Self::List(datatype, values) => {
                writeln!(f, "{prefix}list({datatype})")?;
                for value in values {
                    value.ast_fmt(f, &prefix.add_leaf())?;
                }
                Ok(())
            }
        }
    }
}
