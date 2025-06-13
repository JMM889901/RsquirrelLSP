use chumsky::{error::Error, span::SimpleSpan};
use sqparse::TokenItem;
use std::fmt::{Debug, Display};
use crate::{errors::{DefaultErrorRank, ErrorLevel, ErrorRank, ParseError}, parsed_element::StringToken};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnexpectedToken<T> {
    pub token: T,
    pub range: SimpleSpan,
}
impl<T: StringToken + Debug + Clone + Send + Sync + 'static> ParseError for UnexpectedToken<T>
{
    fn rank(&self) -> usize {
        DefaultErrorRank::unknown_token()
    }

    fn level(&self) -> ErrorLevel {
        ErrorLevel::Error
    }

    fn text(&self) -> String {
        format!("Syntax Error: Unexpected token: {}", self.token.as_string())
    }

    fn range(&self) -> SimpleSpan {
        self.range
    }
}
impl<T: Debug + Send + Sync + StringToken> Display for UnexpectedToken<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Syntax Error: Unexpected token: {}", self.token.as_string())
    }
}
