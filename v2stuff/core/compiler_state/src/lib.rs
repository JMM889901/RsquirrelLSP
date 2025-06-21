
use crate::{state::sorted_vec_state::SortedVecState, term::arc_term::ArcTerm};
pub use crate::term::PreprocessorTerm;

pub mod state;
pub mod term;

pub type CompilerTerm = ArcTerm;
pub type CompilerState = SortedVecState<ArcTerm>;//This is so close to being as fast as index term but is way more convenient


#[cfg(test)]
mod bench {
    use crate::{state::{hashmap_state::HashMapState, CompilerState}, term::{arc_term::{generate_map, to_arc_term, ArcTerm}, index_term::{self, to_idx_term, IndexTerm}, string_term::StringTerm}};

    use super::*;
    fn all_combs_recur(terms: &[&str]) -> Vec<Vec<String>> {
        if terms.is_empty() {
            return vec![vec![]];
        }
        let mut result = Vec::new();
        let first = terms[0];
        let rest = &terms[1..];
        for comb in all_combs_recur(rest) {
            result.push(comb.clone());
            let mut new_comb = comb.clone();
            new_comb.push(first.to_string());
            result.push(new_comb);
            let mut new_comb = comb.clone();
            new_comb.push(format!("!{}", first));
            result.push(new_comb);
        }
        result
    }
    
    #[test]
    fn bench_sorted_string() {
        //This is quick and dirty because im tired and frankly, its not that huge of a deal if its not the *fastest* option
        let terms = vec!["SERVER", "MP", "DEV", "TEST", "FUNNY"];
        let all_combs = all_combs_recur(&terms);
        let mut all_combs = all_combs.into_iter().map(|mut x| { x.sort(); x }).collect::<Vec<_>>();
        all_combs.sort();
        all_combs.dedup();
        let all_combs = all_combs.into_iter().filter(|x| x.len() > 1).collect::<Vec<_>>();
        println!("Generated {} combinations", all_combs.len());
        let sorted: Vec<SortedVecState<StringTerm>> = all_combs.into_iter().map(|terms| {
            let terms: Vec<StringTerm> = terms.into_iter().map(StringTerm::from).collect();
            SortedVecState::new(terms)
        }).collect();
        println!("Running SortedVecState with string benchmark with {} states", sorted.len());
        let start = std::time::Instant::now();
        let sorted = SortedVecState::resolve_set(sorted);
        let duration = start.elapsed();
        println!("SortedVecState took: {:?}", duration);
        for result in &sorted {
            println!("result: {:?}", result);
        }
    }

    #[test]
    fn bench_hashmap_string() {
        let terms = vec!["SERVER", "MP", "DEV", "TEST", "FUNNY"];
        let all_combs = all_combs_recur(&terms);
        let mut all_combs = all_combs.into_iter().map(|mut x| { x.sort(); x }).collect::<Vec<_>>();
        all_combs.sort();
        all_combs.dedup();
        let all_combs = all_combs.into_iter().filter(|x| x.len() > 1).collect::<Vec<_>>();
        println!("Generated {} combinations", all_combs.len());
        let hashmap: Vec<HashMapState<String>> = all_combs.into_iter().map(|terms| {
            let terms: Vec<StringTerm> = terms.into_iter().map(StringTerm::from).collect();
            HashMapState::from(terms)
        }).collect();
        println!("Running HashMapState with string benchmark with {} states", hashmap.len());
        let start = std::time::Instant::now();
        let hashmap = HashMapState::resolve_set(hashmap);
        let duration = start.elapsed();
        println!("HashMapState took: {:?}", duration);
        for result in &hashmap {
            println!("result: {:?}", result);
        }
    }

    #[test]
    fn bench_sorted_arc() {
        let terms = vec!["SERVER", "MP", "DEV", "TEST", "FUNNY"];
        let all_combs = all_combs_recur(&terms);
        let mut all_combs = all_combs.into_iter().map(|mut x| { x.sort(); x }).collect::<Vec<_>>();
        all_combs.sort();
        all_combs.dedup();
        let all_combs = all_combs.into_iter().filter(|x| x.len() > 1).collect::<Vec<_>>();
        println!("Generated {} combinations", all_combs.len());
        let map = generate_map(all_combs.clone());
        let sorted: Vec<SortedVecState<ArcTerm>> = all_combs.into_iter().map(|terms| {
            let terms: Vec<ArcTerm> = terms.into_iter().map(|term| to_arc_term(&term, &map)).collect();
            SortedVecState::new(terms)
        }).collect();

        println!("Running SortedVecState with arc benchmark with {} states", sorted.len());
        let start = std::time::Instant::now();
        let sorted = SortedVecState::resolve_set(sorted);
        let duration = start.elapsed();
        println!("SortedVecState took: {:?}", duration);
        for result in &sorted {
            println!("result: {:?}", result);
        }
    }

    #[test]
    fn bench_sorted_usize() {
        let terms = vec!["SERVER", "MP", "DEV", "TEST", "FUNNY"];
        let all_combs = all_combs_recur(&terms);
        let mut all_combs = all_combs.into_iter().map(|mut x| { x.sort(); x }).collect::<Vec<_>>();
        all_combs.sort();
        all_combs.dedup();
        let all_combs = all_combs.into_iter().filter(|x| x.len() > 1).collect::<Vec<_>>();
        println!("Generated {} combinations", all_combs.len());
        let map = index_term::generate_map(all_combs.clone());
        let sorted: Vec<SortedVecState<IndexTerm>> = all_combs.into_iter().map(|terms| {
            let terms: Vec<IndexTerm> = terms.into_iter().map(|term| to_idx_term(&term, &map)).collect();
            SortedVecState::new(terms)
        }).collect();
        println!("Running SortedVecState with usize benchmark with {} states", sorted.len());
        let start = std::time::Instant::now();
        let sorted = SortedVecState::resolve_set(sorted);
        let duration = start.elapsed();
        println!("SortedVecState took: {:?}", duration);
        for result in &sorted {
            println!("result: {:?}", result);
        }

    }
}


