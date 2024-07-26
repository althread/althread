use std::fmt;

use super::datatype::DataType;

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl Value {
    pub fn from_datatype(datatype: &DataType) -> Self {
        match datatype {
            DataType::Void => Value::Null,
            DataType::Bool => Value::Bool(false),
            DataType::Int => Value::Int(0),
            DataType::Float => Value::Float(0.0),
            DataType::String => Value::String("".to_string()),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(fl) => fl.to_string(),
            Value::String(s) => s.clone(),
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            Value::Null | Value::Int(0) | Value::Float(0.0) => true,
            Value::String(s) if s.is_empty() => true,
            _ => false,
        }
    }

    pub fn add(&self, other: &Value) -> Result<Self, String> {
        match (self, other) {
            (Value::Int(i), Value::Int(j)) => Ok(Value::Int(i + j)),
            (Value::Float(i), Value::Float(j)) => Ok(Value::Float(i + j)),
            (Value::String(i), Value::String(j)) => Ok(Value::String(format!("{}{}", i, j))),
            (i, j) => Err(format!(
                "Cannot add {} and {}",
                DataType::from_value(i),
                DataType::from_value(j)
            )),
        }
    }

    pub fn sub(&self, other: &Value) -> Result<Self, String> {
        match (self, other) {
            (Value::Int(i), Value::Int(j)) => Ok(Value::Int(i - j)),
            (Value::Float(i), Value::Float(j)) => Ok(Value::Float(i - j)),
            (i, j) => Err(format!(
                "Cannot subtract {} and {}",
                DataType::from_value(i),
                DataType::from_value(j)
            )),
        }
    }

    pub fn mul(&self, other: &Value) -> Result<Self, String> {
        match (self, other) {
            (Value::Int(i), Value::Int(j)) => Ok(Value::Int(i * j)),
            (Value::Float(i), Value::Float(j)) => Ok(Value::Float(i * j)),
            (i, j) => Err(format!(
                "Cannot multiply {} and {}",
                DataType::from_value(i),
                DataType::from_value(j)
            )),
        }
    }

    pub fn div(&self, other: &Value) -> Result<Self, String> {
        match (self, other) {
            (i, j) if j.is_null() => Err(format!("Cannot divide {} by {}", i, j)),
            (Value::Int(i), Value::Int(j)) if *j != 0 => Ok(Value::Int(i / j)),
            (Value::Float(i), Value::Float(j)) if *j != 0.0 => Ok(Value::Float(i / j)),
            (i, j) => Err(format!(
                "Cannot divide {} and {}",
                DataType::from_value(i),
                DataType::from_value(j)
            )),
        }
    }

    pub fn modulo(&self, other: &Value) -> Result<Self, String> {
        match (self, other) {
            (i, j) if j.is_null() => Err(format!("Cannot divide {} by {}", i, j)),
            (Value::Int(i), Value::Int(j)) if *j != 0 => Ok(Value::Int(i % j)),
            (Value::Float(i), Value::Float(j)) if *j != 0.0 => Ok(Value::Float(i % j)),
            (i, j) => Err(format!(
                "No modulo between {} and {}",
                DataType::from_value(i),
                DataType::from_value(j)
            )),
        }
    }

    pub fn eq(&self, other: &Value) -> Result<Self, String> {
        match (self, other) {
            (Value::Int(i), Value::Int(j)) => Ok(Value::Bool(i == j)),
            (Value::Float(i), Value::Float(j)) => Ok(Value::Bool(i == j)),
            (Value::String(i), Value::String(j)) => Ok(Value::Bool(i == j)),
            (Value::Bool(i), Value::Bool(j)) => Ok(Value::Bool(i == j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                DataType::from_value(i),
                DataType::from_value(j)
            )),
        }
    }

    pub fn ne(&self, other: &Value) -> Result<Self, String> {
        match (self, other) {
            (Value::Int(i), Value::Int(j)) => Ok(Value::Bool(i != j)),
            (Value::Float(i), Value::Float(j)) => Ok(Value::Bool(i != j)),
            (Value::String(i), Value::String(j)) => Ok(Value::Bool(i != j)),
            (Value::Bool(i), Value::Bool(j)) => Ok(Value::Bool(i != j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                DataType::from_value(i),
                DataType::from_value(j)
            )),
        }
    }

    pub fn gt(&self, other: &Value) -> Result<Self, String> {
        match (self, other) {
            (Value::Int(i), Value::Int(j)) => Ok(Value::Bool(i > j)),
            (Value::Float(i), Value::Float(j)) => Ok(Value::Bool(i > j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                DataType::from_value(i),
                DataType::from_value(j)
            )),
        }
    }

    pub fn ge(&self, other: &Value) -> Result<Self, String> {
        match (self, other) {
            (Value::Int(i), Value::Int(j)) => Ok(Value::Bool(i >= j)),
            (Value::Float(i), Value::Float(j)) => Ok(Value::Bool(i >= j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                DataType::from_value(i),
                DataType::from_value(j)
            )),
        }
    }

    pub fn lt(&self, other: &Value) -> Result<Self, String> {
        match (self, other) {
            (Value::Int(i), Value::Int(j)) => Ok(Value::Bool(i < j)),
            (Value::Float(i), Value::Float(j)) => Ok(Value::Bool(i < j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                DataType::from_value(i),
                DataType::from_value(j)
            )),
        }
    }

    pub fn le(&self, other: &Value) -> Result<Self, String> {
        match (self, other) {
            (Value::Int(i), Value::Int(j)) => Ok(Value::Bool(i <= j)),
            (Value::Float(i), Value::Float(j)) => Ok(Value::Bool(i <= j)),
            (i, j) => Err(format!(
                "Cannot compare {} and {}",
                DataType::from_value(i),
                DataType::from_value(j)
            )),
        }
    }

    pub fn and(&self, other: &Value) -> Result<Self, String> {
        match (self, other) {
            (Value::Bool(i), Value::Bool(j)) => Ok(Value::Bool(*i && *j)),
            (i, j) => Err(format!(
                "Cannot perform logical AND between {} and {}",
                DataType::from_value(i),
                DataType::from_value(j)
            )),
        }
    }

    pub fn or(&self, other: &Value) -> Result<Self, String> {
        match (self, other) {
            (Value::Bool(i), Value::Bool(j)) => Ok(Value::Bool(*i || *j)),
            (i, j) => Err(format!(
                "Cannot perform logical OR between {} and {}",
                DataType::from_value(i),
                DataType::from_value(j)
            )),
        }
    }

    pub fn neg(&self) -> Result<Self, String> {
        match self {
            Value::Int(i) => Ok(Value::Int(-i)),
            Value::Float(f) => Ok(Value::Float(-f)),
            _ => Err("Cannot negate non-numeric value".to_string()),
        }
    }

    pub fn not(&self) -> Result<Self, String> {
        match self {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            _ => Err("Cannot negate non-boolean value".to_string()),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => {
                if fl.fract() == 0.0 {
                    write!(f, "{:.1}", fl)
                } else {
                    write!(f, "{}", fl)
                }
            }
            Value::String(s) => write!(f, "{}", s),
        }
    }
}
