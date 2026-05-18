use chumsky::span::{SimpleSpan, Span as _};
use std::fmt;

use crate::{
    error::{AlthreadError, ErrorType, Pos},
    parser::syntax::SyntaxSnippet,
};

pub type Span = SimpleSpan<usize>;
pub type Spanned<T> = (T, Span);

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    AtPrivate,
    At,
    Shared,
    Main,
    Always,
    Check,
    Eventually,
    Until,
    Program,
    Fn,
    Import,
    As,
    Let,
    Const,
    Run,
    Send,
    Channel,
    Await,
    First,
    Seq,
    Receive,
    If,
    Else,
    While,
    For,
    Exists,
    In,
    Loop,
    Atomic,
    Break,
    Continue,
    Return,
    Label,
    Proc,
    List,
    Tuple,
    BoolType,
    IntType,
    FloatType,
    StringType,
    VoidType,
    True,
    False,
    Null,
    Dollar,
    StringLiteral(String),
    FloatLiteral(String),
    IntLiteral(String),
    Ident(String),
    Arrow,
    FatArrow,
    EqEq,
    NotEq,
    LtEq,
    GtEq,
    ShiftLeft,
    ShiftRight,
    DotDot,
    AndAnd,
    OrOr,
    Bang,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    Pipe,
    Eq,
    Dot,
    Comma,
    Colon,
    Semi,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Lt,
    Gt,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::AtPrivate => write!(f, "@private"),
            Token::At => write!(f, "@"),
            Token::Shared => write!(f, "shared"),
            Token::Main => write!(f, "main"),
            Token::Always => write!(f, "always"),
            Token::Check => write!(f, "check"),
            Token::Eventually => write!(f, "eventually"),
            Token::Until => write!(f, "until"),
            Token::Program => write!(f, "program"),
            Token::Fn => write!(f, "fn"),
            Token::Import => write!(f, "import"),
            Token::As => write!(f, "as"),
            Token::Let => write!(f, "let"),
            Token::Const => write!(f, "const"),
            Token::Run => write!(f, "run"),
            Token::Send => write!(f, "send"),
            Token::Channel => write!(f, "channel"),
            Token::Await => write!(f, "await"),
            Token::First => write!(f, "first"),
            Token::Seq => write!(f, "seq"),
            Token::Receive => write!(f, "receive"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::While => write!(f, "while"),
            Token::For => write!(f, "for"),
            Token::Exists => write!(f, "exists"),
            Token::In => write!(f, "in"),
            Token::Loop => write!(f, "loop"),
            Token::Atomic => write!(f, "atomic"),
            Token::Break => write!(f, "break"),
            Token::Continue => write!(f, "continue"),
            Token::Return => write!(f, "return"),
            Token::Label => write!(f, "label"),
            Token::Proc => write!(f, "proc"),
            Token::List => write!(f, "list"),
            Token::Tuple => write!(f, "tuple"),
            Token::BoolType => write!(f, "bool"),
            Token::IntType => write!(f, "int"),
            Token::FloatType => write!(f, "float"),
            Token::StringType => write!(f, "string"),
            Token::VoidType => write!(f, "void"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Null => write!(f, "null"),
            Token::Dollar => write!(f, "$"),
            Token::StringLiteral(_) => write!(f, "string literal"),
            Token::FloatLiteral(_) => write!(f, "float literal"),
            Token::IntLiteral(_) => write!(f, "int literal"),
            Token::Ident(name) => write!(f, "{name}"),
            Token::Arrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            Token::EqEq => write!(f, "=="),
            Token::NotEq => write!(f, "!="),
            Token::LtEq => write!(f, "<="),
            Token::GtEq => write!(f, ">="),
            Token::ShiftLeft => write!(f, "<<"),
            Token::ShiftRight => write!(f, ">>"),
            Token::DotDot => write!(f, ".."),
            Token::AndAnd => write!(f, "&&"),
            Token::OrOr => write!(f, "||"),
            Token::Bang => write!(f, "!"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Amp => write!(f, "&"),
            Token::Pipe => write!(f, "|"),
            Token::Eq => write!(f, "="),
            Token::Dot => write!(f, "."),
            Token::Comma => write!(f, ","),
            Token::Colon => write!(f, ":"),
            Token::Semi => write!(f, ";"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
        }
    }
}

pub fn lex(source: &str, file_path: &str) -> Result<Vec<Spanned<Token>>, AlthreadError> {
    lex_internal(source, file_path, 0)
}

pub fn lex_snippet(
    snippet: &SyntaxSnippet,
    file_path: &str,
) -> Result<Vec<Spanned<Token>>, AlthreadError> {
    lex_internal_with_base(&snippet.text, file_path, snippet.pos.start, Some(0))
}

fn lex_internal(
    source: &str,
    file_path: &str,
    base_offset: usize,
) -> Result<Vec<Spanned<Token>>, AlthreadError> {
    lex_internal_with_base(source, file_path, base_offset, None)
}

fn lex_internal_with_base(
    source: &str,
    file_path: &str,
    base_offset: usize,
    local_error_base: Option<usize>,
) -> Result<Vec<Spanned<Token>>, AlthreadError> {
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < bytes.len() {
        let b = bytes[i];

        if matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0C) {
            i += 1;
            continue;
        }

        if b == b'/' && i + 1 < bytes.len() {
            if bytes[i + 1] == b'/' {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            if bytes[i + 1] == b'*' {
                let start = i;
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                if i + 1 >= bytes.len() {
                    return Err(invalid_token_error(
                        source,
                        file_path,
                        local_error_base.unwrap_or(base_offset),
                        start,
                        bytes.len(),
                    ));
                }
                i += 2;
                continue;
            }
        }

        let start = i;

        let token = match b {
            b'@' => {
                if source[start..].starts_with("@private")
                    && !is_ident_continue(peek_byte(bytes, start + 8))
                {
                    i += 8;
                    Token::AtPrivate
                } else {
                    i += 1;
                    Token::At
                }
            }
            b'$' => {
                i += 1;
                Token::Dollar
            }
            b'"' => {
                i += 1;
                let mut escaped = false;
                while i < bytes.len() {
                    let cur = bytes[i];
                    if escaped {
                        escaped = false;
                        i += 1;
                        continue;
                    }
                    if cur == b'\\' {
                        escaped = true;
                        i += 1;
                        continue;
                    }
                    if cur == b'"' {
                        i += 1;
                        break;
                    }
                    if cur == b'\n' {
                        return Err(invalid_token_error(
                            source,
                            file_path,
                            local_error_base.unwrap_or(base_offset),
                            start,
                            i,
                        ));
                    }
                    i += 1;
                }
                if i > bytes.len() || bytes.get(i.wrapping_sub(1)) != Some(&b'"') {
                    return Err(invalid_token_error(
                        source,
                        file_path,
                        local_error_base.unwrap_or(base_offset),
                        start,
                        bytes.len(),
                    ));
                }
                Token::StringLiteral(source[start..i].to_string())
            }
            b'0'..=b'9' => {
                if b == b'0' && i + 1 < bytes.len() {
                    match bytes[i + 1] {
                        b'x' | b'X' => {
                            i += 2;
                            let hex_start = i;
                            while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                                i += 1;
                            }
                            if i == hex_start {
                                return Err(invalid_token_error(
                                    source,
                                    file_path,
                                    local_error_base.unwrap_or(base_offset),
                                    start,
                                    i.min(bytes.len()),
                                ));
                            }
                            Token::IntLiteral(source[start..i].to_string())
                        }
                        b'b' | b'B' => {
                            i += 2;
                            let bin_start = i;
                            while i < bytes.len() && matches!(bytes[i], b'0' | b'1') {
                                i += 1;
                            }
                            if i == bin_start {
                                return Err(invalid_token_error(
                                    source,
                                    file_path,
                                    local_error_base.unwrap_or(base_offset),
                                    start,
                                    i.min(bytes.len()),
                                ));
                            }
                            Token::IntLiteral(source[start..i].to_string())
                        }
                        _ => {
                            i += 1;
                            while i < bytes.len() && bytes[i].is_ascii_digit() {
                                i += 1;
                            }
                            if i < bytes.len()
                                && bytes[i] == b'.'
                                && i + 1 < bytes.len()
                                && bytes[i + 1].is_ascii_digit()
                            {
                                i += 2;
                                while i < bytes.len() && bytes[i].is_ascii_digit() {
                                    i += 1;
                                }
                                Token::FloatLiteral(source[start..i].to_string())
                            } else {
                                Token::IntLiteral(source[start..i].to_string())
                            }
                        }
                    }
                } else {
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                    if i < bytes.len()
                        && bytes[i] == b'.'
                        && i + 1 < bytes.len()
                        && bytes[i + 1].is_ascii_digit()
                    {
                        i += 2;
                        while i < bytes.len() && bytes[i].is_ascii_digit() {
                            i += 1;
                        }
                        Token::FloatLiteral(source[start..i].to_string())
                    } else {
                        Token::IntLiteral(source[start..i].to_string())
                    }
                }
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                i += 1;
                while i < bytes.len() && is_ident_continue(Some(bytes[i])) {
                    i += 1;
                }
                keyword_or_ident(&source[start..i])
            }
            b'-' => {
                if peek_byte(bytes, i + 1) == Some(b'>') {
                    i += 2;
                    Token::Arrow
                } else {
                    i += 1;
                    Token::Minus
                }
            }
            b'=' => {
                if peek_byte(bytes, i + 1) == Some(b'>') {
                    i += 2;
                    Token::FatArrow
                } else if peek_byte(bytes, i + 1) == Some(b'=') {
                    i += 2;
                    Token::EqEq
                } else {
                    i += 1;
                    Token::Eq
                }
            }
            b'!' => {
                if peek_byte(bytes, i + 1) == Some(b'=') {
                    i += 2;
                    Token::NotEq
                } else {
                    i += 1;
                    Token::Bang
                }
            }
            b'<' => {
                if peek_byte(bytes, i + 1) == Some(b'=') {
                    i += 2;
                    Token::LtEq
                } else if peek_byte(bytes, i + 1) == Some(b'<') {
                    i += 2;
                    Token::ShiftLeft
                } else {
                    i += 1;
                    Token::Lt
                }
            }
            b'>' => {
                if peek_byte(bytes, i + 1) == Some(b'=') {
                    i += 2;
                    Token::GtEq
                } else if peek_byte(bytes, i + 1) == Some(b'>') {
                    i += 2;
                    Token::ShiftRight
                } else {
                    i += 1;
                    Token::Gt
                }
            }
            b'.' => {
                if peek_byte(bytes, i + 1) == Some(b'.') {
                    i += 2;
                    Token::DotDot
                } else {
                    i += 1;
                    Token::Dot
                }
            }
            b'&' => {
                if peek_byte(bytes, i + 1) == Some(b'&') {
                    i += 2;
                    Token::AndAnd
                } else {
                    i += 1;
                    Token::Amp
                }
            }
            b'|' => {
                if peek_byte(bytes, i + 1) == Some(b'|') {
                    i += 2;
                    Token::OrOr
                } else {
                    i += 1;
                    Token::Pipe
                }
            }
            b'+' => {
                i += 1;
                Token::Plus
            }
            b'*' => {
                i += 1;
                Token::Star
            }
            b'/' => {
                i += 1;
                Token::Slash
            }
            b'%' => {
                i += 1;
                Token::Percent
            }
            b',' => {
                i += 1;
                Token::Comma
            }
            b':' => {
                i += 1;
                Token::Colon
            }
            b';' => {
                i += 1;
                Token::Semi
            }
            b'(' => {
                i += 1;
                Token::LParen
            }
            b')' => {
                i += 1;
                Token::RParen
            }
            b'{' => {
                i += 1;
                Token::LBrace
            }
            b'}' => {
                i += 1;
                Token::RBrace
            }
            b'[' => {
                i += 1;
                Token::LBracket
            }
            b']' => {
                i += 1;
                Token::RBracket
            }
            _ => {
                return Err(invalid_token_error(
                    source,
                    file_path,
                    local_error_base.unwrap_or(base_offset),
                    start,
                    (start + 1).min(bytes.len()),
                ));
            }
        };

        tokens.push((
            token,
            Span::new((), (base_offset + start)..(base_offset + i)),
        ));
    }

    Ok(tokens)
}

fn keyword_or_ident(text: &str) -> Token {
    match text {
        "shared" => Token::Shared,
        "main" => Token::Main,
        "always" => Token::Always,
        "check" => Token::Check,
        "eventually" => Token::Eventually,
        "until" => Token::Until,
        "program" => Token::Program,
        "fn" => Token::Fn,
        "import" => Token::Import,
        "as" => Token::As,
        "let" => Token::Let,
        "const" => Token::Const,
        "run" => Token::Run,
        "send" => Token::Send,
        "channel" => Token::Channel,
        "await" => Token::Await,
        "first" => Token::First,
        "seq" => Token::Seq,
        "receive" => Token::Receive,
        "if" => Token::If,
        "else" => Token::Else,
        "while" => Token::While,
        "for" => Token::For,
        "exists" => Token::Exists,
        "in" => Token::In,
        "loop" => Token::Loop,
        "atomic" => Token::Atomic,
        "break" => Token::Break,
        "continue" => Token::Continue,
        "return" => Token::Return,
        "label" => Token::Label,
        "proc" => Token::Proc,
        "list" => Token::List,
        "tuple" => Token::Tuple,
        "bool" => Token::BoolType,
        "int" => Token::IntType,
        "float" => Token::FloatType,
        "string" => Token::StringType,
        "void" => Token::VoidType,
        "true" => Token::True,
        "false" => Token::False,
        "null" => Token::Null,
        _ => Token::Ident(text.to_string()),
    }
}

fn invalid_token_error(
    source: &str,
    file_path: &str,
    pos_base: usize,
    start: usize,
    end: usize,
) -> AlthreadError {
    let safe_start = start.min(source.len());
    let safe_end = end.min(source.len()).max(safe_start + 1).min(source.len());
    AlthreadError::new(
        ErrorType::SyntaxError,
        Some(Pos::from_offsets(
            source,
            file_path,
            pos_base + safe_start,
            pos_base + safe_end,
        )),
        format!("invalid token '{}'", &source[safe_start..safe_end]),
    )
}

fn peek_byte(bytes: &[u8], index: usize) -> Option<u8> {
    bytes.get(index).copied()
}

fn is_ident_continue(byte: Option<u8>) -> bool {
    matches!(byte, Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_'))
}
