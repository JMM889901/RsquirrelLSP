use std::{fmt::Display};

use sqparse::TokenItem;

#[derive(Debug, Clone, PartialEq)]
pub struct ASTElement<T, E>{
    pub item: ASTState<T, E>,
    pub range: std::ops::Range<usize>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum ASTState<T, E>{
    Matched(T),
    Substitute(T, T, Vec<T>),//Gave, Got, Expected, used for explicit substitution error handling (IE, automatically replacing a single & with && in a preprocessor expression)
    Expected(T, Vec<T>),//Got, Expected, but we know roughly what was expected
    Recoverable(T),//Got
    Unrecoverable(T),//Got
    WithError(Box<ASTState<T, E>>, E),//Bad, dont use
    Error(E), //Error message
}
impl<T: Display, E: Display> Display for ASTState<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ASTState::Matched(item) => write!(f, "Matched: {}", item),
            ASTState::Substitute(gave, got, expected) => write!(f, "Got: {}, Substituted: {}, Expected: {}", got, gave, expected.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", ")),
            //If god did not want me to create horrific oneliners, he would not have made map functions
            ASTState::Expected(item, expected) => write!(f, "Got: {}, Expected: {}", item, expected.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", ")),
            ASTState::Recoverable(item) => write!(f, "Recoverable, got: {}", item),
            ASTState::Unrecoverable(item) => write!(f, "Unrecoverable, got: {}", item),//The name is bad, generally "non-recoverable" shouldn't happen, Thats the entire reason
            //I'm not just using Sqparses inbuilt parser, specifically this is the equivalent of a not-matcheds
            ASTState::WithError(state, error) => write!(f, "With Error: {}, Error: {}", state, error),
            ASTState::Error(error) => write!(f, "Error: {}", error),
        }
    }
}
impl<T, E> ASTElement<T, E> {
    pub fn can_continue(&self) -> bool {
        matches!(&self.item, 
            ASTState::Matched(_) | 
            ASTState::Recoverable(_) |
            ASTState::Substitute(_, _, _))
    }
}


pub fn lex_string(source: &str) -> Vec<TokenItem>{
    sqparse::tokenize(source, sqparse::Flavor::SquirrelRespawn).unwrap()
}
