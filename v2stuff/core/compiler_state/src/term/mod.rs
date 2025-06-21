use std::fmt::Display;

pub mod index_term;
pub mod arc_term;
pub mod string_term;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
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
    fn is_vm(&self) -> bool;

    fn is_accept(&self) -> bool;
    fn is_reject(&self) -> bool;
    fn not(&self) -> Self;
    
}

//Some shared logic stuff

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum KnownTerm {
    Identifier(String),
    SomeImpossibleSecondOption,
    //I think there are more, i forgor
}
impl Display for KnownTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnownTerm::Identifier(id) => write!(f, "{}", id),
            _ => { 
                write!(f, "This is a debug thingy to make clippy shut up")
            }
            // Add more cases as needed
        }
    }
}
impl KnownTerm {
    pub fn is_vm(&self) -> bool {
        match self {
            KnownTerm::Identifier(id) => {
                id == "SERVER" || id == "CLIENT" || id == "UI"
            },
            _ => false,
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