/* 
    Allow a given step/variant to query the results of prior steps in abstractly, they should never directly interact with preprocessor stuff or the individual results themselves
    Some fancy caching shenanigans will probably need to happen for this to be fast, but this is honestly just a recreation of the old thing that did exactly this

    So I don't know why I'm acting like "probably" be needed, it will, and it's also quite easy to do
*/

use std::{any::TypeId, collections::HashMap, hash::Hash, sync::Arc};

use compiler_state::state::CompilerState;
use preprocessor::VariantID;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{analysis_error::AnalysisError, cache::{CacheLevel, CachedResults}, results::{AnalysisResult, AnalysisResults}, tree::VariantData, Analyser, AnalysisContext, PrebuildTrees}; 

pub enum QueryMask{
    All,
    Before,
    After,
}

pub trait CachableQuery<ResultType: AnalysisResult, Output> : Clone + Hash + Eq + Send + Sync + 'static {
    fn apply(&self, variant_id: &VariantID, idx: usize, result: &ResultType) -> Option<Output>;
}

impl Analyser {
    pub fn query_fn<ResultType: AnalysisResult, Output, F: Fn(&VariantID, usize, &ResultType) -> Option<Output>>(&self, variant_id: &AnalysisContext, query_fn: F, mask: QueryMask) -> Result<VariantData<Output>, AnalysisError> {
        //TODO: I'm not sure what the best way of caching this is, since it does need to iterate over everything again to apply the fn
        //All caching stands to gain is not needing to redo state resolving, but we do kinda need to anyway because filtering
        self.build_query_tree(variant_id, query_fn, mask)//TODO: this should be the fallback for cached results, but i am not sure yet if cached stuff would be all that much faster honestly
        //I suppose if VariantData preserves order then skipwhile on that would be faster than this? but the following resolve will still take *some* time, just less
    }

    pub fn query<ResultType: AnalysisResult, Output: Send + Sync + 'static, Q: CachableQuery<ResultType, Output>>(&self, context: &AnalysisContext, query: Q, mask: QueryMask) -> Result<VariantData<Arc<Output>>, AnalysisError> {

        if self.config.cache_queries() {
            if let Some(cached) = self.cached_query(&context.variant.state, &query) {
                return self.from_cached_query::<ResultType, Output, Q>(context, cached.as_ref(), mask);
            }
            let result = self.build_query_tree(context, |v, idx, res| query.apply(v, idx, res).map(Arc::new), QueryMask::All)?;//For caching might as well grab everything
            let res = self.cache_query(result, &context.variant.state, query);
            match res {
                Ok(res) => return self.from_cached_query::<ResultType, Output, Q>(context, res.as_ref(), mask),
                Err(result) => {
                    return Ok(result);
                }
            }
        }
        let result = self.build_query_tree(context, |v, idx, res| query.apply(v, idx, res).map(Arc::new), mask)?;
        Ok(result)
    }

    //For a cache that sure is a lot of very expensive clones
    pub fn from_cached_query<ResultType: AnalysisResult, Output: Send + Sync + 'static, Q: CachableQuery<ResultType, Output>>(&self, context: &AnalysisContext, res: &VariantData<Arc<Output>>, mask: QueryMask) -> Result<VariantData<Arc<Output>>, AnalysisError> {
        match mask {//TODO: This can be optimized to stop when the first idx < target happens
            //I'd much MUCH rather do borrows than arcs, but that causes downcasting lifetime shenanigans 
            //Reason number 9331 why i should have just used enums
            QueryMask::All => Ok(res.clone().resolve(context.variant)),
            QueryMask::Before => {
                let filtered = res.filter_map(&mut |id, idx, value| {
                    if idx < context.variant_idx {
                        Some(value.clone())
                    } else {
                        None
                    }
                });
                Ok(filtered.resolve(context.variant))
            },
            QueryMask::After => {
                let filtered = res.filter_map(&mut |id, idx, value| {
                    if idx > context.variant_idx {
                        Some(value.clone())
                    } else {
                        None
                    }
                });
                Ok(filtered.resolve(context.variant))
            },
        }
    }


    pub fn build_query_tree<ResultType: AnalysisResult, Output, F: Fn(&VariantID, usize, &ResultType) -> Option<Output>>(&self, variant: &AnalysisContext, map_fn: F, mask: QueryMask) -> Result<VariantData<Output>, AnalysisError> {
        let result = self.get_results::<ResultType>()?;
        assert!(result.len() == self.variants.len(), "Result length does not match variants length");
        let iter = self.variants.iter().zip(result).enumerate();
        let iter: Box<dyn Iterator<Item = _>> = match mask {
            QueryMask::All => Box::new(iter),
            QueryMask::Before => Box::new(iter.take(variant.variant_idx)),//should this be takewhile?
            QueryMask::After => Box::new(iter.skip(variant.variant_idx + 1)),
        };
        let res = VariantData::Multi(iter.filter_map(|(idx, (v, res))| {
            if v.state.rejects_explicit(&variant.variant.state) {
                return None; //This variant is not compatible with the target state, so skip it
            }
            if let Some(res) = res {
                if let Some(res) = map_fn(v, idx, &res) {
                    return Some((v.clone(), idx, res));
                }
            }
            None
        }).collect::<Vec<_>>());
        let res = res.resolve(variant.variant);
        if res.is_some() {
            return Ok(res);
        }
        Err(AnalysisError::TreeBuildFailed {
            variant: variant.variant.clone(),
            reason: "No matching variants found, even though self is in the tree".to_string(),
        })
    }
}



/// Primitive get result functions
impl Analyser {

    pub fn get_results_for_state<ResultType: AnalysisResult>(&self, variant_id: &VariantID) -> Result<VariantData<Arc<ResultType>>, AnalysisError> {
        match self.get_best_cache::<ResultType>(variant_id)? {
            CacheLevel::Result(results) => return Ok(results),
            CacheLevel::Prebuilt(prebuilt) => return self.from_prebuilt::<ResultType>(&prebuilt, variant_id),

            CacheLevel::None => (),
        }
        let built = self.build_tree::<ResultType>(variant_id)?;
        if self.config.cache_result() {
            if let Some(cached) = self.cached_trees.get(&TypeId::of::<ResultType>()) {
                let cast = cached.clone().downcast_arc::<CachedResults<ResultType>>();
                match cast {
                    Ok(cached) => {
                        cached.insert(variant_id.state.clone(), built.clone());
                    }
                    Err(_) => {return Err(AnalysisError::ResultsDowncastFailed {
                        step: std::any::type_name::<ResultType>().to_string(),
                        expected: TypeId::of::<ResultType>(),
                        found: cached.type_id(),
                    })}
                }
            } else {
                return Err(AnalysisError::MissingCache { step: std::any::type_name::<ResultType>().to_string() });
            }
        }
        Ok(built)


    }
    pub fn from_prebuilt<ResultType: AnalysisResult>(&self, prebuilt: &VariantData<()>, variant: &VariantID) -> Result<VariantData<Arc<ResultType>>, AnalysisError> {
        let results = self.get_results::<ResultType>()?;
        let mapped = prebuilt.filter_map(&mut |_, idx, _| {
            let result = results.get(idx).and_then(|r| r.as_ref());
            if let Some(result) = result {
                return Some(result.clone());
            }
            None
        });
        if mapped.is_some() {
            return Ok(mapped);
        }
        Err(AnalysisError::TreeBuildFailed {
            variant: variant.clone(),
            reason: "No matching variants found, even though self is in the tree".to_string(),
        })
    }

    pub fn build_tree<ResultType: AnalysisResult>(&self, variant: &VariantID) -> Result<VariantData<Arc<ResultType>>, AnalysisError> {
        let result = self.get_results::<ResultType>()?;
        assert!(result.len() == self.variants.len(), "Result length does not match variants length");
        let iter = self.variants.iter().zip(result).enumerate();
        let res = VariantData::Multi(iter.filter_map(|(idx, (v, res))| {
            if v.state.rejects_explicit(&variant.state) {
                return None; //This variant is not compatible with the target state, so skip it
            }
            if let Some(res) = res {
                return Some((v.clone(), idx, res));
            }
            None
        }).collect::<Vec<_>>());
        let res = res.resolve(variant);
        if res.is_some() {
            return Ok(res);
        }
        Err(AnalysisError::TreeBuildFailed {
            variant: variant.clone(),
            reason: "No matching variants found, even though self is in the tree".to_string(),
        })
    }


}