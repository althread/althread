use std::fmt;

use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::PrattParser;

use crate::{
    ast::{
        node::{Node, NodeBuilder},
        statement::expression::{list_expression::RangeListExpression, primary_expression::PrimaryExpression, Expression},
    },
    error::{AlthreadError, AlthreadResult, ErrorType, Pos},
    no_rule,
    parser::{parse_expression_with_chumsky, parse_list_expression_with_chumsky, syntax::SyntaxSnippet, Rule},
};

lazy_static::lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};

        PrattParser::new()
            .op(Op::infix(Rule::OR_OP, Left))
            .op(Op::infix(Rule::AND_OP, Left))
            .op(Op::infix(Rule::UNTIL_KW, Left))
    };
}

/// Represents an LTL formula
#[derive(Debug, Clone, PartialEq)]
pub enum LtlExpression {
    Always(Box<LtlExpression>),
    Eventually(Box<LtlExpression>),
    Next(Box<LtlExpression>),
    Not(Box<LtlExpression>),
    Until(Box<LtlExpression>, Box<LtlExpression>),
    And(Box<LtlExpression>, Box<LtlExpression>),
    Or(Box<LtlExpression>, Box<LtlExpression>),
    Implies(Box<LtlExpression>, Box<LtlExpression>),
    Predicate(Node<Expression>),
    ForLoop {
        var_name: String,
        list: Node<Expression>,
        body: Box<LtlExpression>,
    },
}

/// A list of LTL formulas defined in a check block
#[derive(Debug, Clone)]
pub struct CheckBlock {
    pub formulas: Vec<LtlExpression>,
}

impl NodeBuilder for CheckBlock {
    fn build(pairs: Pairs<Rule>, filepath: &str) -> AlthreadResult<Self> {
        let mut formulas = Vec::new();

        for pair in pairs {
            match pair.as_rule() {
                Rule::ltl_statement => {
                    // formulas.push(build_ltl_statement(pair, filepath)?);
                    let mut inner = pair.into_inner();
                    let expr_pair = inner.next().unwrap();
                    let formula = build_ltl_expression(expr_pair, filepath)?;
                    formulas.push(formula);
                }
                _ => {}
            }
        }

        Ok(Self { formulas })
    }
}
/*
fn build_ltl_statement(pair: Pair<Rule>, filepath: &str) -> AlthreadResult<LtlExpression> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::ltl_for_loop => build_ltl_for_loop(inner, filepath),
        Rule::ltl_expression => build_ltl_expression(inner, filepath),
        _ => unreachable!("Invalid ltl statement"),
    }
}
*/
fn build_ltl_for_loop(pair: Pair<Rule>, filepath: &str) -> AlthreadResult<LtlExpression> {
    let mut inner = pair.into_inner();
    let ident_pair = inner.next().unwrap();
    let var_name = ident_pair.as_str().to_string();

    let list_pair = inner.next().unwrap();
    let list_node = match list_pair.as_rule() {
        Rule::range_expression => {
            let range = Node::<RangeListExpression>::build(list_pair, filepath)?;
            Node {
                pos: range.pos.clone(),
                value: Expression::Range(range),
            }
        }
        Rule::expression => Node::<Expression>::build(list_pair, filepath)?,
        _ => unreachable!("Invalid list expression"),
    };

    /*
    let mut body = Vec::new();
    for stmt_pair in inner {
        if stmt_pair.as_rule() == Rule::ltl_statement {
            body.push(build_ltl_statement(stmt_pair, filepath)?);
        }
    }
    */
    let expr_pair = inner.next().unwrap();
    let body = build_ltl_expression(expr_pair, filepath)?;

    Ok(LtlExpression::ForLoop {
        var_name,
        list: list_node,
        body: Box::new(body),
    })
}

fn build_ltl_expression(pair: Pair<Rule>, filepath: &str) -> AlthreadResult<LtlExpression> {
    match pair.as_rule() {
        Rule::ltl_expression => {
            let pairs = pair.into_inner();
            PRATT_PARSER
                .map_primary(|primary| build_ltl_term(primary, filepath))
                .map_infix(|lhs, op, rhs| {
                    let lhs = lhs?;
                    let rhs = rhs?;
                    match op.as_rule() {
                        Rule::OR_OP => Ok(LtlExpression::Or(Box::new(lhs), Box::new(rhs))),
                        Rule::AND_OP => Ok(LtlExpression::And(Box::new(lhs), Box::new(rhs))),
                        Rule::UNTIL_KW => Ok(LtlExpression::Until(Box::new(lhs), Box::new(rhs))),
                        _ => unreachable!("Invalid binary operator"),
                    }
                })
                .parse(pairs)
        }
        _ => Err(no_rule!(pair, "LtlExpression", filepath)),
    }
}

pub(crate) fn parse_ltl_expression_with_chumsky(
    source: &str,
    snippet: &SyntaxSnippet,
    filepath: &str,
) -> AlthreadResult<LtlExpression> {
    LtlParser::new(source, snippet, filepath).parse_expression(0)
}

fn build_ltl_term(pair: Pair<Rule>, filepath: &str) -> AlthreadResult<LtlExpression> {
    match pair.as_rule() {
        Rule::ltl_term => build_ltl_term(pair.into_inner().next().unwrap(), filepath),
        Rule::ltl_for_loop => build_ltl_for_loop(pair, filepath),
        Rule::ltl_unary_expression => {
            let mut inner = pair.into_inner();
            let op = inner.next().unwrap();
            let expr = inner.next().unwrap();
            let built_expr = build_ltl_term(expr, filepath)?;

            match op.as_rule() {
                Rule::ALWAYS_KW => Ok(LtlExpression::Always(Box::new(built_expr))),
                Rule::EVENTUALLY_KW => Ok(LtlExpression::Eventually(Box::new(built_expr))),
                Rule::NOT_OP => Ok(LtlExpression::Not(Box::new(built_expr))),
                _ => unreachable!("Invalid unary operator"),
            }
        }
        Rule::ltl_if_expression => {
            let mut inner = pair.into_inner();
            let cond = build_ltl_expression(inner.next().unwrap(), filepath)?;
            let then_branch = build_ltl_expression(inner.next().unwrap(), filepath)?;

            if let Some(else_pair) = inner.next() {
                let else_branch = build_ltl_expression(else_pair, filepath)?;
                let cond_box = Box::new(cond);
                // (cond -> then_branch) && (!cond -> else_branch)
                let implies_then = LtlExpression::Implies(cond_box.clone(), Box::new(then_branch));
                let not_cond = LtlExpression::Not(cond_box);
                let implies_else =
                    LtlExpression::Implies(Box::new(not_cond), Box::new(else_branch));
                Ok(LtlExpression::And(
                    Box::new(implies_then),
                    Box::new(implies_else),
                ))
            } else {
                Ok(LtlExpression::Implies(
                    Box::new(cond),
                    Box::new(then_branch),
                ))
            }
        }
        Rule::ltl_predicate => {
            let expr_pair = pair.into_inner().next().unwrap(); // expression
            let expr_node = Node::<Expression>::build(expr_pair, filepath)?;
            Ok(LtlExpression::Predicate(expr_node))
        }
        Rule::ltl_expression => {
            // Parenthesized expression
            build_ltl_expression(pair, filepath)
        }
        _ => Err(no_rule!(pair, "LtlTerm", filepath)),
    }
}

impl fmt::Display for LtlExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LtlExpression::Always(e) => write!(f, "[] ({})", e),
            LtlExpression::Eventually(e) => write!(f, "<> ({})", e),
            LtlExpression::Next(e) => write!(f, "X ({})", e),
            LtlExpression::Not(e) => write!(f, "! ({})", e),
            LtlExpression::Until(l, r) => write!(f, "({}) U ({})", l, r),
            LtlExpression::And(l, r) => write!(f, "({}) && ({})", l, r),
            LtlExpression::Or(l, r) => write!(f, "({}) || ({})", l, r),
            LtlExpression::Implies(l, r) => write!(f, "({}) -> ({})", l, r),
            LtlExpression::Predicate(e) => write!(f, "{}", e),
            LtlExpression::ForLoop {
                var_name,
                list,
                body,
            } => {
                write!(f, "for {} in {} {{ {}; }}", var_name, list.value, body)
            }
        }
    }
}

struct LtlParser<'a> {
    source: &'a str,
    file_path: &'a str,
    snippet: &'a SyntaxSnippet,
    text: &'a str,
    index: usize,
}

impl<'a> LtlParser<'a> {
    fn new(source: &'a str, snippet: &'a SyntaxSnippet, file_path: &'a str) -> Self {
        Self {
            source,
            file_path,
            snippet,
            text: snippet.text.as_str(),
            index: 0,
        }
    }

    fn parse_expression(&mut self, min_prec: u8) -> AlthreadResult<LtlExpression> {
        skip_ws(self.text, &mut self.index);
        let mut left = self.parse_term()?;

        loop {
            skip_ws(self.text, &mut self.index);
            let Some((precedence, op_kind, len)) = self.peek_binary_operator() else {
                break;
            };
            if precedence < min_prec {
                break;
            }
            self.index += len;
            let right = self.parse_expression(precedence + 1)?;
            left = match op_kind {
                LtlBinaryOperator::Or => LtlExpression::Or(Box::new(left), Box::new(right)),
                LtlBinaryOperator::And => LtlExpression::And(Box::new(left), Box::new(right)),
                LtlBinaryOperator::Until => LtlExpression::Until(Box::new(left), Box::new(right)),
            };
        }

        skip_ws(self.text, &mut self.index);
        if min_prec == 0 && self.index != self.text.len() {
            return Err(ltl_error(
                self.source,
                self.file_path,
                self.snippet.pos.start + self.index,
                self.snippet.pos.end,
                "unexpected trailing input in LTL expression",
            ));
        }

        Ok(left)
    }

    fn parse_term(&mut self) -> AlthreadResult<LtlExpression> {
        skip_ws(self.text, &mut self.index);
        if self.index >= self.text.len() {
            return Err(ltl_error(
                self.source,
                self.file_path,
                self.snippet.pos.start,
                self.snippet.pos.end,
                "expected LTL term",
            ));
        }

        if self.text.as_bytes()[self.index] == b'(' {
            let end = consume_balanced(self.text, self.index, b'(', b')')?;
            let inner = trimmed_snippet(
                self.source,
                self.file_path,
                self.snippet,
                self.index + 1,
                end - 1,
            );
            self.index = end;
            return parse_ltl_expression_with_chumsky(self.source, &inner, self.file_path);
        }

        if consume_keyword(self.text, &mut self.index, "if") {
            return self.parse_if_expression();
        }
        if consume_keyword(self.text, &mut self.index, "for") {
            return self.parse_for_expression();
        }
        if consume_keyword(self.text, &mut self.index, "always") {
            let expr = match self.try_parse_predicate_operand()? {
                Some(expr) => LtlExpression::Predicate(expr),
                None => self.parse_term()?,
            };
            return Ok(LtlExpression::Always(Box::new(expr)));
        }
        if consume_keyword(self.text, &mut self.index, "eventually") {
            let expr = match self.try_parse_predicate_operand()? {
                Some(expr) => LtlExpression::Predicate(expr),
                None => self.parse_term()?,
            };
            return Ok(LtlExpression::Eventually(Box::new(expr)));
        }
        if self.text.as_bytes().get(self.index) == Some(&b'!') {
            self.index += 1;
            let expr = match self.try_parse_predicate_operand()? {
                Some(expr) => LtlExpression::Predicate(expr),
                None => self.parse_term()?,
            };
            return Ok(LtlExpression::Not(Box::new(expr)));
        }

        self.parse_predicate()
    }

    fn parse_if_expression(&mut self) -> AlthreadResult<LtlExpression> {
        let cond_start = self.index;
        let then_block_start = find_top_level_block_start(self.text, cond_start).ok_or_else(|| {
            ltl_error(
                self.source,
                self.file_path,
                self.snippet.pos.start + cond_start,
                self.snippet.pos.end,
                "expected '{' after if condition",
            )
        })?;
        let cond_snippet = trimmed_snippet(
            self.source,
            self.file_path,
            self.snippet,
            cond_start,
            then_block_start,
        );
        let cond = match parse_expression_with_chumsky(self.source, &cond_snippet, self.file_path) {
            Ok(mut expr) => {
                expr = normalize_ltl_predicate_expression(expr);
                let trimmed = cond_snippet.text.trim_start();
                if !trimmed.starts_with('(')
                    && !cond_snippet.text.contains("&&")
                    && !cond_snippet.text.contains("||")
                {
                    expr.pos.end = self.snippet.pos.start + then_block_start;
                    expand_identifier_predicate_end(&mut expr, self.snippet.pos.start + then_block_start);
                }
                LtlExpression::Predicate(expr)
            }
            Err(_) => parse_ltl_expression_with_chumsky(self.source, &cond_snippet, self.file_path)?,
        };
        let (then_expr, then_end) = parse_ltl_block_expression(
            self.source,
            self.file_path,
            self.snippet,
            self.text,
            then_block_start,
        )?;
        self.index = then_end;
        skip_ws(self.text, &mut self.index);

        if consume_keyword(self.text, &mut self.index, "else") {
            skip_ws(self.text, &mut self.index);
            let (else_expr, else_end) = parse_ltl_block_expression(
                self.source,
                self.file_path,
                self.snippet,
                self.text,
                self.index,
            )?;
            self.index = else_end;
            let cond_box = Box::new(cond);
            let implies_then = LtlExpression::Implies(cond_box.clone(), Box::new(then_expr));
            let not_cond = LtlExpression::Not(cond_box);
            let implies_else = LtlExpression::Implies(Box::new(not_cond), Box::new(else_expr));
            Ok(LtlExpression::And(
                Box::new(implies_then),
                Box::new(implies_else),
            ))
        } else {
            Ok(LtlExpression::Implies(Box::new(cond), Box::new(then_expr)))
        }
    }

    fn parse_for_expression(&mut self) -> AlthreadResult<LtlExpression> {
        skip_ws(self.text, &mut self.index);
        let ident_start = self.index;
        self.index = consume_identifier(self.text, self.index).ok_or_else(|| {
            ltl_error(
                self.source,
                self.file_path,
                self.snippet.pos.start + ident_start,
                self.snippet.pos.end,
                "expected identifier after 'for'",
            )
        })?;
        let var_name = self.text[ident_start..self.index].to_string();
        skip_ws(self.text, &mut self.index);
        if !consume_keyword(self.text, &mut self.index, "in") {
            return Err(ltl_error(
                self.source,
                self.file_path,
                self.snippet.pos.start + self.index,
                self.snippet.pos.end,
                "expected 'in' in LTL for loop",
            ));
        }

        let list_start = self.index;
        let body_block_start = find_top_level_block_start(self.text, list_start).ok_or_else(|| {
            ltl_error(
                self.source,
                self.file_path,
                self.snippet.pos.start + list_start,
                self.snippet.pos.end,
                "expected '{' after LTL list expression",
            )
        })?;
        let list_snippet =
            trimmed_snippet(self.source, self.file_path, self.snippet, list_start, body_block_start);
        let mut list =
            parse_list_expression_with_chumsky(self.source, &list_snippet, self.file_path)?;
        list.pos = Pos::from_offsets(
            self.source,
            self.file_path,
            list.pos.start,
            self.snippet.pos.start + body_block_start,
        );
        let (body, body_end) = parse_ltl_block_expression(
            self.source,
            self.file_path,
            self.snippet,
            self.text,
            body_block_start,
        )?;
        self.index = body_end;

        Ok(LtlExpression::ForLoop {
            var_name,
            list,
            body: Box::new(body),
        })
    }

    fn parse_predicate(&mut self) -> AlthreadResult<LtlExpression> {
        let start = self.index;
        let mut end = scan_ltl_predicate_end(self.text, start);
        while end < self.text.len() && self.text.as_bytes()[end].is_ascii_whitespace() {
            end += 1;
        }
        let mut expr = parse_expression_with_chumsky(
            self.source,
            &start_trimmed_snippet(self.source, self.file_path, self.snippet, start, end),
            self.file_path,
        )?;
        expr = normalize_ltl_predicate_expression(expr);
        expand_identifier_predicate_end(&mut expr, self.snippet.pos.start + end);
        self.index = end;
        Ok(LtlExpression::Predicate(expr))
    }

    fn try_parse_predicate_operand(&mut self) -> AlthreadResult<Option<Node<Expression>>> {
        let start = self.index;
        let mut probe = start;
        skip_ws(self.text, &mut probe);
        if self.text.as_bytes().get(probe) == Some(&b'!')
            || starts_with_word(self.text, probe, "always")
            || starts_with_word(self.text, probe, "eventually")
            || starts_with_word(self.text, probe, "if")
            || starts_with_word(self.text, probe, "for")
        {
            return Ok(None);
        }
        let end = if self.text.as_bytes().get(probe) == Some(&b'(') {
            let mut inner_probe = probe + 1;
            skip_ws(self.text, &mut inner_probe);
            if self.text.as_bytes().get(inner_probe) == Some(&b'!')
                || starts_with_word(self.text, inner_probe, "always")
                || starts_with_word(self.text, inner_probe, "eventually")
                || starts_with_word(self.text, inner_probe, "if")
                || starts_with_word(self.text, inner_probe, "for")
            {
                return Ok(None);
            }
            let paren_end = consume_balanced(self.text, probe, b'(', b')')?;
            let inner_text = &self.text[probe + 1..paren_end - 1];
            if inner_text.contains('!')
                || inner_text.contains("eventually")
                || inner_text.contains("always")
                || inner_text.contains("until")
                || inner_text.contains("||")
                || inner_text.contains(" if ")
                || inner_text.contains(" for ")
            {
                return Ok(None);
            }
            paren_end
        } else {
            scan_group_end(self.text, start)
        };
        let snippet = trimmed_snippet(self.source, self.file_path, self.snippet, start, end);
        match parse_expression_with_chumsky(self.source, &snippet, self.file_path) {
            Ok(expr) => {
                let preserve_end = !matches!(
                    expr.value,
                    Expression::Primary(ref primary)
                        if matches!(primary.value, PrimaryExpression::Expression(_))
                );
                let mut expr = normalize_ltl_predicate_expression(expr);
                if preserve_end {
                    expr.pos = Pos::from_offsets(
                        self.source,
                        self.file_path,
                        expr.pos.start,
                        self.snippet.pos.start + end,
                    );
                    expand_identifier_predicate_end(&mut expr, self.snippet.pos.start + end);
                }
                self.index = end;
                Ok(Some(expr))
            }
            Err(_) => Ok(None),
        }
    }

    fn peek_binary_operator(&self) -> Option<(u8, LtlBinaryOperator, usize)> {
        if self.text[self.index..].starts_with("||") {
            return Some((1, LtlBinaryOperator::Or, 2));
        }
        if self.text[self.index..].starts_with("&&") {
            return Some((2, LtlBinaryOperator::And, 2));
        }
        if starts_with_word(self.text, self.index, "until") {
            return Some((3, LtlBinaryOperator::Until, "until".len()));
        }
        None
    }
}

#[derive(Clone, Copy)]
enum LtlBinaryOperator {
    Or,
    And,
    Until,
}

fn parse_ltl_block_expression(
    source: &str,
    file_path: &str,
    outer: &SyntaxSnippet,
    text: &str,
    block_start: usize,
) -> AlthreadResult<(LtlExpression, usize)> {
    let block_end = consume_balanced(text, block_start, b'{', b'}')?;
    let mut inspect_end = block_end - 1;
    while inspect_end > block_start + 1 && text.as_bytes()[inspect_end - 1].is_ascii_whitespace() {
        inspect_end -= 1;
    }
    let content_end = if inspect_end > block_start + 1 && text.as_bytes()[inspect_end - 1] == b';' {
        inspect_end - 1
    } else {
        block_end - 1
    };
    let inner = start_trimmed_snippet(source, file_path, outer, block_start + 1, content_end);
    Ok((
        parse_ltl_expression_with_chumsky(source, &inner, file_path)?,
        block_end,
    ))
}

fn find_top_level_block_start(text: &str, start: usize) -> Option<usize> {
    let mut index = start;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    while index < text.len() {
        match text.as_bytes()[index] {
            b'"' => {
                index += 1;
                while index < text.len() && text.as_bytes()[index] != b'"' {
                    index += 1;
                }
            }
            b'(' => paren += 1,
            b')' => paren = paren.saturating_sub(1),
            b'[' => bracket += 1,
            b']' => bracket = bracket.saturating_sub(1),
            b'{' if paren == 0 && bracket == 0 => return Some(index),
            _ => {}
        }
        index += 1;
    }
    None
}

fn scan_ltl_predicate_end(text: &str, start: usize) -> usize {
    let mut index = start;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    while index < text.len() {
        if text[index..].starts_with("//") {
            while index < text.len() && text.as_bytes()[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if text[index..].starts_with("/*") {
            index += 2;
            while index + 1 < text.len() && &text[index..index + 2] != "*/" {
                index += 1;
            }
            index = (index + 2).min(text.len());
            continue;
        }
        match text.as_bytes()[index] {
            b'"' => {
                index += 1;
                while index < text.len() && text.as_bytes()[index] != b'"' {
                    index += 1;
                }
                index = (index + 1).min(text.len());
                continue;
            }
            b'(' => paren += 1,
            b')' if paren == 0 && bracket == 0 && brace == 0 => break,
            b')' => paren = paren.saturating_sub(1),
            b'[' => bracket += 1,
            b']' => bracket = bracket.saturating_sub(1),
            b'{' => brace += 1,
            b'}' if paren == 0 && bracket == 0 && brace == 0 => break,
            b'}' => brace = brace.saturating_sub(1),
            _ if paren == 0 && bracket == 0 && brace == 0 => {
                if text[index..].starts_with("||")
                    || text[index..].starts_with("&&")
                    || starts_with_word(text, index, "until")
                {
                    break;
                }
            }
            _ => {}
        }
        index += 1;
    }
    index
}

fn scan_group_end(text: &str, start: usize) -> usize {
    let mut index = start;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    while index < text.len() {
        match text.as_bytes()[index] {
            b'"' => {
                index += 1;
                while index < text.len() && text.as_bytes()[index] != b'"' {
                    index += 1;
                }
                index = (index + 1).min(text.len());
                continue;
            }
            b'(' => paren += 1,
            b')' if paren == 0 && bracket == 0 && brace == 0 => break,
            b')' => paren = paren.saturating_sub(1),
            b'[' => bracket += 1,
            b']' => bracket = bracket.saturating_sub(1),
            b'{' => brace += 1,
            b'}' if paren == 0 && bracket == 0 && brace == 0 => break,
            b'}' => brace = brace.saturating_sub(1),
            _ => {}
        }
        index += 1;
    }
    index
}

fn consume_balanced(text: &str, start: usize, open: u8, close: u8) -> AlthreadResult<usize> {
    let mut depth = 0usize;
    let mut index = start;
    while index < text.len() {
        match text.as_bytes()[index] {
            b'"' => {
                index += 1;
                while index < text.len() && text.as_bytes()[index] != b'"' {
                    index += 1;
                }
            }
            byte if byte == open => depth += 1,
            byte if byte == close => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(index + 1);
                }
            }
            _ => {}
        }
        index += 1;
    }
    Err(AlthreadError::new(
        ErrorType::SyntaxError,
        None,
        "unterminated balanced expression".to_string(),
    ))
}

fn trimmed_snippet(
    source: &str,
    file_path: &str,
    outer: &SyntaxSnippet,
    mut start: usize,
    mut end: usize,
) -> SyntaxSnippet {
    while start < end && outer.text.as_bytes()[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && outer.text.as_bytes()[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    SyntaxSnippet::new(
        Pos::from_offsets(source, file_path, outer.pos.start + start, outer.pos.start + end),
        outer.text[start..end].to_string(),
    )
}

fn start_trimmed_snippet(
    source: &str,
    file_path: &str,
    outer: &SyntaxSnippet,
    mut start: usize,
    end: usize,
) -> SyntaxSnippet {
    while start < end && outer.text.as_bytes()[start].is_ascii_whitespace() {
        start += 1;
    }
    SyntaxSnippet::new(
        Pos::from_offsets(source, file_path, outer.pos.start + start, outer.pos.start + end),
        outer.text[start..end].to_string(),
    )
}

fn consume_keyword(text: &str, index: &mut usize, keyword: &str) -> bool {
    skip_ws(text, index);
    if starts_with_word(text, *index, keyword) {
        *index += keyword.len();
        true
    } else {
        false
    }
}

fn starts_with_word(text: &str, index: usize, keyword: &str) -> bool {
    let Some(rest) = text.get(index..) else {
        return false;
    };
    if !rest.starts_with(keyword) {
        return false;
    }
    let before_ok = if index == 0 {
        true
    } else {
        !text.as_bytes()[index - 1].is_ascii_alphanumeric() && text.as_bytes()[index - 1] != b'_'
    };
    let end = index + keyword.len();
    let after_ok = match text.as_bytes().get(end).copied() {
        None => true,
        Some(byte) => !byte.is_ascii_alphanumeric() && byte != b'_',
    };
    before_ok && after_ok
}

fn consume_identifier(text: &str, mut index: usize) -> Option<usize> {
    let first = text.as_bytes().get(index).copied()?;
    if !first.is_ascii_alphabetic() && first != b'_' {
        return None;
    }
    index += 1;
    while let Some(byte) = text.as_bytes().get(index).copied() {
        if byte.is_ascii_alphanumeric() || byte == b'_' {
            index += 1;
        } else {
            break;
        }
    }
    Some(index)
}

fn skip_ws(text: &str, index: &mut usize) {
    while *index < text.len() && text.as_bytes()[*index].is_ascii_whitespace() {
        *index += 1;
    }
}

fn ltl_error(source: &str, file_path: &str, start: usize, end: usize, message: &str) -> AlthreadError {
    AlthreadError::new(
        ErrorType::SyntaxError,
        Some(Pos::from_offsets(source, file_path, start, end.max(start + 1))),
        message.to_string(),
    )
}

fn normalize_ltl_predicate_expression(expr: Node<Expression>) -> Node<Expression> {
    match expr.value {
        Expression::Primary(primary) => match primary.value {
            PrimaryExpression::Expression(inner) => *inner,
            _ => Node {
                pos: expr.pos,
                value: Expression::Primary(primary),
            },
        },
        _ => expr,
    }
}

fn expand_identifier_predicate_end(expr: &mut Node<Expression>, end: usize) {
    if let Expression::Primary(primary) = &mut expr.value {
        if let PrimaryExpression::Identifier(identifier) = &mut primary.value {
            identifier.pos.end = end;
            primary.pos.end = end;
            expr.pos.end = end;
        }
    }
}
