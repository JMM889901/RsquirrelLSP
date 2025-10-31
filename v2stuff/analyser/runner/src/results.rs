use std::{any::TypeId, collections::HashMap, path::PathBuf, sync::{Arc, RwLock}};

use compiler_state::CompilerState;
use downcast_rs::{impl_downcast, DowncastSend, DowncastSync};
use mod_loader::fileinfo::FileInfo;
use preprocessor::VariantID;

use crate::{analysis_error::AnalysisError, step::StepInfo, Analyser, AnalysisContext};

type StepInfoCast<ResultType> = Arc<StepInfo<ResultType>>;

impl AnalysisContext<'_> {
    /// Get prior result for a given step of this variant, this assumes you are using AnalysisResults and have not made a different contaner
    /// 
    pub fn get_own_result<ResultType : AnalysisResult>(&self) -> Result<Arc<ResultType>, AnalysisError> {
        let res = self.analyser.get_results::<ResultType>()?;
        if let Some(Some(result)) = res.get(self.variant_idx){
            return Ok(result.clone());
        }
        Err(AnalysisError::VariantLacksResult { 
            variant: self.variant_idx, 
            step: std::any::type_name::<ResultType>().to_string() 
        })
    }
}

impl Analyser {
    pub(crate) fn get_results<ResultType: AnalysisResult>(&self) -> Result<Vec<Option<Arc<ResultType>>>, AnalysisError> {
        let step = self.steps.get(&TypeId::of::<ResultType>());
        if let Some(step) = step {
            let downcast: Result<StepInfoCast<ResultType>, Arc<_>> = step.clone().downcast_arc();
            if let Ok(downcast) = downcast {
                let results = downcast.results.read().unwrap();
                return Ok(results.results.to_vec());
            }
            return Err(AnalysisError::ResultsDowncastFailed {
                step: std::any::type_name::<ResultType>().to_string(),
                expected: TypeId::of::<ResultType>(),
                found: step.type_id(),
            });
        }
        Err(AnalysisError::ResultsNotFound {
            step: std::any::type_name::<ResultType>().to_string(),
        })
    }
}


//I was going to allow custom ones of these hence the trait, but thats a lot of work for something nobody will use
pub trait DowncastableResults : DowncastSync{}
pub trait AnalysisResult : 'static + Send + Sync {}
impl_downcast!(sync DowncastableResults);
pub struct AnalysisResults<T: AnalysisResult> {
    pub results: Vec<Option<Arc<T>>>,
    ///Step wide errors without a result
    pub errors: Vec<AnalysisError>,
}
impl<T: AnalysisResult> Default for AnalysisResults<T> {
    fn default() -> Self {
        AnalysisResults {
            results: Vec::new(),
            errors: Vec::new(),
        }
    }
}
impl<T: AnalysisResult> AnalysisResults<T> {

    //Inserting into this is risky, we assume that the results match the order of the files
    //I need to profile but it seems that downcasting is more expensive than hashmap indexing? 
    pub fn get_result(&self, idx: usize) -> Option<Arc<T>> {
        self.results.get(idx).and_then(|r| r.as_ref().cloned())
    }

}

//I don't like that 'static bound, i should not do that
impl<T: AnalysisResult> DowncastableResults for AnalysisResults<T>{}
