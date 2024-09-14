use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlthreadError {
    pos: Option<Pos>,
    message: String,
    error_type: ErrorType,
}

pub type AlthreadResult<T> = Result<T, AlthreadError>;

#[macro_export]
macro_rules! no_rule {
    ($pair:expr, $loc:expr) => {
        $crate::error::AlthreadError::new(
            $crate::error::ErrorType::SyntaxError,
            Some($crate::error::Pos {
                line: $pair.line_col().0,
                col: $pair.line_col().1,
                start: $pair.as_span().start(),
                end: $pair.as_span().end(),
            }),
            format!("Unexpected rule: {:?} in object {}", $pair.as_rule(), $loc),
        )
    };
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ErrorType {
    SyntaxError,
    TypeError,
    VariableError,
    RuntimeError,
    DivisionByZero,
    ArithmeticError,
    ProcessError,
    InstructionNotAllowed,
    ExpressionError,
    NotImplemented,
    UndefinedFunction,
    UndefinedChannel,
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorType::SyntaxError => write!(f, "Syntax Error"),
            ErrorType::TypeError => write!(f, "Type Error"),
            ErrorType::VariableError => write!(f, "Variable Error"),
            ErrorType::RuntimeError => write!(f, "Runtime Error"),
            ErrorType::DivisionByZero => write!(f, "Division by zero"),
            ErrorType::ArithmeticError => write!(f, "Arithmetic Error"),
            ErrorType::ProcessError => write!(f, "Process Error"),
            ErrorType::InstructionNotAllowed => write!(f, "Instruction Not Allowed"),
            ErrorType::ExpressionError => write!(f, "Expression Error"),
            ErrorType::NotImplemented => write!(f, "Not Implemented"),
            ErrorType::UndefinedFunction => write!(f, "Undefined Function"),
            ErrorType::UndefinedChannel => write!(f, "Undefined Channel"),
        }
    }
}

impl AlthreadError {
    pub fn new(error_type: ErrorType, pos: Option<Pos>, message: String) -> Self {
        Self {
            pos,
            message,
            error_type,
        }
    }

    pub fn report(&self, input: &str) {
        match self.pos {
            Some(pos) => {
                eprintln!("Error at {}:{}", pos.line, pos.col);
                self.print_err_line(input);
            }
            None => {
                eprintln!("Runtime Error:");
            }
        };
        eprintln!("{}: {}", self.error_type, self.message);
    }

    fn print_err_line(&self, input: &str) {
        if self.pos.is_none() {
            return;
        }
        let pos = self.pos.unwrap();

        let line = match input.lines().nth(pos.line - 1) {
            Some(line) => line.to_string(),
            None => return,
        };

        let line_indent = " ".repeat(pos.line.to_string().len());
        eprintln!("{} |", line_indent);
        eprintln!("{} | {}", pos.line, line);
        eprintln!("{} |{}^---", line_indent, " ".repeat(pos.col));
        eprintln!("{} |", line_indent);
    }
}
