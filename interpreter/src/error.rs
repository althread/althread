use ariadne::{Color, Label, Report, ReportKind, Source};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, rc::Rc};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
    pub start: usize,
    pub end: usize,
    pub file_path: String,
}

// implement default:
impl Default for Pos {
    fn default() -> Self {
        Self {
            line: 0,
            col: 0,
            start: 0,
            end: 0,
            file_path: "".to_string(),
        }
    }
}

impl Pos {
    pub fn from_offsets(source: &str, file_path: &str, start: usize, end: usize) -> Self {
        let safe_start = start.min(source.len());
        let safe_end = end.min(source.len()).max(safe_start);
        let prefix = &source[..safe_start];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let line_start = prefix.rfind('\n').map_or(0, |idx| idx + 1);
        let col = safe_start.saturating_sub(line_start) + 1;

        Self {
            line,
            col,
            start: safe_start,
            end: safe_end,
            file_path: file_path.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlthreadError {
    pub pos: Option<Rc<Pos>>,
    pub message: String,
    pub error_type: ErrorType,
    pub stack: Vec<Rc<Pos>>,
}

pub type AlthreadResult<T> = Result<T, AlthreadError>;

#[macro_export]
macro_rules! no_rule {
    ($pair:expr, $loc:expr, $filename:expr) => {
        $crate::error::AlthreadError::new(
            $crate::error::ErrorType::SyntaxError,
            Some($crate::error::Pos {
                line: $pair.line_col().0,
                col: $pair.line_col().1,
                start: $pair.as_span().start(),
                end: $pair.as_span().end(),
                file_path: $filename.to_string(),
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
    FunctionReturnTypeMismatch,
    AssertionFailed,
    ImportNameConflict,
    ModuleNotFound,
    ImportMainConflict,
    VariableAlreadyDefined,
    ProgramAlreadyDefined,
    PrivateFunctionCall,
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
            ErrorType::ReturnOutsideFunction => {
                write!(f, "Return statement can only be in a function")
            }
            ErrorType::FunctionAlreadyDefined => write!(f, "Function already defined"),
            ErrorType::FunctionArgumentCountError => write!(f, "Function argument count error"),
            ErrorType::FunctionArgumentTypeMismatch => write!(f, "Function argument type mismatch"),
            ErrorType::FunctionNotFound => write!(f, "Function not found"),
            ErrorType::FunctionMissingReturnStatement => {
                write!(f, "Function missing return statement")
            }
            ErrorType::FunctionReturnTypeMismatch => write!(f, "Function return type mismatch"),
            ErrorType::AssertionFailed => write!(f, "Assertion failed"),
            ErrorType::ImportNameConflict => write!(f, "Import name conflict"),
            ErrorType::ModuleNotFound => write!(f, "Module not found"),
            ErrorType::ImportMainConflict => write!(f, "Import main conflict"),
            ErrorType::VariableAlreadyDefined => write!(f, "Variable already defined"),
            ErrorType::ProgramAlreadyDefined => write!(f, "Program already defined"),
            ErrorType::PrivateFunctionCall => write!(f, "Private function call"),
        }
    }
}

impl AlthreadError {
    pub fn new(error_type: ErrorType, pos: Option<Pos>, message: String) -> Self {
        let rc_pos = pos.map(Rc::new);
        Self {
            pos: rc_pos.clone(),
            message,
            error_type,
            stack: Vec::new(),
        }
    }

    pub fn push_stack(&mut self, pos: Pos) {
        self.stack.push(Rc::new(pos));
    }

    pub fn report(&self, input_map: &HashMap<String, String>) {
        if let Some(rendered) = self.rendered_report(input_map) {
            eprint!("{rendered}");
        } else {
            match &self.pos {
                Some(pos) => {
                    if !pos.file_path.is_empty() {
                        eprintln!("Error in {} at {}:{}", pos.file_path, pos.line, pos.col);
                    } else {
                        eprintln!("Error at {}:{}", pos.line, pos.col);
                    }
                    self.print_err_line(input_map);
                }
                None => {
                    eprintln!("Runtime Error:");
                }
            };
            eprintln!("{}: {}", self.error_type, self.message);
        }

        // Print error stack
        if !self.stack.is_empty() {
            eprintln!("\nError Stack (most recent call last):");
            for pos in self.stack.iter().rev() {
                if !pos.file_path.is_empty() {
                    eprintln!("  at {}:{}:{}", pos.file_path, pos.line, pos.col);
                } else {
                    eprintln!("  at {}:{}", pos.line, pos.col);
                }
            }
        }
    }

    fn print_err_line(&self, input_map: &HashMap<String, String>) {
        if let Some(pos) = &self.pos {
            let file_path = &pos.file_path;
            let input = input_map
                .get(file_path)
                .expect("File path not found in input map");
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

    pub fn rendered_report(&self, input_map: &HashMap<String, String>) -> Option<String> {
        let pos = self.pos.as_ref()?;
        let file_path = if pos.file_path.is_empty() {
            "<input>".to_string()
        } else {
            pos.file_path.clone()
        };
        let source = if pos.file_path.is_empty() {
            input_map
                .get("")
                .or_else(|| input_map.values().next())
                .cloned()?
        } else {
            input_map.get(&pos.file_path).cloned()?
        };

        let mut output = Vec::new();
        let start = pos.start.min(source.len());
        let end = pos.end.max(start + 1).min(source.len());
        let report = Report::build(ReportKind::Error, (&file_path, start..end))
            .with_message(format!("{}: {}", self.error_type, self.message))
            .with_label(
                Label::new((&file_path, start..end))
                    .with_message(self.message.clone())
                    .with_color(Color::Red),
            )
            .finish();

        report
            .write((&file_path, Source::from(source)), &mut output)
            .ok()?;
        Some(String::from_utf8_lossy(&output).into_owned())
    }
}
