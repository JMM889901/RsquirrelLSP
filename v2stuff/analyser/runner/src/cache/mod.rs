mod prebuilt_trees;
mod result_cache;
mod query_cache;
use std::{any::TypeId, sync::Arc};

use preprocessor::VariantID;
pub use result_cache::{CachedResults, DowncastableCache};

use crate::{analysis_error::AnalysisError, results::AnalysisResult, tree::VariantData, Analyser};
pub enum CacheLevel<ResultType> {
    None,
    Prebuilt(VariantData<()>),
    Result(VariantData<Arc<ResultType>>),
}

impl Analyser {
    pub fn get_best_cache<ResultType: AnalysisResult>(&self, variant_id: &VariantID) -> Result<CacheLevel<ResultType>, AnalysisError> {
        if let Some(cached) = self.cached_trees.get(&TypeId::of::<ResultType>()) {
            let cast = cached.clone().downcast_arc::<CachedResults<ResultType>>();
            if let Ok(cached) = cast {
                if let Some(result) = cached.get_id(variant_id) {
                    return Ok(CacheLevel::Result(result));
                }
            } else {
                return Err(AnalysisError::ResultsDowncastFailed {
                    step: std::any::type_name::<ResultType>().to_string(),
                    expected: TypeId::of::<ResultType>(),
                    found: cached.type_id(),
                });
            }
        }
        if let Some(prebuilt) = self.prebuilt_trees.get(&variant_id.state) {
            return Ok(CacheLevel::Prebuilt(prebuilt.clone()));
        }
        Ok(CacheLevel::None)
    }
}