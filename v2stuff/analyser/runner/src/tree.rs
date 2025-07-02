use compiler_state::{state::CompilerState as CompilerStateTrait, CompilerState};
use downcast_rs::{impl_downcast, DowncastSync};
use mod_loader::fileinfo::{FileInfo, LoadedFile};
use preprocessor::VariantID;
use std::fmt::Debug;
///An abstraction over any given "result" type, steps should never interact directly with the actual value because, from their perspective, 
/// there is no situation in which other files have exactly one guaranteed known outcome, because preprocessor shenanigans
/// So basically, queries should interact with and return these, and steps should use provided for_each and for_none methods to handle the results
/// Alternatively they could use map and store one of these in their results
/// TODO: CompilerState is huge, and so its variantID as a result, 48 bytes is a lot of cloning here
/// usize is just the index, to make mapping functions slightly faster
pub enum VariantData<T> {
    Multi(Vec<(VariantID, usize, T)>),
    Possible(Vec<(VariantID, usize, T)>, Vec<CompilerState>),//Second vec is the mismatching states, IE what is preventing this from being a guaranteed match
    Single(VariantID, usize, T),
    NonePreserving(LoadedFile, CompilerState),//Where practical its probably worth storing this
    None,
}


pub trait DowncastableData : DowncastSync{}
impl_downcast!(sync DowncastableData);
///^So i can cache these 
impl<T: Send + Sync + 'static> DowncastableData for VariantData<T> {}


impl<T: Clone> Clone for VariantData<T> {
    fn clone(&self) -> Self {
        match self {
            VariantData::Multi(vec) => VariantData::Multi(vec.clone()),
            VariantData::Possible(vec, bad) => VariantData::Possible(vec.clone(), bad.clone()),
            VariantData::Single(v, idx, data) => VariantData::Single(v.clone(), *idx, data.clone()),
            VariantData::NonePreserving(id, state) => VariantData::NonePreserving(id.clone(), state.clone()),
            VariantData::None => VariantData::None,
        }
    }
}

impl<T> VariantData<T> {


    pub fn is_some(&self) -> bool {
        match self {
            VariantData::Multi(_) => true,
            VariantData::Possible(_, _) => true,
            VariantData::Single(_, _, _) => true,
            VariantData::NonePreserving(_, _) => false,
            VariantData::None => false,
        }
    }

    pub fn map<'a, U, F: FnMut(&VariantID, usize, &'a T) -> U>(&'a self, f: &mut F) -> VariantData<U> {
        match self {
            VariantData::Multi(vec) => VariantData::Multi(vec.iter().map(|(v, idx, data)| (v.clone(), *idx, f(v, *idx, data))).collect()),
            VariantData::Possible(vec, bad) => VariantData::Possible(vec.iter().map(|(v, idx, data)| (v.clone(), *idx, f(v, *idx, data))).collect(), bad.clone()),
            VariantData::Single(v, idx, data) => VariantData::Single(v.clone(), *idx, f(v, *idx, data)),
            VariantData::NonePreserving(id, state) => VariantData::NonePreserving(id.clone(), state.clone()),
            VariantData::None => VariantData::None,
        }
    }

    ///SUPER IMPORTANT: This can and will change completion state, its up to you (by which i mean future me) to identify() afterwards 
    pub fn filter_map<U, F: FnMut(&VariantID, usize, &T) -> Option<U>>(&self, f: &mut F) -> VariantData<U> {
        match self {
            VariantData::Multi(vec) => VariantData::Multi(vec.iter().filter_map(|(v, idx, data)| f(v, *idx, data).map(|d| (v.clone(), *idx, d))).collect()),
            VariantData::Possible(vec, bad) => VariantData::Possible(vec.iter().filter_map(|(v, idx, data)| f(v, *idx, data).map(|d| (v.clone(), *idx, d))).collect(), bad.clone()),
            VariantData::Single(v, idx, data) => f(v, *idx, data).map_or(VariantData::None, |d| VariantData::Single(v.clone(), *idx, d)),
            VariantData::NonePreserving(id, state) => VariantData::NonePreserving(id.clone(), state.clone()),
            VariantData::None => VariantData::None,
        }
    }
    pub fn for_each<F: FnMut(&VariantID, &T)>(&self, mut f: F) {
        match self {
            VariantData::Multi(vec) => vec.iter().for_each(|(v, _, data)| f(v, data)),//It might be better to use analysiscontext? 
            VariantData::Possible(vec, _) => vec.iter().for_each(|(v, _, data)| f(v, data)),//Thought you ideally shouldnt be doing followup queries here
            VariantData::Single(v, _, data) => f(v, data),
            VariantData::NonePreserving(_, _) => {},
            VariantData::None => {},
        }
    }
    pub fn for_missing<F: FnMut(&CompilerState)>(&self, mut f: F) {
        match self {
            VariantData::Multi(_) => {},
            VariantData::Possible(_, bad) => bad.iter().for_each(&mut f),
            VariantData::Single(_, _, _) => {},
            VariantData::NonePreserving(_, state) => f(state),
            VariantData::None => {},
        }
    }

    //This should super duper mega not be used DIRECTLY, its just that resolving if it is complete or not is expensive, and i chain these together
    //So ima just leave this like this and assume that whenever i call this i ALSO call identify 
    pub fn merge_unchecked(tomerge: Vec<Self>) -> VariantData<T> {
        let mut newlist = Vec::new();
        for each in tomerge {
        match each {
            VariantData::Multi(vec) => {
                newlist.extend(vec);
            }
            VariantData::Possible(vec, _) => {
                newlist.extend(vec);
            }
            VariantData::Single(when, idx, data) => {
                newlist.push((when, idx, data));
            }
            _ => ()
        }
        }
        VariantData::Multi(newlist)
    }
    pub fn extend_unchecked(self, other: Self) -> Self{
        let new = match other {
            VariantData::Multi(vec) => vec,
            VariantData::Possible(vec, _) => vec,
            VariantData::Single(when, idx, data) => vec![(when, idx, data)],
            _ => vec![],
        };
        let old = match self {
            VariantData::Multi(vec) => vec,
            VariantData::Possible(vec, _) => vec,
            VariantData::Single(when, idx, data) => vec![(when, idx, data)],
            _ => vec![],
        };
        let mut combined = old;//This should be optimized away i hope
        combined.extend(new);
        if combined.is_empty() {
            return VariantData::None;
        }
        VariantData::Multi(combined)
        
    }


    pub fn resolve(self, context: &VariantID) -> VariantData<T> {
        //Resolves tree for a given variant, this is essentially what turns this into a random vector into the actual thing this is
        //Lets us identify if this has a guaranteed "something" here
        let target = &context.state;

        let mut items = match self {
            VariantData::Multi(vec) => vec,
            VariantData::Possible(vec, _) => vec,
            VariantData::Single(when, idx, data) => vec![(when, idx, data)],
            _ => return VariantData::NonePreserving(context.file.clone(), context.state.clone()),
        };

        let resolved = target.try_resolve(items.iter().map(|(x, idx, y)| &x.state).collect::<Vec<_>>());
        //Ok represents a single state that perfectly accepts target state, ideal outcome (but not that common)
        //Err represents a vector of states that are missing, if target state takes any of these then we can guarantee that *something* will match
        //If the vector is empty, then while we cannot guarantee "a" match, we can guarantee that at least one of the items will match 
        // IE, SERVER will always match with one of SERVER && !MP or SERVER && MP
        if let Ok(resolved) = resolved {
            let find = items.into_iter().find(|(x, _, _)| x.state == resolved);
            if let Some((when, idx, data)) = find {
                return VariantData::Single(when.clone(), idx, data);
            }
            panic!("State had a guaranteed match, but no item matched it? this shouldnt be a thing");
        }
        let resolved = resolved.unwrap_err();
        if resolved.is_empty(){
            //Returned vector is essentially the "missing" states, if empty then at least *something* will always match
            match items.len(){
                1 => {
                    let item = items.remove(0);
                    eprintln!("Resolved {:?} with {:?} to a single item but without returning Ok? somethings wrong here", context, item.0);
                    return VariantData::Single(item.0, item.1, item.2);
                } 
                _ => {
                    //If we have multiple matches then return a multi dependency
                    return VariantData::Multi(items);
                }
            }
        }
        if items.is_empty(){
            VariantData::NonePreserving(context.file.clone(), context.state.clone())
        }else{ //If we have multiple matches then return a multi dependency
            VariantData::Possible(items, resolved)
        }
    }
}

impl<T: Debug> Debug for VariantData<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VariantData::Multi(vec) => f.debug_list().entries(vec).finish(),
            VariantData::Possible(vec, bad) => f.debug_struct("Possible")
                .field("variants", &vec)
                .field("bad_states", &bad)
                .finish(),
            VariantData::Single(v, idx, data) => f.debug_tuple("Single").field(v).field(idx).field(data).finish(),
            VariantData::NonePreserving(id, state) => f.debug_struct("NonePreserving")
                .field("file", &id)
                .field("state", &state)
                .finish(),
            VariantData::None => f.write_str("None"),
        }
    }
}