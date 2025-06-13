use serde::{Deserialize, Serialize};
use std::fmt;
use pest::Span;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
    pub start: usize,
    pub end: usize,
}
// implement default:
impl Default for Pos {
    fn default() -> Self {
        Self {
            line: 0,
            col: 0,
            start: 0,
            end: 0,
        }
    }
}

impl<'i> From<Span<'i>> for Pos {
    fn from(span: Span<'i>) -> Self {
        let start_pos = span.start_pos().line_col();
        Self {
            line: start_pos.0,
            col: start_pos.1,
            start: span.start(),
            end: span.end(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlthreadError {
    pub pos: Option<Pos>,
    pub message: String,
    pub error_type: ErrorType,
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
    InvariantError,
    NoPathError,
    NotImplemented,
    UndefinedFunction,
    UndefinedChannel,
    ReturnOutsideFunction,
    FunctionAlreadyDefined,
    FunctionArgumentCountError,
    FunctionArgumentTypeMismatch,
    FunctionNotFound,
    FunctionMissingReturnStatement,
    FunctionReturnTypeMismatch
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
            ErrorType::InvariantError => write!(f, "Invariant Error"),
            ErrorType::NoPathError => write!(f, "No Path Error"),
            ErrorType::ReturnOutsideFunction => write!(f, "Return statement can only be in a function"),
            ErrorType::FunctionAlreadyDefined => write!(f, "Function already defined"),
            ErrorType::FunctionArgumentCountError => write!(f, "Function argument count error"),
            ErrorType::FunctionArgumentTypeMismatch => write!(f, "Function argument type mismatch"),
            ErrorType::FunctionNotFound => write!(f, "Function not found"),
            ErrorType::FunctionMissingReturnStatement => write!(f, "Function missing return statement"),
            ErrorType::FunctionReturnTypeMismatch => write!(f, "Function return type mismatch"),
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
