use std::fmt::{Debug, Display};

use chumsky::span::SimpleSpan;

use crate::{errors::{DefaultErrorRank, ErrorLevel, ErrorRank, ParseError}, parsed_element::StringToken};


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Substituted<T: Debug + Send + Sync> {
    pub gave: T,//This is probably bad
    pub got: T,
    pub expected: Vec<T>,
    pub range: SimpleSpan,
}
impl<T: Display + Debug + Send + Sync> Display for Substituted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Syntax Error: Substituted {} for {}, expected one of: {}", self.gave, self.got, self.expected.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", "))
    }
}
impl<T: StringToken + Debug + Send + Sync + std::clone::Clone> ParseError for Substituted<T> {
    fn rank(&self) -> usize {
        DefaultErrorRank::substitute()
    }

    fn level(&self) -> ErrorLevel {
        ErrorLevel::Error
    }

    fn text(&self) -> String {
        format!(
            "Syntax Error: Substituted {} for {}, expected one of: {}",
            self.gave.as_string(), self.got.as_string(), self.expected.iter().map(|e| e.as_string()).collect::<Vec<_>>().join(", ")
        )
    }

    fn range(&self) -> SimpleSpan {
        self.range
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defaulted<T: Debug + Send + Sync> {
    pub gave: T,
    pub expected: Vec<T>,
    pub range: SimpleSpan,
}
impl<T: Display + Debug + Send + Sync> Display for Defaulted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Syntax Error: Defaulted {} for expected one of: {}", self.gave, self.expected.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", "))
    }
}
impl<T: StringToken + Debug + Send + Sync + std::clone::Clone> ParseError for Defaulted<T> {
    fn rank(&self) -> usize {
        DefaultErrorRank::substitute()
    }

    fn level(&self) -> ErrorLevel {
        ErrorLevel::Warning
    }

    fn text(&self) -> String {
        format!(
            "Syntax Error: expected one of: {}, defaulted to {}",
            self.gave.as_string(), self.expected.iter().map(|e| e.as_string()).collect::<Vec<_>>().join(", ")
        )
    }

    fn range(&self) -> SimpleSpan {
        self.range
    }
}