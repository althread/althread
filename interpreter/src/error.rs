use ariadne::{Color, Label, Report, ReportKind, Source};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc, thread_local};

#[derive(Debug, Clone)]
struct SourceFileContext {
    source: String,
    line_starts: Vec<usize>,
}

thread_local! {
    static SOURCE_MAP: RefCell<HashMap<String, SourceFileContext>> = RefCell::new(HashMap::new());
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
    pub file_path: String,
}

pub use SourceSpan as Pos;

impl Default for SourceSpan {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            file_path: "".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPos {
    pub line: usize,
    pub col: usize,
}

fn build_line_starts(source: &str) -> Vec<usize> {
    let mut line_starts = vec![0usize];
    for (idx, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            line_starts.push(idx + 1);
        }
    }
    line_starts
}

fn resolve_in_context(ctx: &SourceFileContext, offset: usize) -> ResolvedPos {
    let safe_offset = offset.min(ctx.source.len());
    let line_idx = ctx
        .line_starts
        .partition_point(|&line_start| line_start <= safe_offset)
        .saturating_sub(1);
    let line_start = ctx.line_starts.get(line_idx).copied().unwrap_or(0);
    ResolvedPos {
        line: line_idx + 1,
        col: safe_offset.saturating_sub(line_start) + 1,
    }
}

pub fn register_source(file_path: &str, source: &str) {
    SOURCE_MAP.with(|map| {
        map.borrow_mut().insert(
            file_path.to_string(),
            SourceFileContext {
                source: source.to_string(),
                line_starts: build_line_starts(source),
            },
        );
    });
}

pub fn lookup_source(file_path: &str) -> Option<String> {
    SOURCE_MAP.with(|map| map.borrow().get(file_path).map(|ctx| ctx.source.clone()))
}

impl SourceSpan {
    pub fn new(file_path: impl Into<String>, start: usize, end: usize) -> Self {
        let file_path = file_path.into();
        let safe_end = end.max(start);
        Self {
            start,
            end: safe_end,
            file_path,
        }
    }

    pub fn from_offsets(source: &str, file_path: &str, start: usize, end: usize) -> Self {
        register_source(file_path, source);
        let safe_start = start.min(source.len());
        let safe_end = end.min(source.len()).max(safe_start);
        Self::new(file_path.to_string(), safe_start, safe_end)
    }

    pub fn resolve(&self) -> Option<ResolvedPos> {
        SOURCE_MAP.with(|map| {
            map.borrow()
                .get(&self.file_path)
                .map(|ctx| resolve_in_context(ctx, self.start))
        })
    }

    pub fn line_col(&self) -> Option<(usize, usize)> {
        self.resolve().map(|resolved| (resolved.line, resolved.col))
    }

    pub fn line(&self) -> usize {
        self.resolve().map(|resolved| resolved.line).unwrap_or(0)
    }

    pub fn column(&self) -> usize {
        self.resolve().map(|resolved| resolved.col).unwrap_or(0)
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
            Some($crate::error::Pos::new(
                $filename.to_string(),
                $pair.as_span().start(),
                $pair.as_span().end(),
            )),
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
                    let line = pos.line();
                    let col = pos.column();
                    if !pos.file_path.is_empty() {
                        eprintln!("Error in {} at {}:{}", pos.file_path, line, col);
                    } else {
                        eprintln!("Error at {}:{}", line, col);
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
                let line = pos.line();
                let col = pos.column();
                if !pos.file_path.is_empty() {
                    eprintln!("  at {}:{}:{}", pos.file_path, line, col);
                } else {
                    eprintln!("  at {}:{}", line, col);
                }
            }
        }
    }

    fn print_err_line(&self, input_map: &HashMap<String, String>) {
        if let Some(pos) = &self.pos {
            let file_path = &pos.file_path;
            let input = lookup_source(file_path).or_else(|| input_map.get(file_path).cloned());
            let Some(input) = input else {
                return;
            };
            let line_number = pos.line();
            let column = pos.column();
            let line = match input.lines().nth(line_number.saturating_sub(1)) {
                Some(line) => line.to_string(),
                None => return,
            };

            let line_indent = " ".repeat(line_number.to_string().len());
            eprintln!("{} |", line_indent);
            eprintln!("{} | {}", line_number, line);
            eprintln!("{} |{}^---", line_indent, " ".repeat(column));
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
        let source = lookup_source(&pos.file_path).or_else(|| {
            if pos.file_path.is_empty() {
                input_map
                    .get("")
                    .or_else(|| input_map.values().next())
                    .cloned()
            } else {
                input_map.get(&pos.file_path).cloned()
            }
        })?;

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
