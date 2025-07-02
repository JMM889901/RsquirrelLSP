use std::{any::{Any, TypeId}, collections::HashMap, sync::{Arc, RwLock}, usize};

use downcast_rs::{impl_downcast, DowncastSync};
use preprocessor::{PreprocessedVariant, VariantID};
use rayon::{iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator}, result};

use crate::{analysis_error::AnalysisError, cache::DowncastableCache, results::{AnalysisResult, AnalysisResults}, Analyser, AnalysisContext};

impl Analyser {
    pub fn insert_step<Step: AnalysisStep<Return>, Return: AnalysisResult>(&mut self, step: Step, preserve: PreserveType) {
        let step_info = StepInfo::default_store(
            Box::new(step),
            preserve,
        );
        self.steps.insert(TypeId::of::<Return>(), Arc::new(step_info));
        self.cached_trees.insert(TypeId::of::<Return>(), Arc::new(
            crate::cache::CachedResults::<Return>::default()
        ));
        self.sort_steps();
    }
    pub fn sort_steps(&mut self) {
        //TODO: I should probably test for circular dependencies, but honestly just err, dont have one? 
        //Skill issue i guess
        //This isnt a package manager ffs
        self.steps.sort_by(|ak, av, bk, bv| {
            let a_deps = av.depends_on();
            let b_deps = bv.depends_on();
            //If a depends on b, then a should come after b
            if a_deps.contains(&bv.return_type()) {
                return std::cmp::Ordering::Greater;
            } else if b_deps.contains(&av.return_type()) {
                return std::cmp::Ordering::Less;
            }
            std::cmp::Ordering::Equal
        });
    }

    pub fn run_steps(&mut self) {
        self.prebuild_trees();
        //don't parallelise yet since subsquent steps may depend on prior steps, i definitely did NOT do that by accident and get confused
        let iter = self.steps.values();
        for step in iter {
            let vars = self.variants.as_slice();
            step.analyse(vars, self);
        }
    }
    pub fn run_variant_anonymous(&mut self, variant: &VariantID) -> Result<HashMap<TypeId, Arc<dyn AnalysisResult>>, AnalysisError> {
        let mut map = HashMap::new();
        //This is for files that are not in the load order, so they have no associated json entry
        //This is mostly for debugging and testing purposes
        for step in self.steps.values() {
            let res = step.analyse_anonymous(variant, self)?;
            map.insert(step.return_type(), res);
        }
        Ok(map)
    }
    //I would prefer a borrow but downcasted values are technically owned, i could just use downcast_ref but i dont feel like dealing with the lifetime issues that would bring
    pub fn get_step_info<ResultType: AnalysisResult>(&self) -> Result<Arc<StepInfo<ResultType>>, AnalysisError> {
        let step = self.steps.get(&TypeId::of::<ResultType>());
        if let Some(step) = step {
            let downcast: Result<Arc<StepInfo<ResultType>>, Arc<_>> = step.clone().downcast_arc();
            if let Ok(downcast) = downcast {
                return Ok(downcast);
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




/// Steps should be stateless, or at least should reset the state when analyse is called
/// Steps may get the results of prior runs, but should not get the results of its own run for other files, or future runs
pub trait AnalysisStep<T> : Sync + Send + 'static {
    fn analyse(&self, context: &AnalysisContext) -> Result<T, AnalysisError>;
        fn step_name(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }
    /// A list of typeids for the RETURN types of steps that must run before this step
    fn depends_on(&self) -> Vec<TypeId> {
        Vec::new()
    }
}

/// The degree to which results can be preserved from one run to another
/// 
/// Updating the mod.json or otherwise changing the file structure always causes a full re-parse
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreserveType{
    /// This should generally never be used, I can't think of why you would
    Always,
    /// This should re-run for the edited file and any files AFTER it (in the load order)
    Before,
    /// This should always re-run if any file was edited
    Never,
    /// This only needs to be re-run on the changed file
    Unchanged,
    //TODO: I guess i could have a lambda option but lets be real, i wont use it, and i dont feel like putting in that much effort
}



pub trait DowncastableStepInfo: Send + Sync + DowncastSync {
    fn analyse(&self, vars: &[VariantID], analyser: &Analyser);

    /// Analyse a single variant, this is mostly for files which do not exist in the load order and have just been randomly opened without an associated json entry
    fn analyse_anonymous(&self, var: &VariantID, analyser: &Analyser) -> Result<Arc<dyn AnalysisResult>, AnalysisError>;
    

    fn step_name(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }
    fn step_scope_errors(&self) -> Vec<AnalysisError>;
    fn errors(&self) -> Vec<AnalysisError> {
        self.step_scope_errors()
    }
    fn step_type(&self) -> TypeId;
    fn return_type(&self) -> TypeId;
    fn preserve_type(&self) -> &PreserveType;
    fn depends_on(&self) -> Vec<TypeId>;

}
impl_downcast!(sync DowncastableStepInfo);
pub struct StepInfo<U: AnalysisResult> {
    pub step: Box<dyn AnalysisStep<U>>,
    pub preserve: PreserveType,
    pub return_type: TypeId,
    pub results: RwLock<AnalysisResults<U>>,
}
impl<U: AnalysisResult> StepInfo<U> {
    pub fn default_store(step: Box<dyn AnalysisStep<U>>, preserve: PreserveType) -> Self {
        Self {
            step,
            preserve,
            return_type: TypeId::of::<U>(),
            results: RwLock::new(AnalysisResults::default()),
        }
    }
}

//This all relies on rayon maintaining the order when collecting a multithreaded iterator, i sure hope it does that!
impl<U: AnalysisResult> DowncastableStepInfo for StepInfo<U> {
    fn step_scope_errors(&self) -> Vec<AnalysisError> {
        self.results.read().unwrap().errors.clone()
    }
    fn analyse(&self, vars: &[VariantID], analyser: &Analyser){
        let mut result_write = self.results.write().unwrap();
        let errs = RwLock::new(Vec::new());
        let iter = vars.par_iter().enumerate();
        let results: Vec<Option<Arc<U>>> = iter
        //These need to be option since i need to preserve order, skipping fails kinda fucks indexing
            .map(|(i, file)| {
                let context = AnalysisContext {
                    variant_idx: i,
                    analyser,
                    variant: file,
                };
                match self.step.analyse(&context) {
                    Ok(result) => Some(Arc::new(result)),
                    Err(err) => {
                        errs.write().unwrap().push(err);//Technically creating two vectors might be better, but it would require extra iterations across the list
                        //And i really shouldnt be encountering enough of these errors for that to actually BE faster
                        //Errors here should only be fatal, and fatal errors are something i am explicitly trying not to have
                        None
                    }
                }
            }).collect();
        
        let errs = errs.into_inner().unwrap();
        if !errs.is_empty() {
            result_write.errors.extend(errs);
        }//TODO: This preserves errors from prior runs, which is pointless
        result_write.results = results;

    }
    fn analyse_anonymous(&self, var: &VariantID, analyser: &Analyser) -> Result<Arc<dyn AnalysisResult>, AnalysisError> {
        let context = AnalysisContext {
            variant_idx: usize::MAX,
            analyser,
            variant: var,
        };
        match self.step.analyse(&context) {
            Ok(result) => Ok(Arc::new(result)),
            Err(err) => Err(err),
        }
    }
    fn return_type(&self) -> TypeId {
        self.return_type
    }
    fn step_type(&self) -> TypeId {
        self.step.as_ref().type_id()
    }

    fn step_name(&self) -> String {
        self.step.step_name()
    }

    fn preserve_type(&self) -> &PreserveType {
        &self.preserve
    }

    fn depends_on(&self) -> Vec<TypeId> {
        self.step.depends_on()
    }

}