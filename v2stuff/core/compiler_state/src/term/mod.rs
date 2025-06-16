use std::fmt::Display;

pub mod index_term;
pub mod arc_term;
pub mod string_term;

pub enum CompareResult {
    Accept,
    Reject,
    DontCare,
}


/// Key implementations have a slightly strange relationship with Ord and Eq, Equality is based on the term itself and NOT the truth value
pub trait PreprocessorTerm: Sized + Clone {
    //fn display(&self, map: &TermMap<Self>) -> String;

    fn compare(&self, other: &Self) -> CompareResult;
    fn rejects(&self, other: &Self) -> bool {
        matches!(self.compare(other), CompareResult::Reject)
    }
    fn accepts(&self, other: &Self) -> bool {
        !matches!(self.compare(other), CompareResult::Reject)
    }
    fn not(&self) -> Self;
    
}

//Some shared logic stuff

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum KnownTerm {
    Identifier(String),
    //I think there are more, i forgor
}
impl Display for KnownTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnownTerm::Identifier(id) => write!(f, "{}", id),
            // Add more cases as needed
        }
    }
}


#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum TermTruth {
    True,
    False,
    DontCare,
}
impl Display for TermTruth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TermTruth::True => write!(f, ""),
            TermTruth::False => write!(f, "!"),
            TermTruth::DontCare => write!(f, "?"),
        }
    }
}