use std::{fmt::{format, Display}, hash::Hash, sync::Arc};

use crate::{term::{index_term::TermMap, KnownTerm, TermTruth}, PreprocessorTerm};



#[derive(Debug, Clone)]
pub struct ArcTerm {
    pub term: Arc<KnownTerm>,
    pub truth: TermTruth,
}
impl PreprocessorTerm for ArcTerm {
    fn compare(&self, other: &Self) -> super::CompareResult {
        if Arc::ptr_eq(&self.term, &other.term) {
            return match (&self.truth, &other.truth) {
                (TermTruth::True, TermTruth::True) => super::CompareResult::Accept,
                (TermTruth::False, TermTruth::False) => super::CompareResult::Accept,
                (TermTruth::True, TermTruth::False) => super::CompareResult::Reject,
                (TermTruth::False, TermTruth::True) => super::CompareResult::Reject,
                _ => super::CompareResult::DontCare,
            }
        }
        super::CompareResult::DontCare
    }
    fn not(&self) -> Self {
        match self {
            ArcTerm { term, truth: TermTruth::True } => ArcTerm { term: term.clone(), truth: TermTruth::False },
            ArcTerm { term, truth: TermTruth::False } => ArcTerm { term: term.clone(), truth: TermTruth::True },
            ArcTerm { term, truth: TermTruth::DontCare } => ArcTerm { term: term.clone(), truth: TermTruth::DontCare },
        }
    }
    fn is_vm(&self) -> bool {
        self.term.is_vm()
    }
    
    fn is_accept(&self) -> bool {
        self.truth == TermTruth::True
    }
    
    fn is_reject(&self) -> bool {
        self.truth == TermTruth::False
    }
}
impl Display for ArcTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.truth, self.term)
    }
}
impl ArcTerm {
    pub fn new(term: String, truth: TermTruth) -> Self {
        ArcTerm { term: Arc::new(KnownTerm::Identifier(term)), truth }
    }
    pub fn from_true(term: Arc<KnownTerm>) -> Self {
        ArcTerm { term, truth: TermTruth::True }
    }
    pub fn from_false(term: Arc<KnownTerm>) -> Self {
        ArcTerm { term, truth: TermTruth::False }
    }
    pub fn is_vm(&self) -> bool {
        self.term.is_vm()
    }
}


impl Ord for ArcTerm {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Arc::as_ptr(&self.term).cmp(&Arc::as_ptr(&other.term))
    }
}
impl PartialOrd for ArcTerm {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for ArcTerm{}
impl PartialEq for ArcTerm {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.term, &other.term)
    }
}
//This hash is a bit scary i think, since technically x == y can be true even when hash(x) != hash(y)
impl Hash for ArcTerm {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.term.hash(state);
        self.truth.hash(state);
    }
}



pub fn generate_map(terms: Vec<Vec<String>>) -> Vec<Arc<KnownTerm>> {
    let mut dedup = Vec::new();
    for term in terms.iter().flatten() {
        let term = if let Some(rest) = term.strip_prefix('!') {
            KnownTerm::Identifier(rest.to_string())
        } else {
            KnownTerm::Identifier(term.to_string())
        };
        if !dedup.contains(&Arc::new(term.clone())) {
            dedup.push(Arc::new(term));
        }
    }
    dedup
}

pub fn to_arc_term(term: &str, map: &[Arc<KnownTerm>]) -> ArcTerm {
    if let Some(rest) = term.strip_prefix('!') {
        ArcTerm { term: map.iter().find(|t| matches!(t.as_ref(), KnownTerm::Identifier(x) if x == rest)).unwrap().clone(), truth: TermTruth::False }
    } else if let Some(rest) = term.strip_prefix('?') {
        ArcTerm { term: map.iter().find(|t| matches!(t.as_ref(), KnownTerm::Identifier(x) if x == rest)).unwrap().clone(), truth: TermTruth::DontCare }
    } else {
        ArcTerm { term: map.iter().find(|t| matches!(t.as_ref(), KnownTerm::Identifier(x) if x == term)).unwrap().clone(), truth: TermTruth::True }
    }
}


#[cfg(test)]
mod tests{
    use std::collections::HashMap;

    use crate::{state::{sorted_vec_state::SortedVecState, CompilerState}};


    fn generate_map(terms: Vec<Vec<&str>>) -> Vec<Arc<KnownTerm>> {
        let mut dedup = Vec::new();
        for term in terms.iter().flatten() {
            let term = if let Some(rest) = term.strip_prefix('!') {
                KnownTerm::Identifier(rest.to_string())
            } else {
                KnownTerm::Identifier(term.to_string())
            };
            if !dedup.contains(&Arc::new(term.clone())) {
                dedup.push(Arc::new(term));
            }
        }
        dedup
    }
    type ArcTermState = SortedVecState<ArcTerm>;


    fn contains_by_value(state: &ArcTermState, term: &ArcTerm) -> bool {
        state.terms.iter().any(|t| t.term == term.term && t.truth == term.truth)
    }
    fn state_equals_by_value(state: &ArcTermState, terms: &ArcTermState) -> bool {
        state.terms.len() == terms.terms.len() && state.terms.iter().all(|t| contains_by_value(terms, t))
    }

    fn generate_test(states: Vec<Vec<&str>>) -> Vec<ArcTermState> {
        let map = generate_map(states.clone());

        states.into_iter().map(|terms| {
            let terms: Vec<ArcTerm> = terms.into_iter().map(|t| to_arc_term(t, &map)).collect();
            ArcTermState::new(terms)
        }).collect()
    }

    fn states_contain_str(states: &[ArcTermState], terms: Vec<Vec<&str>>) -> bool {
        //let terms: Vec<Vec<ArcTerm>> = terms.into_iter().map(|v| v.into_iter().map(ArcTerm::from).collect()).collect();
        let expected_states = generate_test(terms);
        states_contain(&states, &expected_states)
    }
    fn states_contain(states: &[ArcTermState], expected: &[ArcTermState]) -> bool {
        expected.iter().all(|state| states.iter().any(|s| state_equals_by_value(s, state)))
    }

    use super::*;
    #[test]
    fn test_simple_merge(){
        //Two states in, can be merged but not fully
        let states = generate_test(vec![
            vec!["SERVER", "MP", "DEV"],
            vec!["SERVER", "MP", "!DEV"],
        ]);
        let result = ArcTermState::resolve_set(states);
        for result in &result {
            println!("{}", result);
        }
        assert!(states_contain_str(&result, vec![vec!["SERVER", "MP"]]));
    }


    #[test]
    fn test_complex_merge(){
        //Four states in, can be reduced to three, but still not fully merged
        let states = generate_test(vec![
            vec!["MP", "DEV"],
            vec!["!MP", "DEV"],
            vec!["!MP", "!DEV"],
            vec!["MP", "!DEV", "SERVER"],

        ]);
        let result = ArcTermState::resolve_set(states);
        for result in &result {
            println!("{}", result);
        }
        let expect = generate_test(vec![
            vec!["!MP"], // If MP is false, we pass if dev is true or false, so always
            vec!["DEV"], // if dev is true, we pass if MP is true or false, so always
            vec!["SERVER"], // If server is true we pass when MP && !DEV, 
            //but from prior, we know that if this is NOT the case, one of the previous 2 will be true
        ]);
        assert!(states_contain(&result, &expect));
    }
    
    #[test]
    fn test_fully_merge() {
        //Fully merges all states, this will always be true
        let states = generate_test(vec![
            vec!["MP", "DEV", "SERVER"], //111
            vec!["!MP", "DEV", "SERVER"], //011
            vec!["MP", "!DEV", "SERVER"], //101
            vec!["!MP", "!DEV", "SERVER"], //001
            vec!["MP", "DEV", "!SERVER"], //110
            vec!["!MP", "DEV", "!SERVER"], //010
            vec!["MP", "!DEV", "!SERVER"], //100
            vec!["!MP", "!DEV", "!SERVER"], //000
        ]);
        let result = ArcTermState::resolve_set(states);
        for result in &result {
            println!("{}", result);
        }
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_cancel() {
        let states = generate_test(vec![
            vec!["!SERVER"],
            vec!["MP", "DEV"],
            vec!["MP", "!DEV", "SERVER"],
        ]);
        /*
            MP && !DEV && SERVER or !SERVER = MP && !DEV or !SERVER 
            Subsequently, MP && DEV or MP && !DEV or !SERVER = MP or !SERVER
         */

        let result = ArcTermState::resolve_set(states);
        for result in &result {
            println!("{}", result);
        }
        let expect = generate_test(vec![
            vec!["MP"],
            vec!["!SERVER"],
        ]);
        assert!(states_contain(&result, &expect));
    }
}