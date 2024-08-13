use std::{fmt::Formatter, str::FromStr};

use pest::iterators::{Pair, Pairs};

use crate::{
    ast::{
        display::{AstDisplay, Prefix},
        node::{NodeBuilder, NodeExecutor},
    },
    env::Env,
    error::{AlthreadError, AlthreadResult, ErrorType},
    no_rule,
    parser::Rule,
};

use super::datatype::DataType;

#[derive(Debug, Clone)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl NodeBuilder for Literal {
    fn build(mut pairs: Pairs<Rule>) -> AlthreadResult<Self> {
        let pair = pairs.next().unwrap();

        fn safe_parse<T: FromStr>(pair: &Pair<Rule>) -> Result<T, AlthreadError> {
            pair.as_str().parse::<T>().map_err(|_| {
                AlthreadError::new(
                    ErrorType::SyntaxError,
                    pair.line_col().0,
                    pair.line_col().1,
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
            _ => return Err(no_rule!(pair)),
        })
    }
}

impl NodeExecutor for Literal {
    fn eval(&self, _env: &mut Env) -> AlthreadResult<Option<Literal>> {
        Ok(Some(self.clone()))
    }
}

impl Literal {
    pub fn get_data_type(&self) -> DataType {
        match self {
            Self::Null => DataType::Void,
            Self::Bool(_) => DataType::Boolean,
            Self::Int(_) => DataType::Integer,
            Self::Float(_) => DataType::Float,
            Self::String(_) => DataType::String,
        }
    }

    pub fn is_true(&self) -> Result<bool, String> {
        match self {
            Self::Null => Ok(false),
            Self::Int(i) if *i == 0 => Ok(false),
            Self::Float(f) if *f == 0.0 => Ok(false),
            Self::Bool(b) if !*b => Ok(false),
            Self::String(s) if s.is_empty() => Ok(false),
            _ => Ok(true),
        }
    }

    pub fn positive(&self) -> Result<Self, String> {
        match self {
            Self::Int(i) => Ok(Self::Int(*i)),
            Self::Float(f) => Ok(Self::Float(*f)),
            i => Err(format!("Cannot make {} positive", i.get_data_type())),
        }
    }

    pub fn negative(&self) -> Result<Self, String> {
        match self {
            Self::Int(i) => Ok(Self::Int(-i)),
            Self::Float(f) => Ok(Self::Float(-f)),
            i => Err(format!("Cannot negate {}", i.get_data_type())),
        }
    }

    pub fn not(&self) -> Result<Self, String> {
        match self {
            Self::Bool(b) => Ok(Self::Bool(!b)),
            i => Err(format!("Cannot negate {}", i.get_data_type())),
        }
    }

    pub fn add(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Int(i + j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Float(i + j)),
            (Self::String(i), Self::String(j)) => Ok(Self::String(format!("{}{}", i, j))),
            (i, j) => Err(format!(
                "Cannot add {} and {}",
                i.get_data_type(),
                j.get_data_type()
            )),
        }
    }

    pub fn subtract(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Int(i - j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Float(i - j)),
            (i, j) => Err(format!(
                "Cannot subtract {} by {}",
                i.get_data_type(),
                j.get_data_type()
            )),
        }
    }

    pub fn multiply(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Int(i * j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Float(i * j)),
            (i, j) => Err(format!(
                "Cannot multiply {} by {}",
                i.get_data_type(),
                j.get_data_type()
            )),
        }
    }

    pub fn divide(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) if *j != 0 => Ok(Self::Int(i / j)),
            (Self::Float(i), Self::Float(j)) if *j != 0.0 => Ok(Self::Float(i / j)),
            (i, j) => Err(format!(
                "Cannot divide {} by {}",
                i.get_data_type(),
                j.get_data_type()
            )),
        }
    }

    pub fn modulo(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) if *j != 0 => Ok(Self::Int(i % j)),
            (Self::Float(i), Self::Float(j)) if *j != 0.0 => Ok(Self::Float(i % j)),
            (i, j) => Err(format!(
                "No modulo between {} and {}",
                i.get_data_type(),
                j.get_data_type()
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
                i.get_data_type(),
                j.get_data_type()
            )),
        }
    }

    pub fn not_equals(&self, other: &Self) -> Result<Self, String> {
        Ok(Self::Bool(!self.equals(other)?.is_true()?))
    }

    pub fn less_than(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Bool(i < j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Bool(i < j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                i.get_data_type(),
                j.get_data_type()
            )),
        }
    }

    pub fn less_than_or_equal(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Bool(i <= j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Bool(i <= j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                i.get_data_type(),
                j.get_data_type()
            )),
        }
    }

    pub fn greater_than(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Bool(i > j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Bool(i > j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                i.get_data_type(),
                j.get_data_type()
            )),
        }
    }

    pub fn greater_than_or_equal(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Int(i), Self::Int(j)) => Ok(Self::Bool(i >= j)),
            (Self::Float(i), Self::Float(j)) => Ok(Self::Bool(i >= j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                i.get_data_type(),
                j.get_data_type()
            )),
        }
    }

    pub fn and(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Bool(i), Self::Bool(j)) => Ok(Self::Bool(*i && *j)),
            (i, j) => Err(format!(
                "Cannot perform AND operation between {} and {}",
                i.get_data_type(),
                j.get_data_type()
            )),
        }
    }

    pub fn or(&self, other: &Self) -> Result<Self, String> {
        match (self, other) {
            (Self::Bool(i), Self::Bool(j)) => Ok(Self::Bool(*i || *j)),
            (i, j) => Err(format!(
                "Cannot perform OR operation between {} and {}",
                i.get_data_type(),
                j.get_data_type()
            )),
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
        }
    }
}