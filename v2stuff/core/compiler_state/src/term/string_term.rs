use std::fmt::Display;
use std::hash::Hash;
use crate::{term::TermTruth, PreprocessorTerm};

#[derive(Debug, Clone)]
pub struct StringTerm {
    pub term: String,
    pub truth: TermTruth,
}
impl From<String> for StringTerm {
    fn from(value: String) -> Self {
        if let Some(rest) = value.strip_prefix('!') {
            StringTerm { term: rest.to_string(), truth: TermTruth::False }
        } else if let Some(rest) = value.strip_prefix('?') {
            StringTerm { term: rest.to_string(), truth: TermTruth::DontCare }
        } else {
            StringTerm { term: value, truth: TermTruth::True }
        }
    }
}
impl From<&str> for StringTerm {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}
impl PreprocessorTerm for StringTerm {
    fn compare(&self, other: &Self) -> super::CompareResult {
        if self.term == other.term {
            return match (&self.truth, &other.truth) {
                (TermTruth::True, TermTruth::True) | 
                (TermTruth::False, TermTruth::False) => super::CompareResult::Accept,
                (TermTruth::DontCare, _) | (_, TermTruth::DontCare) => super::CompareResult::DontCare,
                _ => super::CompareResult::Reject,
            }
        }
        super::CompareResult::DontCare
    }
    fn not(&self) -> Self {
        match self {
            StringTerm { term, truth: TermTruth::True } => StringTerm { term: term.clone(), truth: TermTruth::False },
            StringTerm { term, truth: TermTruth::False } => StringTerm { term: term.clone(), truth: TermTruth::True },
            StringTerm { term, truth: TermTruth::DontCare } => StringTerm { term: term.clone(), truth: TermTruth::DontCare },
        }
    }
    fn is_vm(&self) -> bool {
        self.term == "SERVER" || self.term == "CLIENT" || self.term == "UI"
    }
    
    fn is_accept(&self) -> bool {
        self.truth == TermTruth::True
    }
    
    fn is_reject(&self) -> bool {
        self.truth == TermTruth::False
    }

}

impl Display for StringTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.truth, self.term)
    }
}

impl Ord for StringTerm {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.term.cmp(&other.term)
    }
}
impl PartialOrd for StringTerm {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for StringTerm{}
impl PartialEq for StringTerm {
    fn eq(&self, other: &Self) -> bool {
        self.term == other.term
    }
}
//This hash is a bit scary i think, since technically x == y can be true even when hash(x) != hash(y)
impl Hash for StringTerm {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.term.hash(state);
        self.truth.hash(state);
    }
}
