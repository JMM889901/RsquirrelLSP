//Man literally every time i think i have an idea to simplify things i come up with something like this lmao
//Thank god nobody but me will read this code
use std::{any::{type_name, Any, TypeId}, collections::HashMap, fmt::{Debug, Display, Formatter}, hash::{DefaultHasher, Hasher}, path::PathBuf, sync::{Arc, RwLock}};

use common::{FileInfo, FileType};
use downcast_rs::{impl_downcast, DowncastSync};
use indexmap::IndexMap;
use rayon::iter::IntoParallelIterator;
use rayon::prelude::*;
use ConfigPredictor::state::SqCompilerState;

use crate::{comp_tree::{for_file, DowncastableData, VariantData}, state_resolver::CompiledState};

pub mod state_resolver;
pub mod comp_tree;

pub type AnalysisReturnType = Result<Arc<dyn AnalysisResultInternal>, AnalysisError>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnalysisStage {
    Parse,//Really should never need to add more than the original step to this, but fuck it i cba to make a special case
    GlobalIdent,//Think locating "Global" function declarations
    GlobalAnalysis,//Think validation function return/struct types
    FunctionIdent,//I have absolutely no idea what you would use this step for
    FunctionAnalysis,//The bulk of things 
}//It seems overkill to have 5 stages for like, 3 tasks


///Associated boolean is whether to search masked externals.json files, you should not ever need to do this
pub enum FileFilter {
    ///Files defined after the target
    After(bool),//I can't think of anything that would use this
    ///Files defined before the target (exclusive)
    Before(bool),
    ///everything
    All(bool),
    ///Target only, this should only be used for post-analysis operations and not within a step
    Target,
}
impl FileFilter{
    ///Files defined after the target
    pub const AFTER: Self = Self::After(false);
    ///Files defined before the target (exclusive)
    pub const BEFORE: Self = Self::Before(false);
    ///everything
    pub const ALL: Self = Self::All(false);
    ///Target only, this should only be used for post-analysis operations and not within a step as race conditions exist
    pub const TARGET: Self = Self::Target;
    pub fn should_search(&self, file_type: &FileType) -> bool {
        if file_type == &FileType::RSquirrel {
            return true;
        }
        self.get_shouldsearch()
    }
    fn get_shouldsearch(&self) -> bool {
        match self {
            FileFilter::After(should_search) => *should_search,
            FileFilter::Before(should_search) => *should_search,
            FileFilter::All(should_search) => *should_search,
            FileFilter::Target => true //If you are asking for yourself and yourself is an external file then go for it i guess
        }
    }
}

pub enum PrebuildTrees{
    Always,//I might add future options for doing this on some things
    Never,//Oh boy that was an awful idea, im not even going to have this here, its always, fight me
}
impl PrebuildTrees{
    pub fn prebuild(&self) -> bool {
        match self {
            PrebuildTrees::Always => true,
            PrebuildTrees::Never => false,
        }
    }

}
pub enum CacheTrees{
    Always,
    Never
}
impl CacheTrees{
    pub fn cache(&self) -> bool {
        match self {
            CacheTrees::Always => true,
            CacheTrees::Never => false,
        }
    }
}
//I wont lie i just wanted to do docstring because it seemed novel, theres no reason for the below to be the only thing with docstring

/// The degree to which results can be preserved from one run to another
/// 
/// Updating the mod.json or otherwise changing the file structure always causes a full re-parse
pub enum PreserveType{
    /// This should generally never be used, I can't think of why you would
    Always,
    /// This should re-run for the edited file and any files AFTER it (in the load order)
    Before,
    /// This should always re-run if any file was edited
    Never,
    /// This only needs to be re-run on the changed file
    Unchanged,

}

pub struct StepInfo {
    pub preserve: PreserveType,
    pub step : Box<dyn AnalysisStep>,
    pub return_type: TypeId,
    pub step_name: String,//Debugging
    pub return_type_name: String,//Debugging

}
use std::hash::Hash;
pub trait HasVariantID {
    fn get_state(&self) -> &CompiledState;
    fn get_file(&self) -> &FileInfo;

}


#[derive(PartialEq, Eq, Hash)]
pub struct SQDistinctVariantInternal {
    pub file: FileInfo,
    pub state: CompiledState,
    pub text: String,
    //...  TODO: Decide what to put here, I'd rather not just stick the text itself
}//Represents a variant of a file by the "new" ish definition, a file with a potentially (or probably) different text, previously a variant could be every possible combination of states
#[derive(Clone)]
pub struct SQDistinctVariant (Arc<SQDistinctVariantInternal>);
impl Debug for SQDistinctVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SQDistinctVariant {{ file: {:?}, state: {:?} }}", self.0.file.name(), self.0.state)
    }
}
impl HasVariantID for SQDistinctVariant {
    fn get_state(&self) -> &CompiledState {
        &self.0.state
    }
    fn get_file(&self) -> &FileInfo {
        &self.0.file
    }
}
impl SQDistinctVariant {
    pub fn new(file: FileInfo, state: CompiledState, text: String) -> Self {
        SQDistinctVariant(Arc::new(SQDistinctVariantInternal { file, state, text }))
    }
    pub fn text(&self) -> &String {
        &self.0.text
    }
    pub fn get_state_id(&self) -> u64 {
        let mut state_hasher = DefaultHasher::new();
        self.0.state.hash(&mut state_hasher);
        state_hasher.finish()
    }
}

pub struct AnalyserSettings {
    pub prebuild_trees: PrebuildTrees,
    pub cache_trees: CacheTrees,
}
impl AnalyserSettings{
    pub fn new() -> Self {
        Self::default()
    }
}
impl Default for AnalyserSettings {
    fn default() -> Self {
        Self { prebuild_trees: PrebuildTrees::Never, cache_trees: CacheTrees::Never }
    }
}
#[derive(Debug, Clone)]
pub enum AnalysisError {
    StepRequestError(String), 
    VariantRequestError(String),
    GenericError(String), //TODO: Fallback for weird stuff
    AnalysisError(String, (usize, usize)), //TODO: This should be a position
}
impl Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisError::StepRequestError(msg) => write!(f, "Step Request Error: {}", msg),
            AnalysisError::VariantRequestError(msg) => write!(f, "Variant Request Error: {}", msg),
            AnalysisError::AnalysisError(msg, (start, end)) => write!(f, "Analysis Error: {} at ({}, {})", msg, start, end),
            AnalysisError::GenericError(msg) => write!(f, "Generic Error: {}", msg),
        }
    }
}
#[cfg(feature = "timed")]
#[derive(Debug, Clone)]
pub struct EventTimeInfo{
    pub duration: std::time::Duration,
    pub occured: usize
}

pub struct Analyser {
    pub settings: AnalyserSettings,
    pub variants: Vec<(FileInfo, Vec<SQDistinctVariant>)>,
    pub steps: IndexMap<AnalysisStage, Vec<StepInfo>>,//These are seperate since this is ordered
    pub steps_data: HashMap<TypeId, RwLock<AnalysisStepResults>>,
    pub prebuilt_trees: HashMap<CompiledState, Vec<(FileInfo, VariantData<()>)>>,//I could build this per-varaint but like 90% of those would be duplicates anyways
    pub cached_trees: RwLock<HashMap<TypeId, HashMap<CompiledState, Vec<(FileInfo, Arc<dyn DowncastableData>)>>>>, //TODO: This is a bit of a hack, but it works for now
    ///Analysis-wide errors, typically missing steps/results
    pub overall_errors: RwLock<Vec<AnalysisError>>,
    #[cfg(feature = "timed")]
    pub arbitrary_times: RwLock<HashMap<String, EventTimeInfo>>, 
    
}
impl Debug for Analyser {//This is only really here because something later derives debug
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Analyser")
//            .field("settings", &self.settings)
            .field("variants", &self.variants)
            .finish()
            //TODO: I want to print the steps but for that i probably need to store the names
    }
}
impl Default for Analyser {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyser {
    #[cfg(feature = "timed")]//This does a lot of locks so slows things down quite a bit
    pub fn record_time(&self, name: String, duration: std::time::Duration) {
        let mut times = self.arbitrary_times.write().unwrap();
        if let Some(event) = times.get_mut(&name) {
            event.duration += duration;
            event.occured += 1;
        } else {
            times.insert(name, EventTimeInfo { duration, occured: 1 });
        }
    }
        #[cfg(feature = "timed")]//This does a lot of locks so slows things down quite a bit
    pub fn record_time_direct(&self, name: String, data: EventTimeInfo) {
        let mut times = self.arbitrary_times.write().unwrap();
        if let Some(event) = times.get_mut(&name) {
            event.duration += data.duration;
            event.occured += data.occured;
        } else {
            times.insert(name, data);
        }
    }
    pub fn new() -> Self {
        let settings = AnalyserSettings::new();
        Analyser::new_settings(settings)
    }
    pub fn new_settings(settings: AnalyserSettings) -> Self {
        let mut map = IndexMap::new();
        map.insert(AnalysisStage::Parse, Vec::new());
        map.insert(AnalysisStage::GlobalIdent, Vec::new());
        map.insert(AnalysisStage::GlobalAnalysis, Vec::new());
        map.insert(AnalysisStage::FunctionIdent, Vec::new());
        map.insert(AnalysisStage::FunctionAnalysis, Vec::new());
        Analyser {
            settings,
            variants: Vec::new(),
            steps: map,
            prebuilt_trees: HashMap::new(),
            cached_trees: RwLock::new(HashMap::new()),
            steps_data: HashMap::new(),
            overall_errors: RwLock::new(Vec::new()),
            #[cfg(feature = "timed")]
            arbitrary_times: RwLock::new(HashMap::new()),
        }
    }
    pub fn clean_all(&mut self) {
        self.steps_data.clear();
    }
    pub fn parralel_clean_single(&mut self, changed_files: &FileInfo, index: usize) {//This vec should be deduped, i will just trust that you do that
        //For simplicity sake just clean all prebuilt trees
        #[cfg(feature = "timed")]
        let start = std::time::Instant::now();
        self.prebuilt_trees.clear();
        self.cached_trees.write().unwrap().clear();
        #[cfg(feature = "timed")]
        self.record_time("Prebuilt/cached tree clear".to_string(), start.elapsed());
        for step in self.steps.values().flatten() {
            #[cfg(feature = "timed")]
            let start = std::time::Instant::now();
            if matches!(step.preserve, PreserveType::Never) {
                //let new_results = AnalysisStepResults::new();
                //self.steps_data.insert(step.return_type, RwLock::new(new_results)); //Cleaning step ASTAnalyser::ReferenceAnalysisStep": EventTimeInfo { duration: 19.509ms, occured: 1 }
                {
                let write = self.steps_data.get_mut(&step.return_type).unwrap();
                let mut write = write.write().unwrap();
                write.results.par_drain().count();//"Cleaning step ASTAnalyser::ReferenceAnalysisStep": EventTimeInfo { duration: 5.5542ms, occured: 1 }
                }
                #[cfg(feature = "timed")]
                {
                self.record_time(format!("Cleaning step {}", step.step_name), start.elapsed());
                }
                continue;
            }
            let data = self.steps_data.get(&step.return_type);
            if data.is_none() {
                let err = AnalysisError::StepRequestError(format!("No data of {:?} for step {:?}, was this step not run?",step.return_type_name, step.step_name));
                self.overall_errors.write().unwrap().push(err);
            }
            let mut results = data.unwrap().write().unwrap();
            let data = &results.results;
            let files = self.variants.par_iter().enumerate();

            let remain = files.filter_map(|(file_index, (file_info, variants))|  {
                let should_clean = match &step.preserve {
                    PreserveType::Unchanged => index == file_index,
                    PreserveType::Never => true,
                    PreserveType::Before => file_index >= index,
                    PreserveType::Always => false,
                    _ => false,//later
                };
                if should_clean {
                    return None;
                }

               // return (file_hash, variants).into();
                data.get_key_value(file_info)
            }).map(|(x, y)| (x.clone(), y.clone())).collect::<HashMap<_, _>>();
            results.results = remain;
            #[cfg(feature = "timed")]
            self.record_time(format!("Cleaning step {}", step.step_name), start.elapsed());
        };

    }


    pub fn get_distinct_variant<B: HasVariantID>(&self, id: &B) -> Option<&SQDistinctVariant> {
        for (file_info, variants) in &self.variants {
            //This is gross, and absolutely unnecessary good christ.
            if file_info.name() == id.get_file().name() && file_info.path() == id.get_file().path() {
                for variant in variants {
                    if &variant.0.state == id.get_state() {
                        return Some(variant);
                    }
                }
            }
        }
        None
    }
    pub fn run_step(&self, step: &Box<dyn AnalysisStep>, variant: &SQDistinctVariant) -> Arc<dyn AnalysisResultInternal> {
        let result = step.analyse(variant, self);
        //self.push_result(variant, result);
        result.unwrap()//TODO: handle these
    }
    pub fn prebuild_tree(&self, state: &CompiledState) -> Vec<(FileInfo, VariantData<()>)> {
        let mut files = Vec::new();
        for file in &self.variants {
            let file_info = file.0.clone();
            let data = for_file(&file.1, state);
            if data.is_some() {
                files.push((file_info, data.unwrap()));
            }
        }
        files
    }

    ///Parralel
    pub fn run_steps(&mut self) {
        if matches!(self.settings.prebuild_trees, PrebuildTrees::Always) {
            let files = self.variants.par_iter();
            let states = files.map(|(file_info, variants)| 
                variants.par_iter().map(|variant| {
                    (variant.get_state().clone(), Vec::new())
                }).collect::<HashMap<_, Vec<VariantData<()>>>>()
            ).flatten().collect::<HashMap<_, _>>();
            
            self.prebuilt_trees = states.into_par_iter().map(|(state, files)| {
                let prebuilt = self.prebuild_tree(&state);
                (state, prebuilt)
            }).collect::<HashMap<_, _>>();
            
        }

        if self.variants.is_empty() || self.steps.is_empty() {
            panic!("Cannot run steps, no variants or steps defined");//TODO: This needs to go in practice because i can just run this on an empty folder
        }
        for (stage, steps) in &self.steps {//We probably could do a bit more parralel but i dont want race conditions
            //println!("Running stage: {:?}", stage);
            for step in steps {
                let last_run = self.steps_data.get(&step.return_type).unwrap();
                let mut last_run = last_run.write().unwrap();    

                let iter = self.variants.par_iter().filter(|(x, _)| !last_run.has_file(x)).collect::<Vec<_>>().into_par_iter();//I'm not sure how rayon handles nested parralel
                //Probably not well but also like, its probably not an issue we arent handling a billion files
                let res = iter.filter_map(|(file_info, variants)| {
                    if !step.step.should_run(file_info.ftype()) {
                        return None;
                    }
                    let for_file = variants.into_par_iter().map(|variant| {
                        (variant.get_state().clone(), self.run_step(&step.step, variant))//TODO: Scary clone! (Not really, but i dont like cloning a bunch of strings)
                    }).collect::<Vec<(CompiledState, _)>>();
                    Some((file_info.clone(), for_file))
                });//.collect::<HashMap<_, _>>();
                //self.push_results(res);

                last_run.results.par_extend(res);
            }
        }
    }
    pub fn get_errors(&self, variant: &SQDistinctVariant) -> Vec<AnalysisError> {
        let mut errs = Vec::new();
        for (type_id, step_results) in &self.steps_data {
            let results = step_results.read().unwrap();
            //Register overall step errors
            errs.extend(results.errors.read().unwrap().clone());
            //Register errors for the actual run
            let errors = results.get(variant);
            if let Some(errors) = errors{
               //let errors = errors.as_ref().downcast_ref::<dyn AnalysisResultInternal>().unwrap();
               let errors = errors.errors(variant);
                for (start, end, error) in errors {
                    errs.push(AnalysisError::AnalysisError(error, (start, end)));
                }
            } else {
                errs.push(AnalysisError::VariantRequestError(format!("No results for variant {:?} in step {:?}", variant.get_file().name(), type_id)));
            }
        }
        errs
    }
    pub fn push_results(&self, results: HashMap<FileInfo, Vec<(CompiledState, Arc<dyn AnalysisResultInternal>)>> ) {
        let type_id = results.values().next().unwrap().first().unwrap().1.as_ref().type_id();//Jesus
        let current = self.steps_data.get(&type_id);
        let step_results = match current {
            Some(results) => results,
            None => {
                for (key, value) in &self.steps_data {
                    println!("Key: {:?}", key);
                }
                panic!("uh, i really hope you just fucked up the add step call, tried to get {:?}", type_id);
                //We can't (or shouldnt) make a new entry in this case because this is multithreaded and i dont want to deal with write locking
            }
        };
        let mut old_results = step_results.write().unwrap();
        for (file, results) in results {
            for (state, result) in results {
                old_results.insert_filestate(file.clone(), state, result);
            }
        }
    }

    pub fn add_step<R: AnalysisResultInternal + Sized>(&mut self, step: Box<dyn AnalysisStep>, preserve: PreserveType, stage: AnalysisStage) {
        let type_id = TypeId::of::<R>();
        let steps = self.steps.get_mut(&stage).unwrap();
        if let std::collections::hash_map::Entry::Vacant(e) = self.steps_data.entry(type_id) {//This is gross, but i genuinely do not want to deal with ownership bullshit
            e.insert(RwLock::new(AnalysisStepResults::new()));
        } else {
            panic!("Steps cannot return duplicates, hashmaps are fun");
        }
        let step_info = StepInfo {
            preserve,
            step_name: step.step_name(),
            step,
            return_type: type_id,
            return_type_name: std::any::type_name::<R>().to_string(),
        };
        steps.push(step_info);
    }
    pub fn push_result(&self, variant: &SQDistinctVariant, result: Arc<dyn AnalysisResultInternal>) {
        let type_id = result.type_id();
        let step_results = self.steps_data.get(&type_id);
        let step_results = match step_results {
            Some(results) => results,
            None => {
                panic!("Rusts ownership system makes me want to cry sometimes, trying to instert here does the funny");
            }
        };
        let mut results = step_results.write().unwrap();
        results.insert(variant, result);

    }
    pub fn get_prior_result<T: AnalysisResultInternal + Sized + 'static, B: HasVariantID>(&self, variant: &B) -> Option<Arc<T>> {
        let type_id = TypeId::of::<T>();
        if let Some(results) = self.steps_data.get(&type_id) {
            let results = results.read().unwrap();
            if let Some(result) = results.get(variant) {
                let as_t = result.clone();
                let as_t = as_t.downcast_arc::<T>().ok();
                return as_t;
            }
            panic!("Tried to get analysis result for a step {:?} that does not exist for this variant: {:?}", std::any::type_name::<T>(), variant.get_file().name());
        }
        panic!("Tried to get analysis result for a step that does not exist {:?}", type_id);
    }


    //This function should not be used raw, at least not during a step. I am specifically trying to abstract away the preprocessor for those
    pub fn get_results<T: AnalysisResultInternal + Sized + 'static, B: HasVariantID>(&self, from: &B, which: FileFilter, resolve: bool) -> Result<VariantData<Arc<T>>, AnalysisError> {
        if self.settings.cache_trees.cache(){
            return self.get_cached_results::<T, B>(from, which, resolve);
        }

        if self.settings.prebuild_trees.prebuild() {
                self.get_results_withtree(from, which, resolve)
        } else {
                self.get_results_notree(from, which, resolve)
        }
    }
    pub fn get_cached_results<T: AnalysisResultInternal + Sized + 'static, B: HasVariantID>(&self, from: &B, which: FileFilter, resolve: bool) -> Result<VariantData<Arc<T>>, AnalysisError> {
        let cached = self.cached_trees.read().unwrap();
        let type_id = TypeId::of::<T>();
        let state_cache = cached.get(&type_id).and_then(|state_cache| {
            state_cache.get(from.get_state())
        });
        
        let state_cache = match state_cache {
            None => {drop(cached);&self.cache_tree::<T, B>(from)},
            Some(state_cache) => state_cache,
        };
        let state_iter: Box<dyn Iterator<Item = &(FileInfo, Arc<dyn DowncastableData>)>> = match which {
            FileFilter::All(_) => Box::new(state_cache.iter()),
            FileFilter::After(_) => Box::new(state_cache.iter().skip_while(|(file_info, _)| file_info.name() != from.get_file().name())),
            FileFilter::Before(_) => Box::new(state_cache.iter().take_while(|(file_info, _)| file_info.name() != from.get_file().name())),
            FileFilter::Target => Box::new(state_cache.iter().filter(|(file_info, _)| file_info.name() == from.get_file().name())),
        };
        let merged = state_iter.map(|(_, data)| {
            data.clone().downcast_arc::<VariantData<Arc<T>>>()
                .map_err(|_| AnalysisError::StepRequestError(format!("Cached data for step {:?} is not of type {:?}", std::any::type_name::<T>(), data.type_id())))
                .unwrap()
        });
        let merged = merged.flat_map(|data| data.as_ref().clone().get_inner()).collect::<Vec<_>>();//TODO: Bad and cringe clone :(
        let merged = VariantData::Multi(merged);
        //let merged = VariantData::merge_unchecked(variants);
        if resolve {
            return Ok(merged.identify(from));
        }
        Ok(merged)
    }
    pub fn get_variant_results_filepath<T: AnalysisResultInternal + Sized + 'static>(&self, file: PathBuf) -> Result<VariantData<Arc<T>>, AnalysisError> {
        let variants = self.variants.iter().find(|(file_info, _)| file_info.path() == &file);
        if variants.is_none() {
            return Err(AnalysisError::VariantRequestError(format!("No variants for file {:?}", file)));
        }
        let step_results = self.steps_data.get(&TypeId::of::<T>());
        if step_results.is_none() {
            return Err(AnalysisError::StepRequestError(format!("No results for step {:?}", std::any::type_name::<T>())));
        }
        let step_results = step_results.unwrap();
        let step_results = step_results.read().unwrap();
        let variants = &variants.unwrap().1;
        let data = variants.iter().map(|variant| {
            let result = step_results.get(variant); 
            if result.is_none(){
                panic!("Tried to get results for file, but variants were not found for file {:?} in step {:?}", file, std::any::type_name::<T>());
            }
            let result = result.unwrap();
            let casted_result = result.clone().downcast_arc::<T>();
            if let Ok(casted_result) = casted_result {
                return (variant.clone(), casted_result);
            }
            let error_text = format!("Result for step {:?} is not of type {:?}", std::any::type_name::<T>(), result.type_id());
            panic!("You need to create actual error handling :3 {:?}", error_text);
        }).collect::<Vec<_>>();
        Ok(VariantData::Multi(data))
    }

    pub fn get_results_notree<T: AnalysisResultInternal + Sized + 'static, B: HasVariantID>(&self, from: &B, which: FileFilter, resolve: bool) -> Result<VariantData<Arc<T>>, AnalysisError> {
        let variants = self.steps_data.get(&TypeId::of::<T>());
        if variants.is_none() {
            return Err(AnalysisError::StepRequestError(format!("No results for step {:?}", std::any::type_name::<T>())));
        }
        let variants = variants.unwrap();
        let results = variants.read().unwrap();
        //Start is the first file, so on
        let files_iter: Box<dyn Iterator<Item = &(FileInfo, Vec<SQDistinctVariant>)>> = match which {//God thats stupid
            FileFilter::All(_) => Box::new(self.variants.iter()),
            FileFilter::After(_) => Box::new(self.variants.iter().skip_while(|(file_info, _)| file_info.name() != from.get_file().name())),
            FileFilter::Before(_) => Box::new(self.variants.iter().take_while(|(file_info, _)| file_info.name() != from.get_file().name())),
            FileFilter::Target => Box::new(self.variants.iter().filter(|(file_info, _)| file_info.name() == from.get_file().name())),
            _ => todo!()
        };

        let mut others = Vec::new();
        for (file_info, variants) in files_iter {
            if !which.should_search(file_info.ftype()) {
                continue;
            }
            let results_for_file = AnalysisStepResults::new();
            if let Some(output) = for_file(variants, from.get_state()) {
                others.push(output.map(
                    &mut|variant, result| {
                        let past_result = results.get(variant);
                        if past_result.is_none() {
                            let error_text = format!("No results of step {:?} for variant {:?} in file {:?}",std::any::type_name::<T>(), variant, file_info.name());
                            //results_for_file.errors.push(AnalysisError::VariantRequestError(error_text));
                            panic!("boo womp");
                        } 
                        let past_result = past_result.unwrap();
                        let casted_result = past_result.clone().downcast_arc::<T>();
                        //let casted_result = casted_result.unwrap();
                        if let Ok(casted_result) = casted_result{
                            return casted_result;
                        }
                        let error_text = format!("Result for step {:?} is not of type {:?}", std::any::type_name::<T>(), past_result.type_id());
                        panic!("You need to create actual error handling :3 {:?}", error_text);
                    }
                ));
            }
        }
        let new = VariantData::merge_unchecked(others);
        if resolve {
            return Ok(new.identify(from));
        }
        
        Ok(new)
    }
    pub fn cache_tree<T: AnalysisResultInternal + Sized + 'static, B: HasVariantID>(&self, from: &B) ->  Vec<(FileInfo, Arc<dyn DowncastableData>)> {
        let type_id = TypeId::of::<T>();
        let results = self.steps_data.get(&type_id);
        if results.is_none() {
            panic!("Tried to cache tree for step {:?} but no results were found", std::any::type_name::<T>());
        }
        let results = results.unwrap();
        let results = results.read().unwrap();
        let to_cache: Vec<(FileInfo, Arc<dyn DowncastableData>)>;
        if self.settings.prebuild_trees.prebuild(){
            let files_iter = self.prebuilt_trees.get(from.get_state()).unwrap().iter();
            to_cache = files_iter.map(|(file_info, variants)| {
                let result = variants.filter_map(
                    &mut|variant, _| {
                        let past_result = results.get(variant);
                        if past_result.is_none() {
                            let error_text = format!("No results of step {:?} for variant {:?} in file {:?}",std::any::type_name::<T>(), variant, file_info.name());
                            panic!("boo womp");
                        } 
                        let past_result = past_result.unwrap();
                        let downcasted = past_result.clone().downcast_arc::<T>().unwrap();
                        Some(downcasted)
                    }
                );
                (file_info.clone(), Arc::new(result) as Arc<dyn DowncastableData>)
            }).collect::<Vec<_>>();
        } else {
            let files_iter = self.variants.iter().filter_map(|(file, details)| {
                let data = for_file(details, from.get_state());
                data.as_ref()?;
                let data = data.unwrap();
                Some((file.clone(), data))
            });

            to_cache = files_iter.map(|(file_info, variants)| {
            let result = variants.filter_map(
                &mut|variant, _| {
                    let past_result = results.get(variant);
                    if past_result.is_none() {
                        let error_text = format!("No results of step {:?} for variant {:?} in file {:?}",std::any::type_name::<T>(), variant, file_info.name());
                        panic!("boo womp");
                    } 
                    let past_result = past_result.unwrap();
                    let downcasted = past_result.clone().downcast_arc::<T>().unwrap();
                    Some(downcasted)
                }
            );
            (file_info.clone(), Arc::new(result) as Arc<dyn DowncastableData>)
            }).collect::<Vec<_>>();
        }

        

        let mut cache = self.cached_trees.write().unwrap();
        let state = from.get_state().clone();
        let type_id = TypeId::of::<T>();
        cache.entry(type_id).or_insert_with(HashMap::new);
        let state_cache = cache.get_mut(&type_id).unwrap();
        state_cache.insert(state.clone(), to_cache);
        state_cache.get(&state).unwrap().clone()
    }
    //I don't really have the words for this honestly


    pub fn get_results_withtree<T: AnalysisResultInternal + Sized + 'static, B: HasVariantID>(&self, from: &B, which: FileFilter, resolve: bool) -> Result<VariantData<Arc<T>>, AnalysisError> {
        let variants = self.prebuilt_trees.get(from.get_state());
        if variants.is_none() {
            return Err(AnalysisError::VariantRequestError(format!("No prebuilt trees for state {:?} in step {:?}", from.get_state(), std::any::type_name::<T>())));
        }
        let variants = variants.unwrap();
        let results = self.steps_data.get(&TypeId::of::<T>());
        if results.is_none() {
            return Err(AnalysisError::StepRequestError(format!("No results for step {:?}", std::any::type_name::<T>())));
        }
        let results = results.unwrap();
        let results = results.read().unwrap();
        let files_iter: Box<dyn Iterator<Item = &(FileInfo, VariantData<()>)>> = match which {
            FileFilter::All(_) => Box::new(variants.iter()),
            FileFilter::After(_) => Box::new(variants.iter().skip_while(|(file_info, _)| file_info.name() != from.get_file().name())),
            FileFilter::Before(_) => Box::new(variants.iter().take_while(|(file_info, _)| file_info.name() != from.get_file().name())),
            FileFilter::Target => Box::new(variants.iter().filter(|(file_info, _)| file_info.name() == from.get_file().name())),
        };
        //let mut others = Vec::new();
        let mut new = Vec::new();
        #[cfg(feature = "timed")]
        let start = std::time::Instant::now();
        #[cfg(feature = "timed")]
        let mut mapping = EventTimeInfo {
            duration: std::time::Duration::ZERO,
            occured: 0
        };
        #[cfg(feature = "timed")]
        let mut collecting = EventTimeInfo {
            duration: std::time::Duration::ZERO,
            occured: 0
        };
        for (file_info, variants) in files_iter {
            if !which.should_search(file_info.ftype()) {
                continue;
            }
            #[cfg(feature = "timed")]
            let start = std::time::Instant::now();
            //let file_results = results.results.get(file_info).unwrap();//TODO: Scary?
            let result = variants.filter_map(
                &mut|variant, _| {
                    //let past_result = file_results.iter().find(|(state, thing)| state == variant.get_state());
                    let past_result = results.get(variant);
                    if past_result.is_none() {
                        let error_text = format!("No results of step {:?} for variant {:?} in file {:?}",std::any::type_name::<T>(), variant, file_info.name());
                        panic!("boo womp");
                    } 
                    let past_result = past_result.unwrap();
                    let casted_result = past_result.clone().downcast_arc::<T>();
                    //let casted_result = casted_result.unwrap();
                    if let Ok(casted_result) = casted_result{
                        return Some(casted_result);
                    }
                    let error_text = format!("Result for step {:?} is not of type {:?}", std::any::type_name::<T>(), past_result.type_id());
                    panic!("You need to create actual error handling :3 {:?}", error_text);
                }
            );
            #[cfg(feature = "timed")]
            {
            mapping.duration += start.elapsed();
            mapping.occured += 1;
            }
            //let result = result.collect::<Vec<_>>();
            #[cfg(feature = "timed")]
            let start = std::time::Instant::now();
            new.extend(result.get_inner());
            #[cfg(feature = "timed")]
            {
            collecting.duration += start.elapsed();
            collecting.occured += 1;
            }
            //others.push(result);
        }
        #[cfg(feature = "timed")]
        {
        self.record_time(format!("Building tree from prebuilt-tree: {}", std::any::type_name::<T>()), start.elapsed());
        self.record_time_direct(format!("Mapping prebuilt-tree: {}", std::any::type_name::<T>()), mapping);
        self.record_time_direct(format!("Collecting prebuilt-tree: {}", std::any::type_name::<T>()), collecting);
        }
        //let new = VariantData::merge_unchecked(others);
        let new = VariantData::Multi(new);
        if resolve {
            return Ok(new.identify(from));
        }
        Ok(new)
        
    }

    //TODO This is too greedy for most purposes, even with prebuilt trees its still ~30ms across all of reference analysis
    //Reference analysis should really just be first-satisifies, i suspect that would be faster (even though it would be calling Q-MK many more times)
    //^TODO: Look into cacheable Q-MK, random thought but presumably should be entirely possible.
    pub fn query_step<T: AnalysisResultInternal + Sized + 'static, R, B: HasVariantID>(&self, from: &B, which: FileFilter, f: &mut impl FnMut(&SQDistinctVariant, &Arc<T>) -> Option<R>) -> Result<VariantData<R>, AnalysisError> {
        //TODO: These are unnecessarily slow
        //As in, the query to get a global function adds 85ms to an otherwise 3ms step (although it queries multiple times) <- down to 30ms, but it was around 15 before switching to this model
        //Difference being this grabs the entire pre-built tree (or builds a new one) and filters it, previously it gradually added files until it got a complete context. 
        //I dont really like doing this for the time loss, but technically the old way had a much worse worst-case time
        #[cfg(feature = "timed")]
        let start = std::time::Instant::now();
        let from_step = self.get_results(from, which, false)?;
        #[cfg(feature = "timed")]
        self.record_time(format!("Querying: {} (Get Tree)", std::any::type_name::<T>()), start.elapsed());
        #[cfg(feature = "timed")]
        let start = std::time::Instant::now();
        let others = from_step.filter_map(f);
        #[cfg(feature = "timed")]
        self.record_time(format!("Querying: {} (filter)", std::any::type_name::<T>()), start.elapsed());
        #[cfg(feature = "timed")]
        let start = std::time::Instant::now();
        let others = others.identify(from);
        #[cfg(feature = "timed")]
        self.record_time(format!("Querying: {} (identify)", std::any::type_name::<T>()), start.elapsed());
        //let others = others.collect::<Vec<_>>();

        Ok(others)
    }
    //TODO: Nicer error handling (maybe with proper position stuff, but really i should leave that to steps themselves) 
}

pub struct AnalysisStepResults {//God, this is gross
    //pub results: HashMap<VariantID, Arc<dyn AnalysisResultInternal>>,
    pub results: HashMap<FileInfo, Vec<(CompiledState, Arc<dyn AnalysisResultInternal>)>>,//This is something of a test, it used to be a HashMap<HashMap> but theres only 2-3 variants per file so this should be faster?
    //Why on gods green earth did i not just say fuck it and use an enum
    ///Represents step-wide errors that (ideally) are not associated with a given file/variants
    pub errors: RwLock<Vec<AnalysisError>>
}
impl AnalysisStepResults {
    fn new() -> Self{
        Self { 
            results: HashMap::new(),
            errors: RwLock::new(Vec::new()),
        }
    }
    fn get(&self, key: &dyn HasVariantID) -> Option<&Arc<dyn AnalysisResultInternal>> {
        self.results.get(key.get_file()).and_then(|x| x.iter().find(|(x, y)| x == key.get_state()).map(|(x, y)| y))
    }
    fn insert(&mut self, key: &dyn HasVariantID, value: Arc<dyn AnalysisResultInternal>) {
        let file = key.get_file().clone();
        let inner = self.results.entry(file).or_default();
        inner.push((key.get_state().clone(), value));
    }
    fn insert_filestate(&mut self, file: FileInfo, state: CompiledState, value: Arc<dyn AnalysisResultInternal>){
        let inner = self.results.entry(file).or_default();
        inner.push((state, value));
    }
    fn has_file(&self, file: &FileInfo) -> bool {

        self.results.contains_key(file)
    }
    fn remove(&mut self, key: &dyn HasVariantID) -> Option<Arc<dyn AnalysisResultInternal>> {
        self.results.get_mut(key.get_file()).and_then(|x| {
            let index = x.iter().position(|(state, _)| state == key.get_state());
            if let Some(index) = index {
                let (_, result) = x.remove(index);
                return Some(result);
            }
            None
        })
    }
    fn remove_file(&mut self, key: &dyn HasVariantID) -> Option<Vec<(CompiledState, Arc<dyn AnalysisResultInternal>)>> {
        self.results.remove(key.get_file())
    }
}
//So truthfully, I'm not sure how this "works" as such
pub trait AnalysisResultInternal : DowncastSync + Sync + Send + 'static {
    fn errors(&self, context: &SQDistinctVariant) -> Vec<(usize, usize, String)>;
    fn as_any(&self) -> &dyn Any;
    fn result_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}
impl Debug for dyn AnalysisResultInternal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnalysisResultInternal: {}", type_name::<Self>())
    }
}
impl_downcast!(sync AnalysisResultInternal);

pub trait AnalysisResult : AnalysisResultInternal + Sized {
    fn get_errors(&self, context: &SQDistinctVariant) -> Vec<(usize, usize, String)> {
        vec![]
    }
}
impl<T: Sized> AnalysisResultInternal for T where T: AnalysisResult {
    fn errors(&self, context: &SQDistinctVariant) -> Vec<(usize, usize, String)> {
        self.get_errors(context)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }

}

/// Steps should be stateless, or at least should reset the state when analyse is called
/// Steps may get the results of prior runs, but should not get the results of its own run for other files, or future runs
pub trait AnalysisStep : Sync + Send {
    fn analyse(&self, variant: &SQDistinctVariant, analyser: &Analyser) -> Result<Arc<dyn AnalysisResultInternal>, AnalysisError>;
    fn step_name(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }
    fn should_run(&self, file_type: &FileType) -> bool {
        file_type == &FileType::RSquirrel
    }
}

#[derive(Debug, Clone)]
pub struct AnalysisResultExample {
    pub some_text: String,
    pub some_number: u32,
}
impl AnalysisResult for AnalysisResultExample {}

#[derive(Default)]
pub struct AnalysisStepExample {
    pub name: String,
}
impl AnalysisStep for AnalysisStepExample {
    fn analyse(&self, variant: &SQDistinctVariant, analyser: &Analyser) -> Result<Arc<dyn AnalysisResultInternal>, AnalysisError> {
        // Example analysis logic
        println!("Running analysis step: {}", self.name);
        let result: AnalysisResultExample = AnalysisResultExample {
            some_text: format!("Analysis of {} with step {}", variant.0.file.name(), self.name),
            some_number: 42,
        };
        Ok(Arc::new(result))
    }
}

pub struct AnalysisStepSuccessorExample {
    pub name: String,
}
impl AnalysisStep for AnalysisStepSuccessorExample {
    fn analyse(&self, variant: &SQDistinctVariant, analyser: &Analyser) -> Result<Arc<dyn AnalysisResultInternal>, AnalysisError> {
        println!("Running successor analysis step: {}", self.name);
        let past_result: Option<Arc<AnalysisResultExample>> = analyser.get_prior_result(variant);
        let past_results: VariantData<Arc<AnalysisResultExample>> = analyser.get_results(variant, FileFilter::AFTER, true)?;
        println!("Past results: {:?}", past_results);
        let past_result = past_result.unwrap();
        let result: AnalysisReturnExampleElectricBoogaloo = AnalysisReturnExampleElectricBoogaloo {
            some_text: format!("Successor analysis of {} with step {}", variant.0.file.name(), self.name),
            some_number: past_result.some_number + 1, // Example logic using previous result
        };
        Ok(Arc::new(result))
    }
}
pub struct AnalysisReturnExampleElectricBoogaloo {
    pub some_text: String,
    pub some_number: u32,
}


impl AnalysisResult for AnalysisReturnExampleElectricBoogaloo {}
#[test]
fn test_analyser() {
    let mut analyser = Analyser::new();
    let step1 = Box::new(AnalysisStepExample { name: "Step 1".to_string() });
    let step2 = Box::new(AnalysisStepSuccessorExample { name: "Step 2".to_string() });

    analyser.add_step::<AnalysisResultExample>(step1, PreserveType::Never, AnalysisStage::GlobalIdent);
    analyser.add_step::<AnalysisReturnExampleElectricBoogaloo>(step2, PreserveType::Never, AnalysisStage::GlobalAnalysis);

    let file_info = FileInfo::new("test_file".to_string(), std::path::PathBuf::from("test_path"), "MP".to_string(), common::FileType::RSquirrel);
    file_info.set_text("some dummy test text".to_string());
    let variant = SQDistinctVariant::new(
        file_info.clone(),
        CompiledState::from(SqCompilerState::one("Test".to_string(), true)), // Assuming a default state for testing
         "some dummy test text".to_string(),
    );
    analyser.variants.push((file_info.clone(), vec![variant.clone()]));

    analyser.run_steps();

    let mut extract_step1_result = |when: &SQDistinctVariant, data: &Arc<AnalysisResultExample>| -> Option<Arc<String>> {
        return Some(Arc::new(data.some_text.clone()));
    };
    let step1_text = analyser.query_step(&variant, FileFilter::ALL, &mut extract_step1_result).unwrap();
    step1_text.for_each(|variant, result| {
            println!("Step 1 Result for {:?}: {}", variant.0.file.name(), result);
        });

    // Check results
    let result: Arc<AnalysisResultExample> = analyser.get_prior_result(&variant).unwrap();
    assert_eq!(result.some_text, "Analysis of test_file with step Step 1");
}

pub struct Test{
    stuff: Vec<Arc<dyn AnalysisResultInternal>>
}
impl Test {
    fn get<T: AnalysisResultInternal + 'static>(&self) -> Option<Arc<T>> {
        for item in &self.stuff {
            if item.result_id() == TypeId::of::<T>() {
                return item.clone().downcast_arc::<T>().ok();
            }
        }
        None
    }
}
#[cfg(test)]
mod guh {
    use super::*;

    #[test]
    fn testthingy(){
        let data = get_data();
        let downcasted =data.into_cast::<AnalysisResultExample>();
        let inner = downcasted.get_inner();
        assert_eq!(inner.len(), 2);
        assert_eq!(inner[0].1.some_text, "Hello from file 1");
        assert_eq!(inner[1].1.some_text, "Hello from file 2");
    }

    fn get_data() -> VariantData<Arc<dyn AnalysisResultInternal>> {
        let data = vec![
            (SQDistinctVariant::new(
                FileInfo::new("file1".to_string(), PathBuf::from("path/to/file1"), "MP".to_string(), common::FileType::RSquirrel),
                CompiledState::from(SqCompilerState::one("Test".to_string(), true)),
                "Content of file 1".to_string(),
            ), Arc::new(AnalysisResultExample {
                some_text: "Hello from file 1".to_string(),
                some_number: 42,
            }) as Arc<dyn AnalysisResultInternal>),
            (SQDistinctVariant::new(
                FileInfo::new("file2".to_string(), PathBuf::from("path/to/file2"), "MP".to_string(), common::FileType::RSquirrel),
                CompiledState::from(SqCompilerState::one("Test".to_string(), true)),
                "Content of file 2".to_string(),
            ), Arc::new(AnalysisResultExample {
                some_text: "Hello from file 2".to_string(),
                some_number: 100,
            }) as Arc<dyn AnalysisResultInternal>),
        ];
        return VariantData::Multi(data);
    }

    fn get_things() -> Vec<Arc<dyn AnalysisResultInternal>> {
        let data =vec![Arc::new(AnalysisResultExample {
            some_text: "Hello".to_string(),
            some_number: 42,
        }) as Arc<dyn AnalysisResultInternal>,
        Arc::new(AnalysisResultExample {
            some_text: "World".to_string(),
            some_number: 100,
        }) as Arc<dyn AnalysisResultInternal>,
        ];
        return data;
    }

}