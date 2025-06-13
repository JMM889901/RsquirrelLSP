use chumsky::span::SimpleSpan;
use parse_common::errors::{ErrorRank, ParseError};
use sqparse::TokenItem;

pub struct PreprocessorErrorRank;
impl ErrorRank for PreprocessorErrorRank {}
impl PreprocessorErrorRank {
    pub fn unmatched_directive() -> usize {
        4
    }
    pub fn unexpected_token() -> usize {
        5
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreprocessorError<'src> {
    UnmatchedDirective(TokenItem<'src>),
    UnexpectedToken(TokenItem<'src>),
}

impl ParseError for PreprocessorError<'_> {
    fn rank(&self) -> usize {
        match self {
            PreprocessorError::UnmatchedDirective(_) => {
                PreprocessorErrorRank::unmatched_directive()
            }
            PreprocessorError::UnexpectedToken(_) => PreprocessorErrorRank::unexpected_token(),
        }
    }

    fn level(&self) -> parse_common::errors::ErrorLevel {
        match self {
            PreprocessorError::UnmatchedDirective(_) => parse_common::errors::ErrorLevel::Error,
            PreprocessorError::UnexpectedToken(_) => parse_common::errors::ErrorLevel::Error,
        }
    }

    fn text(&self) -> String {
        match self {
            PreprocessorError::UnmatchedDirective(token) => {
                format!("Unmatched directive: {}", token.token.ty)
            }
            PreprocessorError::UnexpectedToken(token) => {
                format!("Unexpected token: {}", token.token.ty)
            }
        }
    }

    fn range(&self) -> chumsky::prelude::SimpleSpan {
        match self {
            PreprocessorError::UnmatchedDirective(token)
            | PreprocessorError::UnexpectedToken(token) => {
                SimpleSpan::from(token.token.range.clone())
            }
        }
    }
}
