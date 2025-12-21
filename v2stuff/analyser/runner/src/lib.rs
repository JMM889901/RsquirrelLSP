use std::{any::TypeId, collections::HashMap, path::PathBuf, sync::{Arc, RwLock}};

use compiler_state::CompilerState;
use indexmap::IndexMap;
use preprocessor::{PreprocessedScript, PreprocessedVariant, VariantID};
use std::fmt::{Debug, Display};
use crate::{analysis_error::AnalysisError, cache::DowncastableCache, results::DowncastableResults, step::{AnalysisStep, DowncastableStepInfo, PreserveType, StepInfo}, tree::DowncastableData};

mod runner;
pub mod step;
mod tree;
pub mod results;
pub mod analysis_error;
mod query;
mod cache;

pub struct AnalysisContext<'a> {
    //Index in a given result set, not the index within the files variant list
    pub combined_idx: usize,
    pub analyser: &'a Analyser,
    pub variant: &'a VariantKey

}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantKey {
    pub variant_idx: usize,
    pub file_idx: usize,
    pub variant_id: VariantID,
}
impl Display for VariantKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VariantKey {{ variant_idx: {}, file_idx: {}, variant: {} }}", self.variant_idx, self.file_idx, self.variant_id)
    }
}
impl VariantKey {
    pub fn test_variant(file: PathBuf, state: CompilerState, variant_idx: usize, file_idx: usize) -> Self {
        VariantKey {
            variant_idx,
            file_idx,
            variant_id: VariantID::test_variant(file, state),
        }
    }
}
impl<'a> AnalysisContext<'a> {
}
impl Display for AnalysisContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnalysisContext {{ combined_idx: {}, variant: {} }}", self.combined_idx, self.variant)
    }
}
pub struct StaticVariantID {
    pub path: PathBuf,
    pub state: CompilerState,
}
impl From<&VariantID> for StaticVariantID {
    fn from(variant: &VariantID) -> Self {
        StaticVariantID {
            path: variant.file.file_info.path().clone(),
            state: variant.state.clone(),
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct AnalysisConfig{
    pub prebuild_trees: PrebuildTrees,
    pub cache_trees: CacheTrees,
}
impl AnalysisConfig {
    pub fn cache_result(&self) -> bool {
        matches!(self.cache_trees, CacheTrees::Results | CacheTrees::All)
    }
    pub fn cache_queries(&self) -> bool {
        matches!(self.cache_trees, CacheTrees::Queries | CacheTrees::All)
    }
    pub fn cache_queries_after(&self) -> usize {
        0
        //In future might add requirements (IE, cache after 10 things call it)
        //But that runs the pretty obvious risk of caching something called 11 times on the 10th time, hardly saving any time
    }
}

#[derive(Default)]
pub struct Analyser {
    pub config: AnalysisConfig,
    pub files: Arc<Vec<PreprocessedScript>>,
    pub variants: Vec<VariantKey>,//I dont like this, but i am so mentally done with fucking lifetimes
    ///For future reference, dynamic typing in rust is an absolutely horrendous idea good fucking lord
    /// Why did i do this to myself
    pub steps: IndexMap<TypeId, Arc<dyn DowncastableStepInfo>>,
    pub prebuilt_trees: HashMap<CompilerState, tree::VariantData<()>>,
    pub cached_trees: HashMap<TypeId, Arc<dyn DowncastableCache>>,
    pub cached_queries: RwLock<HashMap<TypeId, Arc<dyn DowncastableCache>>>,//TODO: It doesnt actually matter but i should probably use a different trait since it downcasts to a different type
    pub errors: RwLock<Vec<AnalysisError>>,
}
impl Debug for Analyser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Analyser")
            .field("config", &self.config)
            .field("variants", &self.variants.len())
            .field("steps", &self.steps.values().map(|x| x.step_name()).collect::<Vec<_>>())
            .finish()
    }
}


impl Analyser {

    pub fn push_files(&mut self, files: Arc<Vec<PreprocessedScript>>) {
        self.files = files;
        self.variants = self.files.iter().enumerate().flat_map(|(file_idx, f)| f.variants.iter().map(move |v| VariantKey {
            variant_idx: v.id.variantidx,
            file_idx,
            variant_id: v.id.clone(),
        })).collect();
    }

}

#[derive(Debug, Clone, Copy, Default)]
pub enum PrebuildTrees {
    Always,
    #[default]
    Never,
    After(usize),//If a state exists for this many variants, then prebuild it
}

//Tree caching stores either intermediate results (a map of stepresults) or the final results, depending on if i bother to implement the latter
//Initially i considered leaving this as fixed on, but actually vanilla has a lot more states than i thought, wheras northstar only has like 10
#[derive(Debug, Clone, Copy, Default)]
pub enum CacheTrees {
    Results,
    Queries,
    All,
    #[default]
    Never,
}//Enum because i might make a couple more settings but like
//This isnt really intended for anyone but me to use, so im not that concerned about this actually being usable


#[cfg(test)]
mod messing_around {
    use std::path::PathBuf;

    use crate::{analysis_error::AnalysisError, results::AnalysisResult};

    use super::*;
    use preprocessor::{analyser, Preprocessor};
    #[derive(Debug, Clone)]
    struct Result1(String);
    impl AnalysisResult for Result1 {}
    impl From<String> for Result1 {
        fn from(value: String) -> Self {
            Result1(value)
        }
    }
    #[derive(Debug, Clone)]
    struct Result2(String);
    impl AnalysisResult for Result2 {}
    impl From<String> for Result2 {
        fn from(value: String) -> Self {
            Result2(value)
        }
    }

    struct FirstTestStep{}
    /*impl AnalysisStep<Result1> for FirstTestStep {
        fn analyse(&self, context: &AnalysisContext) -> Result<Result1, AnalysisError> {
            Ok(format!("First step for variant {} in analyser {:?}", context, context.analyser).into())
        }
    }*/
    struct SecondTestStep{}
    /*impl AnalysisStep<Result2> for SecondTestStep {
        fn analyse(&self, context: &AnalysisContext) -> Result<Result2, AnalysisError> {
            let old: Arc<Result1> = context.get_own_result()?;
            println!("Got old result: {}", old.0); 
            
            Ok(format!("Second step for variant {} in analyser {:?}", context, context.analyser).into())
        }
        fn depends_on(&self) -> Vec<TypeId> {
            vec![TypeId::of::<Result1>()] //This step depends on Result1, so it should run after it
        }
    }*/

    #[test]
    fn test_analyser() {
        
        let preprocessor = Preprocessor::new();
        

        let testfile_1 = PathBuf::from("testfile_1.vscript");
        let testfile_2 = PathBuf::from("testfile_2.vscript");

        let variant1 = VariantKey::test_variant(testfile_1.clone(), preprocessor.to_state_primitive("SERVER, MP"), 0, 0);
        let variant2 = VariantKey::test_variant(testfile_1, preprocessor.to_state_primitive("CLIENT, !MP"), 1, 0);
        let variant3 = VariantKey::test_variant(testfile_2, preprocessor.to_state_primitive("SERVER"), 2, 1);

        let mut analyser = Analyser {
            config: AnalysisConfig::default(),
            files: Arc::new(vec![]),
            variants: vec![variant1.clone(), variant2.clone(), variant3.clone()],
            steps: IndexMap::new(),
            prebuilt_trees: HashMap::new(),
            cached_trees: HashMap::new(),
            cached_queries: RwLock::new(HashMap::new()),
            errors: RwLock::new(vec![]),
        };
        /*analyser.insert_step(SecondTestStep {}, PreserveType::Never);
        analyser.insert_step(FirstTestStep {}, PreserveType::Before);*/
        //^These should be sorted into the right order
        let _ = analyser.run_steps();
        let results1 = analyser.get_results::<Result1>().unwrap();
        assert!(!results1.is_empty(), "Results1 should not be empty, got: {:?}", results1);
        println!("Results1: {:?}", results1);
        let step2 = analyser.get_step_info::<Result2>().unwrap();
        let results2 = &step2.results.read().unwrap().results;
        assert!(!results2.is_empty(), "Results2 should not be empty, got: {:?}", step2.errors());
        println!("Results2: {:?}", results2);
        println!("Step2 Errors: {:?}", step2.errors());
        
    }

    #[test]
    fn test_caching() {
        let preprocessor = Preprocessor::new();

        let testfile_1 = PathBuf::from("testfile_1.vscript");
        let testfile_2 = PathBuf::from("testfile_2.vscript");

        let variant1 = VariantKey::test_variant(testfile_1.clone(), preprocessor.to_state_primitive("SERVER, MP"), 0, 0);
        let variant2 = VariantKey::test_variant(testfile_1, preprocessor.to_state_primitive("CLIENT, !MP"), 1, 0);
        let variant3 = VariantKey::test_variant(testfile_2, preprocessor.to_state_primitive("SERVER"), 2, 1);

        let mut analyser = Analyser {
            config: AnalysisConfig {
                prebuild_trees: PrebuildTrees::After(1), //Prebuild trees after 1 variant
                cache_trees: CacheTrees::All,
            },
            files: Arc::new(vec![]),
            variants: vec![variant1.clone(), variant2.clone(), variant3.clone()],
            steps: IndexMap::new(),
            prebuilt_trees: HashMap::new(),
            cached_trees: HashMap::new(),
            cached_queries: RwLock::new(HashMap::new()),
            errors: RwLock::new(vec![]),
        };
    }
}
