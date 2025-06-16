use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::fmt::Display;

use crate::state::CompilerState;
use crate::term::string_term::StringTerm;
use crate::term::{CompareResult, TermTruth};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashMapState<T: Hash + Ord + Clone> {
    terms: HashMap<T, TermTruth>,
}
impl<T: Hash + Ord + Clone + Debug> CompilerState<T> for HashMapState<T> {
    
    type Key<'a> = (&'a T, &'a TermTruth)
        where Self: 'a;
    
    fn terms<'a>(&'a self) -> impl Iterator<Item = Self::Key<'a>> {
        self.terms.iter()
    }

    fn size(&self) -> usize {
        self.terms.len()
    }

    fn rejects_explicit(&self, other: &Self) -> bool {
        for (key, value) in &self.terms {
            if other.terms.contains_key(key) && *value != other.terms[key]{
                return true;
            }
        }
        false
    }

    fn differing_substate(&self, other: &Self) -> Option<Self> {
        let mut new_map = HashMap::new();
        for term in &other.terms {
            match self.cmp_key(&term) {
                CompareResult::Accept => continue,
                CompareResult::DontCare => new_map.insert(term.0.clone(), *term.1),
                CompareResult::Reject => new_map.insert(term.0.clone(), *term.1),
            };
        }
        Some(Self { terms: new_map })
    }

    fn cmp_key(&self, key: &Self::Key<'_>) -> crate::term::CompareResult {
        if let Some(value) = self.terms.get(key.0) {
            if value == key.1 {
                return CompareResult::Accept;
            } else {
                return CompareResult::Reject;
            }
        }
        CompareResult::DontCare
    }

    fn remove_term(&mut self, term: &T) -> bool {
        self.terms.remove(term).is_some()
    }

    fn without_key(&self, key: Self::Key<'_>) -> Self {
        let mut new_map = self.terms.clone();
        new_map.remove(key.0);
        Self { terms: new_map }
    }


}
impl From<Vec<StringTerm>> for HashMapState<String> {
    fn from(terms: Vec<StringTerm>) -> Self {
        let mut map = HashMap::new();
        for term in terms {
            map.insert(term.term, term.truth);
        }
        Self { terms: map }
    }
}
impl<T: Display + Hash + Ord + Debug + Clone> Display for HashMapState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let terms = self.terms.iter().map(|(term, truth)| format!("{}{}", truth, term)).collect::<Vec<_>>().join(" && ");
        write!(f, "{}", terms)
    }
}
#[cfg(test)]
mod tests{
    type HashmapState = HashMapState<String>;
    fn generate_test(states: Vec<Vec<&str>>) -> Vec<HashmapState> {
        states.into_iter().map(|terms| {
            let terms: Vec<StringTerm> = terms.into_iter().map(StringTerm::from).collect();
            HashmapState::from(terms)
        }).collect()
    }

    fn states_contain_str(states: &[HashmapState], terms: Vec<Vec<String>>) -> bool {
        let terms: Vec<Vec<StringTerm>> = terms.into_iter().map(|v| v.into_iter().map(StringTerm::from).collect()).collect();
        let expect_states = terms.into_iter().map(HashmapState::from).collect::<Vec<_>>();
        expect_states.iter().all(|state| states.contains(state))
    }
    fn states_contain(states: &[HashmapState], expected: &[HashmapState]) -> bool {
        expected.iter().all(|state| states.contains(state))
    }
    use crate::term::string_term::StringTerm;

    use super::*;
    #[test]
    fn test_simple_merge(){
        //Two states in, can be merged but not fully
        let states = generate_test(vec![
            vec!["SERVER", "MP", "DEV"],
            vec!["SERVER", "MP", "!DEV"],
        ]);
        let result = HashmapState::resolve_set(states);
        for result in &result {
            println!("{}", result);
        }
        assert!(states_contain_str(&result, vec![vec!["SERVER".to_string(), "MP".to_string()]]));
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
        let result = HashmapState::resolve_set(states);
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
        let result = HashmapState::resolve_set(states);
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
        let result = HashmapState::resolve_set(states);
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