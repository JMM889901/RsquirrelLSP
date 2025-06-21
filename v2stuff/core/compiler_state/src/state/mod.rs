use std::{fmt::{Debug}, vec};

use crate::{term::CompareResult};

pub mod sorted_vec_state;
pub mod hashmap_state;
// I will probably re-define this somewhere more visible, but for the sake of understanding: 
// 
// Variant: A file with an associated state, what is analysed
// 
// File: A file, what is preprocessed
// 
// State: A set of preprocessor terms representing a fixed state, a state is implicitly AND without any OR, 
// however does not have to define every term that can exist, only those that are relevant to the file 
// (Specified in the run-on or actually present within the file)
//
// Term: a single condition, IE SERVER or MP 
// (for simplicity, !TERM is implemented by terms, since it lets me differ the implementation more easily) <- easily my ass lmao


/// Represents the preprocessor terms used when RSquirrel parses a file
///
/// We only care about terms within a given file, we want to have each textually different version of the file,
/// otherwise you wind up with a lot of variants very quickly.
/// 
/// Technically you can actually do everything entirely using comparisons like Quine-McCluskey (My very first attempt did this), 
/// but that would be hard to abstract and would probably be slower in practice.
/// 
/// This moves some (but not all) of the work up-front, but of course each variant we produce
/// has to be run through all of the analysis steps. (though this can be parallelized very easily)
///
/// Multistate or one run is an alternative approach, multistate being that we split varaints at parse-time and only reparse relevant sections
/// while in one-run we handle everything as one parse, this is not possible with a PEG
/// since it requires ambiguity as each input token can have any number of output tokens 
pub trait CompilerState<Repr: Sized + Clone + Sized> : Debug + Sized + Clone + PartialEq + Eq{
    type Key<'a>: 'a
        where Self: 'a;
    fn terms(&self) -> impl Iterator<Item = Self::Key<'_>>;
    fn size(&self) -> usize;

    /// Does SELF explicitly reject other, technically its possible for other not to reject self (or err, is it actually? probably not)
    fn rejects_explicit(&self, other: &Self) -> bool;
    //fn differing_keys(&self, other: &Self) -> Option<Vec<&T>>;

    /// This is specifically keys that the difference between other and self, not vice-versa, 
    /// if self has a key not in other it will not show here
    fn differing_substate(&self, other: &Self) -> Option<Self>;
    fn cmp_key(&self, key: &Self::Key<'_>) -> CompareResult;

    fn remove_term(&mut self, term: &Repr) -> bool;
    fn without_key(&self, key: Self::Key<'_>) -> Self;

    /// Compare a state with a set of states, returning the set of missing partial states to make at least one of the set always true.
    /// 
    /// Any one of these states could be added to self to make the comparison true.
    fn try_resolve(&self, other: &[Self]) -> Result<(), Vec<Self>> {
        let mut differ = vec![];
        for state in other {
            if let Some(different) = self.differing_substate(state) {
                differ.push(different);
            } else {
                return Ok(());
            }
        }
        Err(Self::resolve_set(differ))
    }

    /// This probably won't be used so I can track the actual result for error messages
    fn always_accepts_one(&self, other: Vec<Self>) -> bool{
        let new = self.try_resolve(&other);
        new.is_ok()
    }

    /// Modified Quine-McCluskey algorithm, critically we use cancelling (See resolve_one) to avoid needing a full truth table
    /// 
    /// Truth be told, I'm not actually sure thats logically "correct", but hey i'm sure someone will tell me if its not
    fn resolve_set(states: Vec<Self>) -> Vec<Self> {
        let mut states = states;
        loop {
            //Trying to avoid duplicate states is quite expensive here, i should find a nicer way to do this
            let mut merged = false;
            let mut did_cancel = false;
            let mut merged_states = vec![];
            let mut merge_results = vec![];
            let mut cancelled_states = vec![];
            let mut cancel_results = vec![];
            for (i, state) in states.iter().enumerate() {
                for other in states.iter().skip(i + 1) {
                    match state.resolve_one(other) {
                        MergeResults::Merged(newstate) => {
                            merge_results.push(newstate);
                            merged_states.push(state);
                            merged_states.push(other);
                            merged = true;
                        }
                        MergeResults::Negated => {
                            merged_states.push(state);
                            merged_states.push(other);
                            merged = true;
                            // A + !A = true, we can ignore this
                        }
                        MergeResults::Consumed { remaining, consumed} => {
                            merged_states.push(consumed);
                            merge_results.push(remaining.clone());
                            merged = true;
                        }
                        MergeResults::Cancelled { new, remaining, cancelled } => {
                            cancelled_states.push(cancelled);
                            cancel_results.push(new);//Cancelling in the same cycle as merging is just like
                            //A massive pain in the ass, so lets not do that
                            //Most of the time that we *can* do that, we can also just merge instead
                            cancel_results.push(remaining);
                            did_cancel = true;
                        }//Honestly maybe this should be a seperate loop? not sure if thats more or less expensive
                        MergeResults::None => continue,
                    }
                }
                
            }
            //Dedupping makes me sad :(
            if merged {
                let mut newstates = vec![];
                //println!("Merging, new states: {:?}", merge_results);
                for new_state in merge_results {
                    if !newstates.contains(&new_state) && !merged_states.contains(&&new_state) {
                        newstates.push(new_state);
                    }
                }
                for leftover in states.iter() {
                    if !newstates.contains(leftover) && !merged_states.contains(&leftover) {
                        newstates.push(leftover.clone());
                    }
                }
                if states == newstates {
                    panic!("fuk");
                }
                states = newstates;
            } else if did_cancel {
                let mut newstates = vec![];                
                //println!("Cancelling, new states: {:?}", cancel_results);
                for new_state in cancel_results {
                    if !newstates.contains(&new_state) && !cancelled_states.contains(&&new_state) {
                        newstates.push(new_state);
                    }
                }
                for leftover in states.iter() {
                    if !newstates.contains(leftover) && !cancelled_states.contains(&leftover) {
                        newstates.push(leftover.clone());
                    }
                }
                if states == newstates {
                    panic!("fuk");
                }
                states = newstates;
            } else {
                break;
            }

        }
        states
    }

    /// Attempts to partially merge or cancel two states
    /// 
    /// This is mostly boolean math, we can merge a given state in 3 ways: 
    /// 
    /// Consuming: AB + A = A 
    /// 
    /// Merging: AB + A!B = A, alternatively A + !A = Negated
    /// 
    /// Cancelling: ABC + A!B 
    ///     = A(BC + !B) = A((B+!B)(C + !B)) = A(C+!B) = AC + A!B
    /// 
    /// Cancelling is only needed because we only create states within a files context, meaning comparisons between 2 files have no guarantee that 
    /// All terms are explicitly defined in both files, or the resulting states
    fn resolve_one<'a>(&'a self, other: &'a Self) -> MergeResults<'a, Self> {
        let mut matching = 0;
        let mut different = None;
        //We can only merge if we differ by exactly one key
        for key in self.terms() {
            match other.cmp_key(&key) {
                CompareResult::Accept => matching += 1,
                CompareResult::Reject => {
                    if different.is_none() {
                        different = Some(key);
                    } else {
                        return MergeResults::None; // More than one differing key, cannot merge
                    }
                }
                CompareResult::DontCare => continue, // Ignore keys that don't appear in both
            }
        }
        if different.is_none() {
            // Although we cannot merge, AB + A can still be consumed
            match self.enclosing_state(other) {
                Some((consumed, remaining)) => return MergeResults::Consumed { consumed, remaining  }, // AB + A = A
                None => return MergeResults::None, // A + !A = true
            }
        }
        let different = different.unwrap();
        let self_len = self.size();
        let other_len = other.size();
        if self_len == other_len {
            if self_len == 1 {
                return MergeResults::Negated; // A + !A = true
            }
            if matching == self_len - 1 {
                // AB + A!B = A
                return MergeResults::Merged(self.without_key(different));
            }
        }
        /*
        Comparing ABC + A!B, matching is 1 (A), different is B
        We already confimed they have differing lengths
        We already confimed they differ by exactly one key
        ABC len = 3 - differ = 2
        A!B len = 2 - differ = 1
        A!B Survives, ABC becomes AC  
         */
        if matching == other_len - 1 {
            // ABC + A!B = AC + A!B
            return MergeResults::Cancelled { new: self.without_key(different), remaining: other.clone(), cancelled: self };
        }
        if matching == self_len - 1 {
            return MergeResults::Cancelled { new: other.without_key(different), remaining: self.clone(), cancelled: other };
        }

        MergeResults::None
    }

    /// Returns the state that fully encloses the other state, IE AB + A would return A
    fn enclosing_state<'a>(&'a self, other: &'a Self) -> Option<(&'a Self, &'a Self)> {
        let (larger, smaller) = match self.size().cmp(&other.size()) {
            std::cmp::Ordering::Greater => (self, other),
            _ => (other, self),
        };

        // A state must be less specific (IE, fewer terms) and the larger must explicitly accept all terms of the smaller state
        // Basically, the larger state must be a substate, we need to be able to say that if Smaller is true, Larger MUST also be true 
        for term in smaller.terms() {
            match larger.cmp_key(&term) {
                CompareResult::Accept => continue,
                _ => return None,
            }
        }
        Some((smaller, larger))
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum MergeResults<'a, T> {
    Merged(T),//AB + A!B = A
    Negated, //A + !A = true
    Cancelled{
        new: T,
        remaining: T,
        cancelled: &'a T,
    }, //ABC + A!B = AC + A!B
    Consumed{
        consumed: &'a T,
        remaining: &'a T,
    },//AB + A = A, true is left, false is right
    None,//This arguably should return the original inputs, but i think technically more memory efficient to just return none
}