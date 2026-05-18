use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub enum DeclarationKeyword {
    Let,
    Const,
}

impl fmt::Display for DeclarationKeyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Let => write!(f, "let"),
            Self::Const => write!(f, "const"),
        }
    }
}
