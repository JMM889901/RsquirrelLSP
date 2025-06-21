use std::{fmt::{Debug, Display}, hash::Hash};

use crate::{state::CompilerState, term::{self, arc_term::ArcTerm, CompareResult}, PreprocessorTerm};

//Sorted vec implementation
#[derive(Debug, Clone)]
pub struct SortedVecState<T: PreprocessorTerm + std::hash::Hash>{
    pub terms: Vec<T>,//Sorted List
}
impl<T: PreprocessorTerm + std::hash::Hash + Ord + Debug> CompilerState<T> for SortedVecState<T> {
    type Key<'a> = &'a T where T: 'a;

    
    fn terms(&self) -> impl Iterator<Item = Self::Key<'_>> {
        self.terms.iter()
    }

    fn size(&self) -> usize {
        self.terms.len()
    }

    fn rejects_explicit(&self, other: &Self) -> bool {
        for term in &self.terms {
            match other.cmp_key(&term) {
                CompareResult::Accept => continue,
                CompareResult::DontCare => continue,
                CompareResult::Reject => return true,
            }
        }
        false
    }

    // Prior to genericing everything, this was differing_terms, and always returned a Option<Vec<&T>>
    fn differing_substate(&self, other: &Self) -> Option<Self> {
        let mut new_terms = Vec::new();
        for term in &other.terms {
            match self.cmp_key(&term) {
                CompareResult::Accept => continue,
                CompareResult::DontCare => new_terms.push(term.clone()),
                CompareResult::Reject => new_terms.push(term.clone()),
            }
        }
        if new_terms.is_empty() {
            None
        } else {
            Some(Self{ terms: new_terms })//We assume that other is already sorted so this is as well
        }
    }

    fn cmp_key(&self, key: &&T) -> CompareResult {
        match self.terms.binary_search(key) {
            Ok(index) => {
                self.terms[index].compare(key)
            }
            Err(_) => CompareResult::DontCare
        }
    }

    fn remove_term(&mut self, term: &T) -> bool {
        match self.terms.binary_search(term) {
            Ok(index) => {
                self.terms.remove(index);
                true
            }
            Err(_) => false,
        }
    }

    fn without_key(&self, key: &T) -> Self {
        let mut new_terms = Vec::new();
        for term in &self.terms {
            if term != key {
                new_terms.push(term.clone());
            }
        }
        Self { terms: new_terms }//This is still sorted so
    }

}
impl SortedVecState<ArcTerm> {
    pub fn cmp_string(&self, other: &str) -> CompareResult {
        // Compare the two states by their string representations
        for term in &self.terms {
            match term.term.to_string().as_str() {
                s if s == other => {
                    return match term.truth {
                        crate::term::TermTruth::True => CompareResult::Accept,
                        crate::term::TermTruth::False => CompareResult::Reject,
                        crate::term::TermTruth::DontCare => CompareResult::DontCare,
                    };
                },
                _ => continue,
            }
        }
        CompareResult::DontCare
    }
}
impl<T: PreprocessorTerm + Hash + Display> Display for SortedVecState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (vms, others): (Vec<_>, Vec<_>) = self.terms.iter().partition(|term| {
            term.is_vm()
        });
        let terms = others.iter().map(|term| term.to_string()).collect::<Vec<_>>().join(" && ");
        if vms.is_empty() {
            return write!(f, "{}", terms);
        }
        let active_vm = vms.iter()
            .find(|term| term.is_accept())
            .map(|term| term.to_string())
            .unwrap_or_else(|| "No active VM".to_string());
        let inactive_vms = vms.iter()
            .filter(|term| term.is_reject())
            .map(|term| term.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        if terms.is_empty() {
            return write!(f, "{} ({})", active_vm, inactive_vms);
        }

        write!(f, "{} ({}) && {}", active_vm, inactive_vms, terms)
    }
}
impl<T: PreprocessorTerm + std::hash::Hash + Ord> SortedVecState<T> {
    pub fn get(&self, key: &T) -> Option<&T> {
        self.terms.binary_search(key).ok().map(|index| &self.terms[index])
    }
    pub fn new(mut terms: Vec<T>) -> Self {
        terms.sort();
        Self { terms }
    }
}
impl<T: PreprocessorTerm + std::hash::Hash + Ord> PartialEq for SortedVecState<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.terms.len() != other.terms.len() {
            return false;
        }
        for (a, b) in self.terms.iter().zip(&other.terms) {
            match a.compare(b) {
                CompareResult::Accept => continue,
                _ => return false,
            }
        }
        true
    }
}
impl<T: PreprocessorTerm + std::hash::Hash + Ord> Eq for SortedVecState<T> {}
impl<T: PreprocessorTerm + std::hash::Hash + Ord> Hash for SortedVecState<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for term in &self.terms {
            term.hash(state);
        }
    }
}


#[cfg(test)]
mod tests{
    type StringState = SortedVecState<StringTerm>;
    fn generate_test(states: Vec<Vec<&str>>) -> Vec<SortedVecState<StringTerm>> {
        states.into_iter().map(|terms| {
            let terms: Vec<StringTerm> = terms.into_iter().map(StringTerm::from).collect();
            SortedVecState::new(terms)
        }).collect()
    }
    fn state_equals(state: &StringState, terms: Vec<String>) -> bool {
        let terms: Vec<StringTerm> = terms.into_iter().map(StringTerm::from).collect();
        state.terms == terms
    }
    fn states_contain_str(states: &[StringState], terms: Vec<Vec<String>>) -> bool {
        let terms: Vec<Vec<StringTerm>> = terms.into_iter().map(|v| v.into_iter().map(StringTerm::from).collect()).collect();
        let expect_states = terms.into_iter().map(StringState::new).collect::<Vec<_>>();
        expect_states.iter().all(|state| states.contains(state))
    }
    fn states_contain(states: &[StringState], expected: &[StringState]) -> bool {
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
        let result = StringState::resolve_set(states);
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
        let result = StringState::resolve_set(states);
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
        let result = StringState::resolve_set(states);
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
        let result = StringState::resolve_set(states);
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