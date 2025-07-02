use std::{collections::HashMap, sync::{Arc, RwLock}};

use compiler_state::{state, CompilerState};
use downcast_rs::{impl_downcast, DowncastSync};
use preprocessor::VariantID;

use crate::{results::AnalysisResult, tree::VariantData};

pub struct CachedResults<T: AnalysisResult>{
    results: RwLock<HashMap<CompilerState, VariantData<Arc<T>>>>,
}
impl<T: AnalysisResult> Default for CachedResults<T> {
    fn default() -> Self {
        CachedResults {
            results: RwLock::new(HashMap::new()),
        }
    }
}
impl<T: AnalysisResult> CachedResults<T> {
    pub fn get(&self, state: &CompilerState) -> Option<VariantData<Arc<T>>> {
        self.results.read().unwrap().get(state).cloned()
    }
    
    pub fn insert(&self, state: CompilerState, result: VariantData<Arc<T>>) {
        self.results.write().unwrap().insert(state, result);
    }
    
    pub fn clear(&self) {
        self.results.write().unwrap().clear();
    }
    pub fn get_id(&self, id: &VariantID) -> Option<VariantData<Arc<T>>> {
        // This is a bit of a hack, but it works for now
        // Ideally, we would have a more robust way to get results by VariantID
        self.results.read().unwrap().get(&id.state).cloned()
    }
}
pub trait DowncastableCache: DowncastSync {
    fn guestimate_memory_usage(&self) -> usize;
}
impl_downcast!(sync DowncastableCache);
impl<T: AnalysisResult> DowncastableCache for CachedResults<T> {
    fn guestimate_memory_usage(&self) -> usize {
        
    }
}