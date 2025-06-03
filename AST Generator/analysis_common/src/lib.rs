use std::{cmp::min_by, collections::{btree_set::Difference, HashMap}, hash::Hash};

use common::FileInfo;
use ASTParser::ast::{Element, AST};
use ConfigPredictor::state::SqCompilerState;

pub mod spanning_search;
pub mod variable;
pub mod modjson;

pub trait HasState{
    fn get_state(&self) -> CompiledState;
}

#[derive(Debug, Clone)]
pub struct RunPrimitiveInfo{
    //Essentially the run after parsing, before any further steps
    //Should effectively be able to be used anywhere to say where, what and in what context
    pub file: FileInfo,
    pub context: CompiledState,
    pub id: usize,
    pub ast: Vec<Element<AST>>,//TODO: See about removing this
    //It feels wrong to have it here
}
impl RunPrimitiveInfo{
    pub fn new(file: FileInfo, context: CompiledState, id: usize, ast: Vec<Element<AST>>) -> Self{
        RunPrimitiveInfo{
            file,
            context,
            id,
            ast
        }
    }
}
impl HasState for RunPrimitiveInfo{
    fn get_state(&self) -> CompiledState {
        return self.context.clone();
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledState(HashMap<String, bool>);
impl From<HashMap<String, bool>> for CompiledState{
    fn from(value: HashMap<String, bool>) -> Self {
        return CompiledState(value);
    }
}
impl CompiledState{
    pub fn string_out_simple(&self) -> String{
        let mut result = Vec::new();
        for (key, value) in &self.0 {
            result.push(format!("{}{}", value.then_some("").unwrap_or("!"), key));
        }
        return result.join("&");
    }

    pub fn do_i_reject_explicit(&self, other: &CompiledState) -> bool{
        //This is a bit of a hack, but it works
        //We are going to assume that the context is a set of states
        //If any state in self is not in other, then self rejects other
        for (key, value) in &self.0 {
            //If they dont have the key then we dont really know
            //Generally though, going down the dependency tree means we should reject this,
            //Wheras going up means we should accept
            if other.0.contains_key(key) && *value != other.0[key]{
                return true;
            }
        }
        return false;
    }

    pub fn get_problematic_keys(&self, sets: &Vec<CompiledState>) -> Vec<CompiledState>{
        //We assume that there is no one option that passes
        //First, determine common keys with self
        //We largely assume that do_i_reject_explicit is false
        let newsets = sets.into_iter().filter_map(|set|{
            self.differing_keys(set)
        });
        let cancelled = CompiledState::cancel_set(&newsets.collect());
        return cancelled;
    }

    pub fn will_i_always_accept_one_of(&self, sets: &Vec<CompiledState>) -> bool{
        //We assume that there is no one option that passes

        //First, determine common keys with self
        //We larely assume that do_i_reject_explicit is false
        let newsets = sets.into_iter().filter_map(|set|{
            self.differing_keys(set)
        });
        let cancelled = CompiledState::cancel_set(&newsets.collect());
        return cancelled.is_empty();
        //Now determine if all keys have a negation
    }
    pub fn cancel_set(sets: &Vec<CompiledState>) -> Vec<CompiledState>{
        //This is effectively a none-recursive Quine McCluskey algorithm
        //It didnt used to be, but it also used to not work
        let mut merged = true;
        let mut newsets = sets.clone();
        while merged {//Basically just keep running until theres nothing more to do
            //println!("\n\nNew sets: {:?}", newsets);
            //Sets that we have proven to negated or otherwise unimportant
            let mut newersets = Vec::new();
            let mut merged_sets = Vec::new();
            let mut merge_results: Vec<CompiledState> = Vec::new();
            let mut cancelled_sets = Vec::new();
            let mut cancel_results = Vec::new();
            merged = false;
            for (idx, set) in newsets.iter().enumerate(){
                
                for otherset in newsets[idx+1..].iter(){
                    let result = set.attempt_merge_onecond(otherset);
                    match result{
                        MergeResults::Merged(newset) => {
                            //println!("Merged: {:?} and {:?} into {:?}", set, otherset, newset);
                            merge_results.push(newset.clone());
                            merged_sets.push(set.clone());
                            merged_sets.push(otherset.clone());
                            continue;
                        }
                        MergeResults::Consumed { consumed, remaining } => {
                            //This is a bit of a hack, but it works
                            //println!("Consumed: {:?} and {:?} into {:?}", set, otherset, remaining);
                            merge_results.push(remaining.clone());
                            merged_sets.push(consumed);
                            continue;
                        }
                        MergeResults::CancelledElem(self_new, other_new) => {
                            //This is overcomplicated and not a part of the original algorithm, my goal here was to be more
                            //Greedy when negating elements, it seems to work but I dont have as much confidence due to it not 
                            //being a part of the original algorithm
                            //println!("Cancelled: {:?} and {:?} into {:?} and {:?}", set, otherset, self_new, other_new);
                            cancel_results.push(self_new.clone());
                            cancel_results.push(other_new.clone());
                            if &self_new != set && &self_new != otherset{
                                cancelled_sets.push(set.clone());
                            }
                            if &other_new != otherset && &other_new != set{
                                cancelled_sets.push(otherset.clone());
                            }
                            continue;
                        }
                        MergeResults::Cancelled => {
                            //println!("Cancelled: {:?} and {:?}", set, otherset);
                            //This is basically just like, fully merging
                            merged_sets.push(set.clone());
                            merged_sets.push(otherset.clone());
                            continue;
                        }
                        MergeResults::None => {
                        }
                    }
                }
            }
            
            //We dont cancel if we have merged
            if merged_sets.len() > 0{
                for result in merge_results{
                    if !newersets.contains(&result) && !merged_sets.contains(&result){
                        newersets.push(result);
                    }
                }
                for set in &newsets{
                    if !newersets.contains(&set) && !merged_sets.contains(&set){
                        newersets.push(set.clone());
                    }
                }
                merged = true;
            } else if cancelled_sets.len() > 0{
                //println!("Cancelled to sets: {:?}", cancel_results);
                for result in cancel_results{
                    if !newersets.contains(&result) && !cancelled_sets.contains(&result){
                        newersets.push(result);
                    }
                }
                for set in &newsets{
                    if !newersets.contains(&set) && !cancelled_sets.contains(&set){
                        newersets.push(set.clone());
                    }
                }
                //println!("Removed sets: {:?}", cancelled_sets);
                //println!("new sets: {:?}", newersets);
                merged = true;
            }
            if !merged{
                break;
            }
            if newsets == newersets{
                //We have not changed anything, so we can stop
                panic!("No changes made, this should not happen");
            }
            newsets = newersets;
        }
        
        return newsets;         
    }
    /*
        the condition MP && DEV and !MP should result in DEV being the problematic element
        The condition !MP && DEV and MP && !DEV should result in both being problematic
        take for example the condition SERVER && MP && DEV and !DEV
        on an overall level, this should effectively resolve down to SERVER && MP and !DEV
        However: RANDOM && CONDITION && DEV and SOME_OTHER && !DEV should not resolve down
     */
    fn attempt_merge_onecond(&self, other: &CompiledState) -> MergeResults{
        //the only condition this returns Ok is if both elements differ on exactly one key
        //At the simplest level the goal is to remove a condition from one or both of these
        //If we can prove that the value of that condition specifically does not effect compilation
        //Either because it is always negated, or its presence only indicates the requirement for other conditions

        //Ensure there is exactly one key that is explicitly different
        let mut matching_keys = 0;
        let mut different = None;
        for (key, value) in &self.0 {
            if other.0.contains_key(key){
                if *value != other.0[key]{
                    if different.is_some(){
                        return MergeResults::None;
                    }
                    different = Some(key.clone());
                } else {
                    matching_keys += 1;
                }
            }
        };
        if different.is_none(){
            //Contains no explicitly different keys
            let merged = min_by(self, other, |a, b| a.0.len().cmp(&b.0.len()));
            let mut all_match = true;
            for (key, value) in &merged.0 {
                if self.0.contains_key(key) && other.0.contains_key(key){
                    if *value != self.0[key] || *value != other.0[key]{
                        all_match = false;
                    }
                } else {
                    all_match = false;
                }
            }
            //Confirm that this is lesser specificity
            if all_match{
                let consumed;
                if self.0.len() > other.0.len(){
                    consumed = self.clone();
                } else {
                    consumed = other.clone();
                }
                
                return MergeResults::Consumed { consumed, remaining: merged.clone() }
            }
            return MergeResults::None;
            //return MergeResults::None;//These conditions are the same or do not interact
        };
        let self_single = self.0.len() - matching_keys == 1;
        let other_single = other.0.len() - matching_keys == 1;

        //If both are single, then we can fully cancel
        if self.0.len() == 1 && other.0.len() == 1{
            return MergeResults::Cancelled;
        }
        let mut self_new = self.0.clone();
        let mut other_new = other.0.clone();
        if self_single{
            other_new.remove(different.as_ref().unwrap());
        }
        if other_single{
            self_new.remove(different.as_ref().unwrap());
        }
        //If they explicitly match except for this element, we can merge
        if self_single && other_single{
            let self_new = CompiledState(self_new);
            return MergeResults::Merged(self_new);
        }
        //if if one of these is a single-point condition (IE, the the differing condition is also the only not-present condition)
        //Then we can perform an implicit merge such that the lower specificity condition remains, 
        if self_single || other_single
        {
            let self_new = CompiledState(self_new);
            let other_new = CompiledState(other_new);
            return MergeResults::CancelledElem(self_new, other_new);
        }
        return MergeResults::None;
    }

    pub fn differing_keys(&self, other: &CompiledState) -> Option<CompiledState>{
        let self_inner = &self.0;
        let other_inner = &other.0;
        let mut new_map = HashMap::new();
        //Explicitly this is keys that are in or different in other
        for (key, value) in other_inner {
            if !self_inner.contains_key(key) || *value != self_inner[key]{
                new_map.insert(key.clone(), *value);
            }
        }
        if new_map.is_empty(){
            return None;
        }
        return Some(CompiledState(new_map));        
    }

    pub fn get(&self, key: &String) -> Option<bool>{//Id like more abstraction really, every place i should use this
        //Should be internal
        if let Some(value) = self.0.get(key){
            return Some(*value);
        }
        return None;
    }
}
impl From<SqCompilerState> for CompiledState{
    fn from(value: SqCompilerState) -> Self {
        return CompiledState(value.0);
    }//These have the same internal type but they have different purposes
    //CompilerState is from preprocessor to construct an estimated state or set of states, CompiledState is for comparisons and conclusions on that state
    //To be honest i just dont like importing things from preprocessor in analyser
}
impl FromIterator<(String, bool)> for CompiledState{
    fn from_iter<T: IntoIterator<Item = (String, bool)>>(iter: T) -> Self {
        let mut map = HashMap::new();
        for (key, value) in iter{
            map.insert(key, value);
        }
        return CompiledState(map);
    }
}

#[derive(Debug, Clone, PartialEq)]
enum MergeResults{
    Merged(CompiledState),//The 2 conditions are now one
    CancelledElem(CompiledState, CompiledState), //One element was cancelled out
    Consumed{
        consumed: CompiledState,
        remaining: CompiledState,
    },
    Cancelled,//Both conditions merged perfectly, IE !A && A
    None,//This arguably should return the original inputs, but its technically more memory efficient to just return none
}

#[cfg(test)]
mod cancel_tests{
use super::*;
#[test]
fn test_cancel_simple_failure(){
    let SERVER_and_MP_and_DEV = CompiledState(HashMap::from([("SERVER".to_string(), true), ("MP".to_string(), true), ("DEV".to_string(), true)]));
    let SERVER_and_MP_and_notDEV = CompiledState(HashMap::from([("SERVER".to_string(), true), ("MP".to_string(), true), ("DEV".to_string(), false)]));
    let result = CompiledState::cancel_set(&vec![SERVER_and_MP_and_DEV, SERVER_and_MP_and_notDEV]);
    //Should cancel into SERVER and MP
    println!("{:?}", result);
    assert!(result == vec![CompiledState(HashMap::from([("SERVER".to_string(), true), ("MP".to_string(), true)]))]);
}
#[test]
fn test_cancel_complex_failure(){
    let MP_and_DEV = CompiledState(HashMap::from([("MP".to_string(), true), ("DEV".to_string(), true)]));
    let MP_and_notDEV_and_RAND = CompiledState(HashMap::from([("MP".to_string(), true), ("DEV".to_string(), false), ("RAND".to_string(), true)]));
    let notMP_and_DEV = CompiledState(HashMap::from([("MP".to_string(), false), ("DEV".to_string(), true)]));
    let notMP_and_notDEV = CompiledState(HashMap::from([("MP".to_string(), false), ("DEV".to_string(), false)]));
    let result = CompiledState::cancel_set(&vec![MP_and_DEV, MP_and_notDEV_and_RAND, notMP_and_DEV, notMP_and_notDEV]);
    //Should identify that !MP, DEV and RAND are the insufficiently defined conditions with any one of them guaranteeing a pass of at least some condition
    println!("{:?}", result);
    assert!(result.contains(&CompiledState(HashMap::from([("MP".to_string(), false)]))));
    assert!(result.contains(&CompiledState(HashMap::from([("DEV".to_string(), true)]))));
    assert!(result.contains(&CompiledState(HashMap::from([("RAND".to_string(), true)]))));
    assert!(result.len() == 3);
    
}
#[test]
fn test_very_complex_pass(){
    let MP_and_DEV_and_TEST = CompiledState(HashMap::from([("MP".to_string(), true), ("DEV".to_string(), true), ("TEST".to_string(), true)]));//111
    let MP_and_DEV_and_notTEST = CompiledState(HashMap::from([("MP".to_string(), true), ("DEV".to_string(), true), ("TEST".to_string(), false)]));//110
    let MP_and_notDEV_and_TEST = CompiledState(HashMap::from([("MP".to_string(), true), ("DEV".to_string(), false), ("TEST".to_string(), true)]));//101
    let MP_and_notDEV_and_notTEST = CompiledState(HashMap::from([("MP".to_string(), true), ("DEV".to_string(), false), ("TEST".to_string(), false)]));//100
    let notMP_and_DEV_and_TEST = CompiledState(HashMap::from([("MP".to_string(), false), ("DEV".to_string(), true), ("TEST".to_string(), true)]));//011
    let notMP_and_DEV_and_notTEST = CompiledState(HashMap::from([("MP".to_string(), false), ("DEV".to_string(), true), ("TEST".to_string(), false)]));//010
    let notMP_and_notDEV_and_TEST = CompiledState(HashMap::from([("MP".to_string(), false), ("DEV".to_string(), false), ("TEST".to_string(), true)]));//001
    let notMP_and_notDEV_and_notTEST = CompiledState(HashMap::from([("MP".to_string(), false), ("DEV".to_string(), false), ("TEST".to_string(), false)]));//000

    let result = CompiledState::cancel_set(&vec![MP_and_DEV_and_TEST, MP_and_DEV_and_notTEST, MP_and_notDEV_and_TEST, MP_and_notDEV_and_notTEST, notMP_and_DEV_and_TEST, notMP_and_DEV_and_notTEST, notMP_and_notDEV_and_TEST, notMP_and_notDEV_and_notTEST]);
    //Should fully cancel
    println!("{:?}", result);
}
#[test]
fn test_cancel_overlap_pass(){
    let NOT_MP = CompiledState(HashMap::from([("MP".to_string(), false)]));
    let MP_AND_DEV = CompiledState(HashMap::from([("MP".to_string(), true), ("DEV".to_string(), true)]));
    let MP_AND_NOT_DEV = CompiledState(HashMap::from([("MP".to_string(), true), ("DEV".to_string(), false)]));
    let result = CompiledState::cancel_set(&vec![MP_AND_NOT_DEV, NOT_MP, MP_AND_DEV]);
    println!("{:?}", result);
}
#[test]
fn test_cancel_one_failure(){
    let NOT_MP_NOT_DEV = CompiledState(HashMap::from([("MP".to_string(), false), ("DEV".to_string(), false), ("TEST".to_string(), true)]));
    let NOT_MP_NOT_DEV_NOT_TEST = CompiledState(HashMap::from([("MP".to_string(), false), ("DEV".to_string(), false), ("TEST".to_string(), false)])); 
    let MP_AND_NOT_DEV = CompiledState(HashMap::from([("MP".to_string(), true), ("DEV".to_string(), false), ("TEST".to_string(), true)]));
    let SOME_OTHER = CompiledState(HashMap::from([("MP".to_string(), true), ("DEV".to_string(), false), ("TEST".to_string(), false)]));
    let result = CompiledState::cancel_set(&vec![MP_AND_NOT_DEV, NOT_MP_NOT_DEV, SOME_OTHER, NOT_MP_NOT_DEV_NOT_TEST]);
    println!("{:?}", result);
}
//2 merge conditions to 3
//One low specificity cancels 3 into 2
//remaining 2 conditions merge into 1 singular
#[test]
fn test_multistep_cancel_merge(){
    //Order of cancellation and merging is important
    let MP_AND_DEV_AND_LOBBY_AND_PC = CompiledState(HashMap::from([("MP".to_string(), true), ("DEV".to_string(), true), ("LOBBY".to_string(), true), ("PC".to_string(), true)]));
    let MP_AND_DEV_AND_LOBBY_AND_NOT_PC = CompiledState(HashMap::from([("MP".to_string(), true), ("DEV".to_string(), true), ("LOBBY".to_string(), true), ("PC".to_string(), false)]));
    //These should merge first
    let MP_AND_NOT_DEV = CompiledState(HashMap::from([("MP".to_string(), true), ("DEV".to_string(), false)]));
    let NOT_LOBBY = CompiledState(HashMap::from([("LOBBY".to_string(), false)]));
    let result = CompiledState::cancel_set(&vec![MP_AND_DEV_AND_LOBBY_AND_PC, MP_AND_DEV_AND_LOBBY_AND_NOT_PC, MP_AND_NOT_DEV, NOT_LOBBY]);
    println!("{:?}", result);

}
}
mod merge_tests{
use super::*;
#[test]
fn test_merge_double_fail(){
    let mut map1 = HashMap::new();
    map1.insert("A".to_string(), true);
    map1.insert("B".to_string(), false);
    let mut map2 = HashMap::new();
    map2.insert("A".to_string(), false);
    map2.insert("B".to_string(), true);
    let set1 = CompiledState(map1);
    let set2 = CompiledState(map2);
    let result = set1.attempt_merge_onecond(&set2);
    //Should fail
    println!("{:?}", result);
    assert!(result == MergeResults::None);
}
#[test]
fn test_merge_merge_success(){
    let A_and_B_and_C = CompiledState(HashMap::from([("A".to_string(), true), ("B".to_string(), true), ("C".to_string(), true)]));
    let A_and_B_and_notC = CompiledState(HashMap::from([("A".to_string(), true), ("B".to_string(), true), ("C".to_string(), false)]));
    let result = A_and_B_and_C.attempt_merge_onecond(&A_and_B_and_notC);
    //should merge int A and B
    println!("{:?}", result);
    assert!(result == MergeResults::Merged(CompiledState(HashMap::from([("A".to_string(), true), ("B".to_string(), true)]))));
}
#[test]
fn test_merge_cancel_partial_success(){
    let A_and_B_and_C = CompiledState(HashMap::from([("A".to_string(), true), ("B".to_string(), true), ("C".to_string(), true)]));
    let not_C = CompiledState(HashMap::from([("C".to_string(), false)]));
    let result = A_and_B_and_C.attempt_merge_onecond(&not_C);
    //Should cancel into A and B or !C
    //A B and C is effectively shrunk because we know that if C is false it will pass
    // !C remains in play, as it is lower specificity
    //Essentially, we want to prove that if either of the two returned statements are true, then at least one of the provided must also be true
    println!("{:?}", result);
    assert!(result == MergeResults::CancelledElem(CompiledState(HashMap::from([("A".to_string(), true), ("B".to_string(), true)])), not_C.clone()));
}
#[test]
fn test_merge_cancel_full_success(){
    let not_C = CompiledState(HashMap::from([("C".to_string(), false)]));
    let C = CompiledState(HashMap::from([("C".to_string(), true)]));
    let reuslt = C.attempt_merge_onecond(&not_C);
    //Should fully cancel
    println!("{:?}", reuslt);
    assert!(reuslt == MergeResults::Cancelled);
}

#[test]
fn test_merge_cancel_partial_twoway_success_2(){
    let A_and_B_and_C = CompiledState(HashMap::from([("A".to_string(), true), ("B".to_string(), true), ("C".to_string(), true)]));
    let A_and_Not_C = CompiledState(HashMap::from([("A".to_string(), true), ("C".to_string(), false)]));
    let result = A_and_B_and_C.attempt_merge_onecond(&A_and_Not_C);
    //Should cancel into A and B or A and !C
    println!("{:?}", result);
    assert!(result == MergeResults::CancelledElem(CompiledState(HashMap::from([("A".to_string(), true), ("B".to_string(), true)])), A_and_Not_C.clone()));
    if let MergeResults::CancelledElem(a, b) = result{
        let repeat = a.attempt_merge_onecond(&b);
        println!("{:?}", repeat);
    }

}
}