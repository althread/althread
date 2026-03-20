use crate::error::Pos;

#[derive(Debug, Clone)]
pub struct SyntaxProgram {
    pub blocks: Vec<SyntaxBlock>,
}

#[derive(Debug, Clone)]
pub struct SyntaxSnippet {
    pub pos: Pos,
    pub text: String,
}

impl SyntaxSnippet {
    pub fn new(pos: Pos, text: String) -> Self {
        Self { pos, text }
    }
}

#[derive(Debug, Clone)]
pub struct SyntaxBlock {
    pub kind: SyntaxBlockKind,
    pub pos: Pos,
    pub text: String,
    pub detail: SyntaxBlockDetail,
}

impl SyntaxBlock {
    pub fn new(kind: SyntaxBlockKind, pos: Pos, text: String) -> Self {
        Self {
            kind,
            pos,
            text,
            detail: SyntaxBlockDetail::Opaque,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SyntaxBlockDetail {
    Opaque,
    Main {
        body_pos: Pos,
        body: Vec<SyntaxSnippet>,
    },
    Global {
        body_pos: Pos,
        body: Vec<SyntaxSnippet>,
    },
    Condition {
        body_pos: Pos,
        body: Vec<SyntaxSnippet>,
    },
    Check {
        body_pos: Pos,
        formulas: Vec<SyntaxSnippet>,
    },
    Program {
        is_private: bool,
        name: SyntaxSnippet,
        args: SyntaxSnippet,
        body_pos: Pos,
        body: Vec<SyntaxSnippet>,
    },
    Function {
        is_private: bool,
        name: SyntaxSnippet,
        args: SyntaxSnippet,
        return_type: SyntaxSnippet,
        body_pos: Pos,
        body: Vec<SyntaxSnippet>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxBlockKind {
    Import,
    Main,
    Global,
    Always,
    Never,
    Check,
    Program,
    Function,
}
