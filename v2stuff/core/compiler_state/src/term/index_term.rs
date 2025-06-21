use std::hash::Hash;

use crate::{state::sorted_vec_state::SortedVecState, term::{KnownTerm, TermTruth}, PreprocessorTerm};

/// This is a baked representation of compiler state, representing conditions as a usize
/// The logic for identifying variants of a file is done in the preprocessor crate
/// This is used after the preprocessor has run, because Quine McCluskey is expensive and string comparison is costly 
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TermMap {
    pub map: Vec<KnownTerm>,//its this or a map of usize -> term, but like, thats just this with extra steps
}
impl TermMap {
    pub fn get_index(&self, term: &str) -> Option<usize> {
        self.map.iter().position(|t| matches!(t, KnownTerm::Identifier(x) if x == term))
    }
}


#[derive(Debug, Clone, Copy)]
pub struct IndexTerm {
    pub term: usize,// Paired with a map of usize -> term, probably the cheapest implementation
    pub truth: TermTruth,
    pub vm: bool, 
}
impl PreprocessorTerm for IndexTerm {
    fn compare(&self, other: &Self) -> super::CompareResult {
        if self.term == other.term {
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
            IndexTerm { term, truth: TermTruth::True, vm } => IndexTerm { term: *term, truth: TermTruth::False, vm: *vm },
            IndexTerm { term, truth: TermTruth::False, vm } => IndexTerm { term: *term, truth: TermTruth::True, vm: *vm },
            IndexTerm { term, truth: TermTruth::DontCare, vm } => IndexTerm { term: *term, truth: TermTruth::DontCare, vm: *vm },
        }
    }
    fn is_vm(&self) -> bool {
        self.vm// Bit sketch
    }
    
    fn is_accept(&self) -> bool {
        self.truth == TermTruth::True
    }
    
    fn is_reject(&self) -> bool {
        self.truth == TermTruth::False
    }
    

}
impl IndexTerm {
    pub fn display(&self, map: &TermMap) -> String {
        let term = &map.map[self.term];
        format!("{}{}", self.truth, term)
    }
}

impl Ord for IndexTerm {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.term.cmp(&other.term)
    }
}
impl PartialOrd for IndexTerm {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for IndexTerm{}
impl PartialEq for IndexTerm {
    fn eq(&self, other: &Self) -> bool {
        self.term == other.term
    }
}
//This hash is a bit scary i think, since technically x == y can be true even when hash(x) != hash(y)
impl Hash for IndexTerm {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.term.hash(state);
        self.truth.hash(state);
    }
}

impl SortedVecState<IndexTerm> {
    pub fn display(&self, map: &TermMap) -> String {
        self.terms.iter().map(|term| term.display(map)).collect::<Vec<_>>().join(" && ")
    }
}



pub fn generate_map(terms: Vec<Vec<String>>) -> TermMap {
    let mut dedup = Vec::new();
    for term in terms.iter().flatten() {
        let term = if let Some(rest) = term.strip_prefix('!') {
            KnownTerm::Identifier(rest.to_string())
        } else {
            KnownTerm::Identifier(term.to_string())
        };
        if !dedup.contains(&term) {
            dedup.push(term);
        }
    }
    TermMap { map: dedup }
}
pub fn is_vm(term: &str) -> bool {
    term == "SERVER" || term == "CLIENT" || term == "UI"
}

pub fn to_idx_term(term: &str, map: &TermMap) -> IndexTerm {
    if let Some(rest) = term.strip_prefix('!') {
        IndexTerm { term: map.get_index(rest).unwrap(), truth: TermTruth::False, vm: is_vm(rest) }
    } else if let Some(rest) = term.strip_prefix('?') {
        IndexTerm { term: map.get_index(rest).unwrap(), truth: TermTruth::DontCare, vm: is_vm(rest) }
    } else {
        IndexTerm { term: map.get_index(term).unwrap(), truth: TermTruth::True, vm: is_vm(term) }
    }
}


#[cfg(test)]
mod tests{
    

    use crate::{state::{sorted_vec_state::SortedVecState, CompilerState}};

    type IndexTermState = SortedVecState<IndexTerm>;


    fn contains_by_value(state: &IndexTermState, term: &IndexTerm) -> bool {
        state.terms.iter().any(|t| t.term == term.term && t.truth == term.truth)
    }
    fn state_equals_by_value(state: &IndexTermState, terms: &IndexTermState) -> bool {
        state.terms.len() == terms.terms.len() && state.terms.iter().all(|t| contains_by_value(terms, t))
    }

    fn generate_test(states: Vec<Vec<&str>>) -> (Vec<IndexTermState>, TermMap) {
        let states: Vec<Vec<String>> = states.into_iter().map(|v| v.into_iter().map(|s| s.to_string()).collect()).collect();
        let map = generate_map(states.clone());

        let states = states.into_iter().map(|terms| {
            let terms: Vec<IndexTerm> = terms.into_iter().map(|t| to_idx_term(&t, &map)).collect();
            IndexTermState::new(terms)
        }).collect();
        (states, map)
    }
    fn generate_states(states: Vec<Vec<&str>>, map: &TermMap) -> Vec<IndexTermState> {
        states.into_iter().map(|terms| {
            let terms: Vec<IndexTerm> = terms.into_iter().map(|t| to_idx_term(t, map)).collect();
            IndexTermState::new(terms)
        }).collect()
    }

    fn states_contain_str(states: &[IndexTermState], terms: Vec<Vec<&str>>) -> bool {
        //let terms: Vec<Vec<IndexTerm>> = terms.into_iter().map(|v| v.into_iter().map(IndexTerm::from).collect()).collect();
        let expected_states = generate_test(terms).0;
        states_contain(states, &expected_states)
    }
    fn states_contain(states: &[IndexTermState], expected: &[IndexTermState]) -> bool {
        expected.iter().all(|state| states.iter().any(|s| state_equals_by_value(s, state)))
    }

    use super::*;
    #[test]
    fn test_simple_merge(){
        //Two states in, can be merged but not fully
        let (states, map) = generate_test(vec![
            vec!["SERVER", "MP", "DEV"],
            vec!["SERVER", "MP", "!DEV"],
        ]);
        let result = IndexTermState::resolve_set(states);
        for result in &result {
            println!("{}", result.display(&map));
        }
        assert!(states_contain_str(&result, vec![vec!["SERVER", "MP"]]));
    }


    #[test]
    fn test_complex_merge(){
        //Four states in, can be reduced to three, but still not fully merged
        let (states, map) = generate_test(vec![
            vec!["MP", "DEV"],
            vec!["!MP", "DEV"],
            vec!["!MP", "!DEV"],
            vec!["MP", "!DEV", "SERVER"],

        ]);
        let result = IndexTermState::resolve_set(states);
        for result in &result {
            println!("{}", result.display(&map));
        }
        let expect = generate_states(vec![
            vec!["!MP"], // If MP is false, we pass if dev is true or false, so always
            vec!["DEV"], // if dev is true, we pass if MP is true or false, so always
            vec!["SERVER"], // If server is true we pass when MP && !DEV, 
            //but from prior, we know that if this is NOT the case, one of the previous 2 will be true
        ], &map);
        assert!(states_contain(&result, &expect));
    }
    
    #[test]
    fn test_fully_merge() {
        //Fully merges all states, this will always be true
        let (states, map) = generate_test(vec![
            vec!["MP", "DEV", "SERVER"], //111
            vec!["!MP", "DEV", "SERVER"], //011
            vec!["MP", "!DEV", "SERVER"], //101
            vec!["!MP", "!DEV", "SERVER"], //001
            vec!["MP", "DEV", "!SERVER"], //110
            vec!["!MP", "DEV", "!SERVER"], //010
            vec!["MP", "!DEV", "!SERVER"], //100
            vec!["!MP", "!DEV", "!SERVER"], //000
        ]);
        let result = IndexTermState::resolve_set(states);
        for result in &result {
            println!("{}", result.display(&map));
        }
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_cancel() {
        let (states, map) = generate_test(vec![
            vec!["!SERVER"],
            vec!["MP", "DEV"],
            vec!["MP", "!DEV", "SERVER"],
        ]);
        /*
            MP && !DEV && SERVER or !SERVER = MP && !DEV or !SERVER 
            Subsequently, MP && DEV or MP && !DEV or !SERVER = MP or !SERVER
         */
        for state in &states {
            println!("{:?}", state);
        }
        let result = IndexTermState::resolve_set(states);
        
        for result in &result {
            println!("{}", result.display(&map));
            println!("result: {:?}", result);      
        }
        let expect = generate_states(vec![
            vec!["MP"],
            vec!["!SERVER"],
        ], &map);
        assert!(states_contain(&result, &expect));
    }
}