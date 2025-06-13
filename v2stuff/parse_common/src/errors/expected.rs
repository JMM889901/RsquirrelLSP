use chumsky::span::SimpleSpan;

use crate::{errors::{DefaultErrorRank, ErrorLevel, ErrorRank, ParseError}, parsed_element::StringToken};


// Expected one of set of tokens, but got something else
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expected<T> {
    pub got: T,
    pub expected: Vec<T>, // What we expected
    pub range: SimpleSpan,
}

//Expected one of set of tokens, but got nothing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedNoMatch<T> {//This will be applied to the parent match rule typically
    pub expected: Vec<T>, // What we expected
    pub range: SimpleSpan,
}
impl<T> ExpectedNoMatch<T> {
    pub fn new(expected: Vec<T>, position: usize) -> Self {
        Self { expected, range: SimpleSpan { start: position, end: position, context: () } }
    }
}



impl<T: StringToken + std::fmt::Debug + Clone + Send + Sync + 'static> ParseError for Expected<T> {
    fn rank(&self) -> usize {
        DefaultErrorRank::expected()
    }

    fn level(&self) -> ErrorLevel {
        ErrorLevel::Error
    }

    fn text(&self) -> String {
        format!(
            "Syntax Error: Expected one of: {}, but got: {}",
            self.expected
                .iter()
                .map(|e| e.as_string())
                .collect::<Vec<_>>()
                .join(", "),
            self.got.as_string()
        )
    }

    fn range(&self) -> SimpleSpan {
        self.range
    }
}

impl<T: StringToken + std::fmt::Debug + Clone + Send + Sync + 'static> ParseError for ExpectedNoMatch<T> {
    fn rank(&self) -> usize {
        DefaultErrorRank::expected()
    }

    fn level(&self) -> ErrorLevel {
        ErrorLevel::Error
    }

    fn text(&self) -> String {
        format!(
            "Syntax Error: Expected one of: {}",
            self.expected
                .iter()
                .map(|e| e.as_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn range(&self) -> SimpleSpan {
        self.range
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedArbitary {
    pub expected: String,
    pub range: SimpleSpan,
}
impl ExpectedArbitary {
    pub fn new(expected: String, position: usize) -> Self {
        Self { expected, range: SimpleSpan { start: position, end: position, context: () } }
    }
}
impl ParseError for ExpectedArbitary {
    fn rank(&self) -> usize {
        DefaultErrorRank::expected()
    }

    fn level(&self) -> ErrorLevel {
        ErrorLevel::Error
    }

    fn text(&self) -> String {
        format!("Syntax Error: Expected {}", self.expected)
    }

    fn range(&self) -> SimpleSpan {
        self.range
    }
}