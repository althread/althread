use pest::{
    error::{ErrorVariant, InputLocation, LineColLocation},
    iterators::Pairs,
    Parser,
};
use pest_derive::Parser;

use crate::error::{AlthreadError, ErrorType, Pos};

#[derive(Parser)]
#[grammar = "althread.pest"]
struct AlthreadParser;

pub fn parse(source: &str) -> Result<Pairs<Rule>, AlthreadError> {
    AlthreadParser::parse(Rule::program, source).map_err(|e| {
        let mut pos = match e.line_col {
            LineColLocation::Pos(pos) | LineColLocation::Span(pos, _) => Pos {
                line: pos.0,
                col: pos.1,
                start: 0,
                end: 0,
            },
        };
        match e.location {
            InputLocation::Pos(p) => {
                pos.start = p;
                pos.end = p + 1;
            }
            InputLocation::Span((start, end)) => {
                pos.start = start;
                pos.end = end;
            }
        };

        let error_message = match e.variant {
            ErrorVariant::ParsingError { positives, .. } => {
                format!("Expected one of {:?}", positives)
                // TODO: better error message
                // For example, if the dev uses return in the wrong place
                // we should tell them, that return can't be used in there.
                // the output is like this at the moment:
                // Error at 56:3
                //    |
                // 56 |   return 0;
                //    |   ^---
                //    |
                // Syntax Error: Expected one of [statement]
            }
            ErrorVariant::CustomError { message } => message,
        };
        AlthreadError::new(ErrorType::SyntaxError, Some(pos), error_message)
    })
}
