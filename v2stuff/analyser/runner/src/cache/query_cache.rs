use std::{any::TypeId, collections::HashMap, process::Output, sync::{Arc, RwLock}};

use compiler_state::CompilerState;
use rayon::iter::Either;//This is kinda stupid, i shouldnt use rayons either, but i'm lazy

use crate::{cache::DowncastableCache, query::CachableQuery, results::AnalysisResult, tree::VariantData, Analyser};

type CachedOr<Output> = Either<usize, Arc<VariantData<Arc<Output>>>>;
//This shouldnt need to be an arc^, but downcasting is stupid
//I really truly dont know why i didnt just use an enum
//This is so much shit for literally just worse enums
//I am my own worst enemy
impl Analyser {
    pub fn cached_query<ResultType: AnalysisResult, Output: Send + Sync + 'static, Query: CachableQuery<ResultType, Output>>(
        &self,
        variant_id: &CompilerState,
        query: &Query,
    ) -> Option<Arc<VariantData<Arc<Output>>>> {
        let queries = self.cached_queries.read().unwrap();
        let cache = queries.get(&TypeId::of::<Query>())?;
        let cast = cache.clone().downcast_arc::<RwLock<QueryCache<Query, Output, ResultType>>>();
        if let Ok(cache) = cast {
            let cache = cache.read().unwrap();
            if let Some(cached) = cache.get(variant_id, query) {
                match cached {
                    CachedOr::Left(idx) => {
                        // Return the cached result
                        None
                    }
                    CachedOr::Right(data) => {
                        // Return the VariantData
                        Some(data.clone())
                    }
                }
            } else {
                None
            }
        } else {
            eprintln!("Failed to downcast cache for query: {}", std::any::type_name::<Query>());
            None
        }
    }

    pub fn cache_query<ResultType: AnalysisResult, Output: Send + Sync + 'static, Query: CachableQuery<ResultType, Output>>(
        &self,
        result: VariantData<Arc<Output>>,
        variant_id: &CompilerState,
        query: Query,
    ) -> Result<Arc<VariantData<Arc<Output>>>, VariantData<Arc<Output>>> {
        if !self.config.cache_queries() {
            return Err(result);
        }
        let mut queries = self.cached_queries.write().unwrap();
        let cache = queries.entry(TypeId::of::<Query>())
            .or_insert_with(|| Arc::new(RwLock::new(QueryCache::<Query, Output, ResultType>::default())));
        let cast = cache.clone().downcast_arc::<RwLock<QueryCache<Query, Output, ResultType>>>();
        if let Ok(cache) = cast {
            let cache_after = self.config.cache_queries_after();
            let mut cache = cache.write().unwrap();
            match cache.get_mut(variant_id, &query) {
                Some(CachedOr::Left(idx)) if *idx < cache_after => {
                    Err(result)
                }
                Some(CachedOr::Left(idx)) => {
                    // If the cache is full, we replace the oldest entry
                    // This is a bit of a hack, but it works for now
                    *idx += 1; 
                    Err(result)
                }
                _ => {
                    // Otherwise, we cache the result
                    let res = Arc::new(result);
                    cache.insert(variant_id.clone(), query, CachedOr::Right(res.clone()));
                    Ok(res)
                }
            }
        } else {
            eprintln!("Failed to downcast cache for query: {}", std::any::type_name::<Query>());
            Err(result)
        }
    }

}


pub struct QueryCache<Query: CachableQuery<ResultType, Output>, Output, ResultType: AnalysisResult> {
    queries: HashMap<CompilerState, HashMap<Query, CachedOr<Output>>>,
    _phantom: std::marker::PhantomData<ResultType>,
}
impl<Query: CachableQuery<ResultType, Output>, Output: Send + Sync + 'static, ResultType: AnalysisResult> Default for QueryCache<Query, Output, ResultType> {
    fn default() -> Self {
        QueryCache {
            queries: HashMap::new(),
            _phantom: std::marker::PhantomData,
        }
    }
}
impl<Query: CachableQuery<ResultType, Output>, Output: Send + Sync + 'static, ResultType: AnalysisResult> QueryCache<Query, Output, ResultType> {
    pub fn get(&self, state: &CompilerState, query: &Query) -> Option<&CachedOr<Output>> {
        self.queries.get(state).and_then(|q| q.get(query))
    }
    pub fn get_mut(&mut self, state: &CompilerState, query: &Query) -> Option<&mut CachedOr<Output>> {
        self.queries.get_mut(state).and_then(|q| q.get_mut(query))
    }

    pub fn insert(&mut self, state: CompilerState, query: Query, result: CachedOr<Output>) {
        self.queries.entry(state).or_default().insert(query, result);
    }

    pub fn clear(&mut self) {
        self.queries.clear();
    }
}
impl<Query: CachableQuery<ResultType, Output>, Output: Send + Sync + 'static, ResultType: AnalysisResult> DowncastableCache for RwLock<QueryCache<Query, Output, ResultType>> {
    fn guestimate_memory_usage(&self) -> usize {
        self.read().unwrap().queries.iter().map(|(state, queries)| {
            let size = std::mem::size_of_val(state) + queries.len() * std::mem::size_of::<Query>();
            size
        }).sum()
    }
}