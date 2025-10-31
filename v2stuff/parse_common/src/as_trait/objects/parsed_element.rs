//This is just the old one but without lifetime bullshit, yes its innefficient, i do not care
//Effectively, this mandates ownership of everything, including errors
use chumsky::span::SimpleSpan;

use crate::as_trait::{ParseError, ParseErrorDyn};

pub struct Parsed<T> {
    pub range: SimpleSpan,
    pub item: T,
    pub polluted: usize, // Worst match that has occured (0 is perfect)
    pub errors: Vec<Box<dyn ParseErrorDyn<T>>>,
}