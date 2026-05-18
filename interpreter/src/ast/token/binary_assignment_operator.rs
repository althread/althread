use std::fmt;

use super::literal::Literal;

#[derive(Debug, PartialEq, Clone)]
pub enum BinaryAssignmentOperator {
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
    OrAssign,
}

impl BinaryAssignmentOperator {
    pub fn apply(&self, left: &Literal, right: &Literal) -> Result<Literal, String> {
        match self {
            Self::Assign => Ok(right.clone()),
            Self::AddAssign => left.add(right),
            Self::SubtractAssign => left.subtract(right),
            Self::MultiplyAssign => left.multiply(right),
            Self::DivideAssign => left.divide(right),
            Self::ModuloAssign => left.modulo(right),
            Self::OrAssign => left.or(right),
        }
    }
}

impl fmt::Display for BinaryAssignmentOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Assign => write!(f, "="),
            Self::AddAssign => write!(f, "+="),
            Self::SubtractAssign => write!(f, "-="),
            Self::MultiplyAssign => write!(f, "*="),
            Self::DivideAssign => write!(f, "/="),
            Self::ModuloAssign => write!(f, "%="),
            Self::OrAssign => write!(f, "|="),
        }
    }
}
