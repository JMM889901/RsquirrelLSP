use core::panic;
use std::fmt::Debug;

use super::*;


pub trait DowncastableData : DowncastSync{}
impl_downcast!(sync DowncastableData);

//Comp tree is effectively a successor to the dependency tree from load_order.rs, its basically the same thing but that was not build to be used as a generic thingy

//Unlike a dependency tree, this is not a list of files we require, it is the list of variants which can be be active whenever this one is 

//Unlike the old one, this is stored by state not variant. Simply because theres actually not *THAT* many states when compared to variants (Which in hindsight, makes a lot of sense)
//I need to stress that this is primarily just an optimisation thing, realistically you can do all this at parse for the same effect
pub enum VariantData<T> {
    Multi(Vec<(SQDistinctVariant, T)>),
    Possible(Vec<(SQDistinctVariant, T)>, Vec<CompiledState>),//Second vec is the mismatching states, IE what is preventing this from being a guaranteed match
    Single(SQDistinctVariant, T),
    NonePreserving(FileInfo, CompiledState),//Where practical its probably worth storing this
    None,
}
impl<T: AnalysisResultInternal> DowncastableData for VariantData<Arc<T>>{}

impl<T: Clone> Clone for VariantData<T> {
    fn clone(&self) -> Self {
        match self {
            VariantData::Multi(vec) => VariantData::Multi(vec.clone()),
            VariantData::Possible(vec, bad) => VariantData::Possible(vec.clone(), bad.clone()),
            VariantData::Single(v, data) => VariantData::Single(v.clone(), data.clone()),
            VariantData::NonePreserving(id, state) => VariantData::NonePreserving(id.clone(), state.clone()),
            VariantData::None => VariantData::None,
        }
    }
}
impl VariantData<Arc<dyn AnalysisResultInternal>> {//This makes me feel so safe and not at all in grave danger
    pub fn into_cast<T: AnalysisResultInternal + 'static>(self) -> VariantData<Arc<T>> {
        match self {
            VariantData::NonePreserving(id, state) => return VariantData::NonePreserving(id, state),
            VariantData::None => return VariantData::None,
            VariantData::Single(v, data) => return VariantData::Single(v, data.downcast_arc::<T>().expect("wuh")),
            _ => ()
        }
        let mut new_bad = None;;
        let vec = match self {
            VariantData::Multi(vec) => vec,
            VariantData::Possible(vec, bad) => {new_bad = Some(bad); vec},
            _ => panic!("oof"),
        };
        let ptr = vec.as_ptr() as *const (SQDistinctVariant, Arc<dyn AnalysisResultInternal>);
        let len = vec.len();
        let capacity = vec.capacity();
        std::mem::forget(vec); // Prevent Vec from being dropped
        let new_vec = unsafe { Vec::from_raw_parts(ptr as *mut (SQDistinctVariant, Arc<T>), len, capacity) };
        match new_bad {
            None => VariantData::Multi(new_vec),
            Some(new_bad) => VariantData::Possible(new_vec, new_bad),
            _ => panic!(":("),
        }
    }

        pub fn downcast<T: AnalysisResultInternal + 'static>(&self) -> VariantData<Arc<T>> {
        match self {
            VariantData::Multi(vec) => VariantData::Multi(unsafe { Vec::from_raw_parts(vec.as_ptr() as *mut _, vec.len(), vec.capacity()) }),
            VariantData::Possible(vec, bad) => VariantData::Possible(unsafe { Vec::from_raw_parts(vec.as_ptr() as *mut _, vec.len(), vec.capacity()) }, bad.clone()),
            VariantData::Single(v, data) => VariantData::Single(v.clone(), data.clone().downcast_arc::<T>().expect("wuh")),
            VariantData::NonePreserving(id, state) => VariantData::NonePreserving(id.clone(), state.clone()),
            VariantData::None => VariantData::None,
        }
    }
}
//TODO: I really need to make a better compiledstate representation, at the point it is created i absolutely do know all of the possible conditions, so i should 
//probably just store them as an int or smth instead of strings
impl<T> VariantData<T> {
    pub fn get_first_file(&self) -> Option<&FileInfo> {
        match self {
            VariantData::Multi(vec) => vec.first().map(|(v, _)| &v.0.file),
            VariantData::Possible(vec, _) => vec.first().map(|(v, _)| &v.0.file),
            VariantData::Single(v, _) => Some(&v.0.file),
            VariantData::NonePreserving(id, _) => Some(id),
            VariantData::None => None,
        }
    }
    pub fn from_vec<B:HasVariantID>(vec: Vec<(SQDistinctVariant, T)>, id: &B) -> Self {
        let temp = Self::Multi(vec);
        temp.identify(id)

    }
    pub fn map<U, F: FnMut(&SQDistinctVariant, &T) -> U>(&self, f: &mut F) -> VariantData<U> {
        match self {
            VariantData::Multi(vec) => VariantData::Multi(vec.into_iter().map(|(v, data)| (v.clone(), f(v, data))).collect()),
            VariantData::Possible(vec, bad) => VariantData::Possible(vec.into_iter().map(|(v, data)| (v.clone(), f(v, data))).collect(), bad.clone()),
            VariantData::Single(v, data) => VariantData::Single(v.clone(), f(v, data)),
            VariantData::NonePreserving(id, state) => VariantData::NonePreserving(id.clone(), state.clone()),
            VariantData::None => VariantData::None,
        }
    }

    ///SUPER IMPORTANT: This can and will change completion state, its up to you (by which i mean future me) to identify() afterwards 
    pub fn filter_map<U, F: FnMut(&SQDistinctVariant, &T) -> Option<U>>(&self, f: &mut F) -> VariantData<U> {
        match self {
            VariantData::Multi(vec) => VariantData::Multi(vec.into_iter().filter_map(|(v, data)| f(v, data).map(|d| (v.clone(), d))).collect()),
            VariantData::Possible(vec, bad) => VariantData::Possible(vec.into_iter().filter_map(|(v, data)| f(v, data).map(|d| (v.clone(), d))).collect(), bad.clone()),
            VariantData::Single(v, data) => f(v, data).map_or(VariantData::None, |d| VariantData::Single(v.clone(), d)),
            VariantData::NonePreserving(id, state) => VariantData::NonePreserving(id.clone(), state.clone()),
            VariantData::None => VariantData::None,
        }
    }
    pub fn for_each<F: FnMut(&SQDistinctVariant, &T)>(&self, mut f: F) {
        match self {
            VariantData::Multi(vec) => vec.iter().for_each(|(v, data)| f(v, data)),
            VariantData::Possible(vec, _) => vec.iter().for_each(|(v, data)| f(v, data)),
            VariantData::Single(v, data) => f(v, data),
            VariantData::NonePreserving(_, _) => {},
            VariantData::None => {},
        }
    }
    pub fn for_missing<F: FnMut(&CompiledState)>(&self, mut f: F) {
        match self {
            VariantData::Multi(_) => {},
            VariantData::Possible(_, bad) => bad.iter().for_each(|state| f(state)),
            VariantData::Single(_, _) => {},
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
            VariantData::Single(when, data) => {
                newlist.push((when, data));
            }
            _ => ()
        }
        }
        return VariantData::Multi(newlist);
    }
    pub fn extend_unchecked(self, other: Self) -> Self{
        let new = match other {
            VariantData::Multi(vec) => vec,
            VariantData::Possible(vec, _) => vec,
            VariantData::Single(when, data) => vec![(when, data)],
            _ => vec![],
        };
        let old = match self {
            VariantData::Multi(vec) => vec,
            VariantData::Possible(vec, _) => vec,
            VariantData::Single(when, data) => vec![(when, data)],
            _ => vec![],
        };
        let mut combined = old;//This should be optimized away i hope
        combined.extend(new);
        if combined.is_empty() {
            return VariantData::None;
        }
        return VariantData::Multi(combined);
        
    }
    pub fn get_inner(self) -> Vec<(SQDistinctVariant, T)> {
        match self {
            VariantData::Multi(vec) => vec,
            VariantData::Possible(vec, _) => vec,
            VariantData::Single(v, data) => vec![(v, data)],
            VariantData::NonePreserving(_, _) => vec![],
            VariantData::None => vec![],
        }
    }
    pub fn identify<B: HasVariantID>(self, context: &B) -> VariantData<T> {
        //Updates the variant depending on if we now have a guaranteed match, expensive :(

        let target = context.get_state();
        let mut items = match self {
            VariantData::Multi(vec) => vec,
            VariantData::Possible(vec, _) => vec,
            VariantData::Single(when, data) => vec![(when, data)],
            _ => return VariantData::NonePreserving(context.get_file().clone(), context.get_state().clone()),
        };
        //There should really never be any reason to filter this bit since they *should* always be accepted by the time we get here
        //Apply increasing filters to the context
        //let filter = items.iter().filter(|(file, data)| {

            //If this context explicitly contradicts the provided context then skip it
        //    !target.do_i_reject_explicit(&file.state)
//
        //}).map(|x| x.clone()).collect::<Vec<_>>();

        //if will_allways_pass(provided, &filter.iter().map(|x| x.0.clone()).collect()){
        let resolved = target.try_resolve(&items.iter().map(|(x, y)| x.0.state.clone()).collect::<Vec<_>>());
        if resolved.is_empty(){
            //If we have a match then return the first one
            if items.len() == 1{
                let item = items.remove(0);
                return VariantData::Single(item.0, item.1);
            }else if items.len() > 1{
                //If we have multiple matches then return a multi dependency
                return VariantData::Multi(items);
            }
        }
        if items.len() == 0{
            return VariantData::NonePreserving(context.get_file().clone(), context.get_state().clone());
        }else{ //If we have multiple matches then return a multi dependency
            return VariantData::Possible(items, resolved);
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, VariantData::None) || matches!(self, VariantData::NonePreserving(_, _))
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
            VariantData::Single(v, data) => f.debug_tuple("Single").field(v).field(data).finish(),
            VariantData::NonePreserving(id, state) => f.debug_struct("NonePreserving")
                .field("file", &id)
                .field("state", &state)
                .finish(),
            VariantData::None => f.write_str("None"),
        }
    }
}
//TODO: Flatmap here? if i need to do it more then once maybe
impl<T> VariantData<Vec<T>> {
    pub fn flatten(&self) -> Vec<&T> {
        match self {
            VariantData::Multi(vec) => vec.iter().map(|(_, data)| data).flatten().collect(),
            VariantData::Possible(vec, _) => vec.iter().map(|(_, data)| data).flatten().collect(),
            VariantData::Single(_, data) => data.iter().collect(),
            VariantData::NonePreserving(_, _) => vec![],
            VariantData::None => vec![],
        }
    }
    pub fn into_flatten(self) -> Vec<T> {
        match self {
            VariantData::Multi(vec) => vec.into_iter().flat_map(|(_, data)| data).collect(),
            VariantData::Possible(vec, _) => vec.into_iter().flat_map(|(_, data)| data).collect(),
            VariantData::Single(_, data) => data,
            VariantData::NonePreserving(_, _) => vec![],
            VariantData::None => vec![],
        }
    }
}

//This really sucks
pub fn for_file(content: &Vec<SQDistinctVariant>, provided: &CompiledState) -> Option<VariantData<()>> {
        //Apply increasing filters to the context
    let mut filter = content.iter().filter(|(file)| {

        //If this context explicitly contradicts the provided context then skip it
        !provided.do_i_reject_explicit(&file.0.state)

    }).map(|x| (x.clone(), ())).collect::<Vec<_>>();

    //if will_allways_pass(provided, &filter.iter().map(|x| x.0.clone()).collect()){
    let resolved = provided.try_resolve(&filter.iter().map(|(x, y)| x.0.state.clone()).collect::<Vec<_>>());
        if resolved.is_empty(){
        //If we have a match then return the first one
        if filter.len() == 1{
            let item = filter.remove(0);
            return Some(VariantData::Single(item.0, item.1));
        }else if filter.len() > 1{
            //If we have multiple matches then return a multi dependency
            return Some(VariantData::Multi(filter));
        }
    }
    if filter.len() == 0{
        return None;
    }else{ //If we have multiple matches then return a multi dependency
        return Some(VariantData::Possible(filter, resolved));
    }
}

