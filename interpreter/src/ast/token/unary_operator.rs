use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Positive,
    Negative,
    Not,
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            UnaryOperator::Positive => "+",
            UnaryOperator::Negative => "-",
            UnaryOperator::Not => "!",
        };

        write!(f, "{}", op)
    }
}
