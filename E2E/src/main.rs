use core::panic;
use std::{collections::HashMap, env, path::PathBuf, sync::{Arc, RwLock, Weak}};

use analysis_common::{spanning_search::TraversableMap, variable::VariableReference, CompiledState, modjson::{load_base, load_mod}};
use ASTAnalyser::{analyse_state, find_funcs, load_order::{identify_file_tree, identify_globals, File, FilePreAnalysis, ParseType}, single_file::{analyse, collect_errs, AnalysisState}, LogicError, Scope};
use common::{FileInfo, FileType};
use ASTParser::{ast::Element, error::Error};
use ConfigAnalyser::get_file_varaints;
use rayon::prelude::*;
mod ValidSets;
mod MemTests;
#[test]
fn artificial_noexternal(){
    let path = "../TestSets/ValidStructure/noexternal";
    let res = load_mod(PathBuf::from(path));
    let modfile = res.unwrap();
    //We should raise an error if the file is not found, that however is a language server problem not an analyser problem so should be handled in the LSP
    let files = modfile.scripts;

    //Preprocess all files
    let preproc = identify_globals(files);
    let tree = identify_file_tree(preproc);
    //analyse tree
    //TODO: Multithread
    for file in tree{
        println!("Analysing file: {:?}", file.load.name());
        let length = file.load.len();       
        for variant in file.variants.get_direct(){
            println!("Analysing variant: {:?}", variant.globalinfo.primitive.file.name());
            let scope = Scope::new((0, length));
            let steps = &variant.globalinfo.primitive.ast;
            find_funcs(scope.clone(), &steps);
            let state = Arc::new(RwLock::new(AnalysisState::new(variant.clone(), scope.clone())));
            analyse(state.clone(), &steps, variant.untyped);
            let errors = collect_errs(scope.clone());
            assert!(errors.is_empty(), "{} Errors found in file {:?} state {:?}: \n {:?}", errors.len(), file.load.name(), variant.globalinfo.primitive.context, errors);
        }
    }
}


#[test]
fn artificial_external(){
    let path = "../TestSets/ValidStructure/external";
    let res = load_mod(PathBuf::from(path));
    let modfile = res.unwrap();
    //We should raise an error if the file is not found, that however is a language server problem not an analyser problem so should be handled in the LSP
    let scripts = modfile.scripts;
    //Preprocess all files
    let preproc = identify_globals(scripts);
    let tree = identify_file_tree(preproc);
    //analyse tree
    //TODO: Multithread
    for file in tree{
        if file.parse_type == ParseType::PreAnalysis{
            continue;
        }
        println!("Analysing file: {:?}", file.load.name());
        let length = file.load.len();       
        for variant in file.variants.get_direct(){
            println!("Analysing variant: {:?}", variant.globalinfo.primitive.file.name());
            let scope = Scope::new((0, length));
            let steps = &variant.globalinfo.primitive.ast;
            find_funcs(scope.clone(), &steps);
            let state = Arc::new(RwLock::new(AnalysisState::new(variant.clone(), scope.clone())));
            analyse(state.clone(), &steps, variant.untyped);
            let errors = collect_errs(scope.clone());
            assert!(errors.is_empty(), "{} Errors found in file {:?} state {:?}: \n {:?}", errors.len(), file.load.name(), variant.globalinfo.primitive.context, errors);
            //Grab all variablereferences
        }
    }
}


#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[test]
pub fn northstar(){
    let path = "../../NorthstarMods-1.30.0";
    let natives = "../../NorthstarMods-1.30.0/.github/nativefuncs.json";
    let res = run_paths(&path.to_string(), Some(&natives.to_string()));
    for run in res {
        for variant in &run.outputs{
            let err = collect_errs(variant.clone());
            for err in &err{
                match err.value.as_ref(){
                    LogicError::SyntaxError(_) => {
                        panic!("Syntax error in file {:?} : \n {:?}", run.file.load.name(), err);
                    }
                    _ => {}
                }
            }
            //assert!(err.len() == 0, "Error in run: {:?} with error: {:?}", variant, err);
        }
    }
}



#[test]
pub fn MPack(){
    let path = "../TestSets/RealSets/MutatorPack";
    let natives = "../../NorthstarMods-1.30.0/.github/nativefuncs.json";
    let res = run_paths(&path.to_string(), Some(&natives.to_string()));
    for run in res {
        for variant in &run.outputs{
            let err = collect_errs(variant.clone());
            for err in &err{
                match err.value.as_ref(){
                    LogicError::SyntaxError(_) => {
                        panic!("Syntax error in file {:?} : \n {:?}", run.file.load.name(), err);
                    }
                    _ => {}
                }
            }
        }
    }
}


pub fn main(){
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("no provided path");
        return;
    }
    let path = &args[1];
    let natives = args.get(2);
    let hold_this = run_paths(path, natives);

}

pub fn run_paths(path: &String, natives: Option<&String>) -> Vec<Arc<RunData>>{


    let mut scripts = Vec::new();

    if let Some(natives) = natives {
        if let Ok(base) = load_base(PathBuf::from(natives)){
            scripts = base.scripts;
        } else {
            println!("Failed to load natives: {}", natives);

        }
    } else {
        println!("Natives disabled")
    }

    println!("Path: {:?}", path);
    if let Ok(modfile) = load_mod(PathBuf::from(path)){
        scripts = modfile.scripts;
    } else {
        //Get subfolders
        let path = PathBuf::from(path);
        for entry in path.read_dir().unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                if let Ok(modfile) = load_mod(path.clone()){
                    scripts.extend(modfile.scripts.clone());
                }
            }
        }

    }
    //Hacky to get "true" line count and condition list
    let mut condition_set = HashMap::from([("SERVER".to_string(), ()), ("CLIENT".to_string(), ()), ("UI".to_string(), ())]);//Stores (), just to prevent dupes
    let mut unique_conditions = HashMap::new();
    let mut size_preprocessed = 0;
    {
        for file in &scripts{
            let variants = get_file_varaints(file.clone());
            size_preprocessed += variants.iter().map(|x| x.text().len()).sum::<usize>();
            for (name, _) in variants.iter().flat_map(|x| x.state.0.clone()){
                condition_set.insert(name, ());
            }
            for variant in variants{
                if let Some(count) = unique_conditions.get_mut(&variant.state.identifier()){
                    *count += 1;
                } else {
                    unique_conditions.insert(variant.state.identifier(), 1);
                }
            }

        }
    }
    //We should raise an error if the file is not found, that however is a language server problem not an analyser problem so should be handled in the LSP
    //Preprocess all files
    
    let true_start = std::time::Instant::now();
    #[cfg(feature = "timed")]
    let parsing_time_start = std::time::Instant::now();
    let preproc = identify_globals(scripts);
    let tree = identify_file_tree(preproc);
    #[cfg(feature = "timed")]//Todo can i do this in one?
    let parsing_time = std::time::Instant::now().checked_duration_since(parsing_time_start).unwrap_or_default();
    #[cfg(feature = "timed")]
    let preprocessing_time = tree.iter().map(|x| x.load.get_preproc_time()).sum::<std::time::Duration>();
    #[cfg(feature = "timed")]
    let sq_parse_time = tree.iter().map(|x| x.load.get_sq_parse_time()).sum::<std::time::Duration>();
    //It should be noted, this is parrallelised, so it technically is not the real time taken
    
    //let mut file_stats = Vec::new();
    let mut global_vars: Vec<(Arc<_>, Vec<Arc<VariableReference>>)> = Vec::new();
    let mut size_real = 0;
    let mut variants_total = 0;
    let mut files_total = tree.len();
    let pre_analysis_count = tree.iter().filter(|x| x.parse_type == ParseType::PreAnalysis).count();
    files_total -= pre_analysis_count;
    let mut first = true;
    let iter = tree.into_par_iter();
    let analysis_time_start = std::time::Instant::now();
    let file_stats = iter.filter_map(|file| {

        if file.parse_type == ParseType::PreAnalysis{
            return None;
        }
        //println!("Analysing file: {:?}", file.load.name());
        let length = file.load.len();
        let variants = file.variants.get_direct();

        let variant_count = variants.len();
        let iter = variants.into_par_iter();
        #[cfg(feature = "detailed")]
        println!("Analysing file: {:?}", file.load.name());
        let results  = iter.map(|variant| {
            ////println!("Analysing variant: {:?}", variant.globalinfo.primitive.file.name());
            //let scope = Scope::new((0, length));
            //let steps = &variant.globalinfo.primitive.ast;
            //find_funcs(scope.clone(), &steps);
            ////(scope, variant.clone());
            ////let steps = &variant.globalinfo.primitive.ast;
            //let state = Arc::new(RwLock::new(AnalysisState::new(variant.clone(), scope.clone())));
            //analyse(state.clone(), &steps, variant.untyped);
            //scope
            analyse_state(variant.clone())
        }).collect::<Vec<_>>();
        //println!("Took from {:?} to {:?}", analysis_time_start, end);
        Some((file, results, length, variant_count.clone()))
    });
    let file_stats = file_stats.collect::<Vec<_>>();
    let longest_file = file_stats.iter().max_by(|a, b| a.1.len().cmp(&b.1.len())).cloned();
    let most_variants = file_stats.iter().max_by(|a, b| a.3.cmp(&b.3)).cloned();

    let end_analysis = std::time::Instant::now();
    let analysis_time = end_analysis.checked_duration_since(analysis_time_start).unwrap_or_default();
    let true_end = std::time::Instant::now();
    let mut runs    = Vec::new();
    let file_stats = file_stats.into_iter().map(|(file, results, file_size_real, file_variants_total)| {
        size_real += file_size_real;
        variants_total += file_variants_total;
        let mut undefined_vars: HashMap<String, Vec<Element<LogicError>>> = HashMap::new();
        let mut undefined_vars_conditional: HashMap<String, Vec<(Vec<CompiledState>, Element<LogicError>)>> = HashMap::new();
        let mut does_not_return = Vec::new();
        let mut syntax_errors = Vec::new();
        let mut syntax_warnings = Vec::new();

        let data = RunData{
            file: file.clone(),
            outputs: results.clone()
        };
        //This seems to be a result of specific runs (most likely empty files) taking < a nano, so this is used in place of .elapsed
        for scope in results{
            for err in collect_errs(scope.clone()){
                match err.value.as_ref(){
                    LogicError::UndefinedVariableError(name) => {
                        if let Some(arr) = undefined_vars.get_mut(name){
                            arr.push(err.clone());
                        } else {
                            undefined_vars.insert(name.clone(), vec![err.clone()]);
                        }
                    }
                    LogicError::UndefinedVariableErrorConditional(cond, name) => {
                        if let Some(arr) = undefined_vars_conditional.get_mut(name){
                            arr.push((cond.clone(), err.clone()));
                        } else {
                            undefined_vars_conditional.insert(name.clone(),  vec![(cond.clone(), err.clone())]);
                        }
                    }
                    LogicError::SyntaxError(err_int) => {
                        syntax_errors.push(err.clone());
                    }
                    LogicError::SyntaxWarning(err_int) => {
                        syntax_warnings.push(err.clone());
                    }
                    LogicError::DoesNotReturnError => {
                        does_not_return.push(err.clone());
                    }
                }
                //println!("{:?} at \n {} \n", err, file.load.text()[err.range.0 .. err.range.1].to_string());
            }
            for reference in scope.all_references(){
                if reference.is_target_global() && reference.target.file_path().is_some_and(|x| &x != file.load.path() ) {
                    if let Some(arr) = global_vars.iter_mut().find(|x| Arc::ptr_eq(&x.0, &reference.target)){
                        arr.1.push(reference.clone());
                    } else {
                        global_vars.push((reference.target.clone(), vec![reference.clone()]));
                    }
                }
            }
            //Grab all variablereferences
            //println!("references for scope: {:?}", scope.all_references())
        };

        runs.push(Arc::new(data));
        (file.load.name().clone(), RunResult{
            real_size: size_real,
            variants_total: variants_total,
            undefined_vars,
            undefined_vars_conditional,
            syntax_errors,
            does_not_return
        })
        //file_stats.push((file.load.name().clone(), results));
    }).collect::<Vec<_>>();
    //let results_iter = file_stats.into_par_iter();
    //let results = results_iter.map(|(name, result)| {
    //    let reduce = result.reduce(|| RunResult::new(), |mut a, b| {
    //        a.merge(&b);
    //        a
    //    });
    //    (name, reduce)
    //}).collect::<Vec<_>>();
    //let file_stats = results;

    //Give some stats
    println!("=== File Stats ===");
    let mut failed_vars = HashMap::new();
    for (name, file) in file_stats.iter(){
        #[cfg(feature = "detailed")]
        println!("\nFile {}: ", name);
        #[cfg(feature = "detailed")]
        println!("Undefined Variables ({}): ", file.undefined_vars.len());
        for (key, value) in &file.undefined_vars{
            //#[cfg(feature = "detailed")]
            //println!("{}: {:?}", key, value.iter().map(|x| x.range).collect::<Vec<_>>());
            if let Some(count) = failed_vars.get_mut(key){
                *count += value.len();
            } else {
                failed_vars.insert(key.clone(), value.len());
            }
        }
        #[cfg(feature = "detailed")]
        println!("Undefined Variables Conditional ({}): ", file.undefined_vars_conditional.len());
        for (key, value) in &file.undefined_vars_conditional{
            #[cfg(feature = "detailed")]
            println!("{}: {:?}", key, value);
            if let Some(count) = failed_vars.get_mut(key){
                *count += value.len();
            } else {
                failed_vars.insert(key.clone(), value.len());
            }
        }
        #[cfg(feature = "detailed")]
        println!("Syntax Errors ({}): ", file.syntax_errors.len());
        for err in &file.syntax_errors{
            #[cfg(feature = "detailed")]
            println!("{:?}", err);
        }
        #[cfg(feature = "detailed")]
        println!("Does Not Return Errors ({}): ", file.does_not_return.len());
        for err in &file.does_not_return{
            #[cfg(feature = "detailed")]
            println!("{:?}", err);
        }
    }
    print!("\nMost failed variable: ");
    failed_vars.iter().max_by(|a, b| a.1.cmp(b.1)).map(|(key, value)| {
        print!("{}: {}", key, value);
    });
    println!("\nMost failed variables: ");
    let vec = failed_vars.iter().collect::<Vec<_>>();
    let mut sorted = vec.clone();
    sorted.sort_by(|a, b| a.1.cmp(b.1));
    for (key, value) in sorted.iter().rev().take(10){
        println!("{}: {}", key, value);
    }
    println!("Most used global variable: ");
    let sorted = global_vars.sort_by(|a, b| a.1.len().cmp(&b.1.len()));
    for (key, value) in global_vars.iter().rev().take(10){
        println!("{}: {} ({})", key.ast().text_none_rec(), value.len(), key.file_path().unwrap().display());
    }
    #[cfg(feature = "timed")]
    {
        println!("=== Timings ===");
        println!("Total time: {:?}", true_end.checked_duration_since(true_start).unwrap_or_default());
        println!("Parsing time: {:?}", parsing_time);
        println!("\tPreprocessing time: {:?}", preprocessing_time);
        println!("\tRSquirrel time: {:?}", sq_parse_time);
        println!("\tNote: Parrallelised, Sum duration accross all threads (May therefore be larger than Parsing time)");
        println!("Analysis time: {:?}", analysis_time);
    }
    println!("=== Summary ===");
    print!("Files analysed: {} \n", files_total);
    print!("Variants analysed: {} \n", variants_total);
    println!("Real size: {} characters", size_real);
    println!("Parsed size: {} characters", size_preprocessed);
    if let Some((file, _, length, _)) = longest_file{
        println!("Longest file: {} ({})", file.load.name(), length);
    }
    if let Some((file, _, _, count)) = most_variants{
        println!("Most variants: {} ({})", file.load.name(), count);
    }
    let syntax_errors = file_stats.iter().map(|x| x.1.syntax_errors.len()).sum::<usize>();
    println!("Total syntax errors: {}", syntax_errors);//In most cases these are a problem on my end
    let undefined_vars = file_stats.iter().map(|x| x.1.undefined_vars.len()).sum::<usize>();
    let undefined_vars_conditional = file_stats.iter().map(|x| x.1.undefined_vars_conditional.len()).sum::<usize>();
    //let undefined_vars_conditional = 0;
    println!("Total undefined variables: {}", undefined_vars + undefined_vars_conditional);
    //Similar to basic binary, each condition is true of false
    //If we were to require that all be specified (IE, remove the idea of "potentially" valid)
    //We essentially just get 2^n combinations
    //However as the three VMs are mutually exclusive, we can just remove them from the equation and multiply by 3
    println!("All referenced conditions: {:?}", condition_set.keys().collect::<Vec<_>>());
    let possible_variants = 2u64.pow((condition_set.len() - 3) as u32);
    let possible_variants = possible_variants * 3;
    println!("Possible condition sets count: {}", possible_variants);
    println!("Possible parsed variants: {}", files_total as u64 * possible_variants);
    println!("Unique condition sets count: {}", unique_conditions.len());
    println!("Unique condition sets: {:?}", unique_conditions.iter().collect::<Vec<_>>());
    return runs;
}


#[derive(Debug, Clone)]
pub struct RunData{
    file: Arc<File>,
    outputs: Vec<Arc<Scope>>,
}


#[derive(Debug, Clone)]
pub struct RunResult{
    real_size: usize,
    variants_total: usize,
    undefined_vars: HashMap<String, Vec<Element<LogicError>>>,
    undefined_vars_conditional: HashMap<String, Vec<(Vec<CompiledState>, Element<LogicError>)>>,
    syntax_errors: Vec<Element<LogicError>>,
    does_not_return: Vec<Element<LogicError>>,
}// cargo run --features timed -- ..\..\northstar ..\..\northstar\.github\nativefuncs.json
impl RunResult{
    pub fn new() -> Self{
        RunResult{
            real_size: 0,
            variants_total: 0,
            undefined_vars: HashMap::new(),
            undefined_vars_conditional: HashMap::new(),
            syntax_errors: Vec::new(),
            does_not_return: Vec::new()
        }
    }
    pub fn merge(&mut self, other: &Self){
        for (key, value) in other.undefined_vars.iter(){
            if let Some(arr) = self.undefined_vars.get_mut(key){
                arr.extend(value.clone());
            } else {
                self.undefined_vars.insert(key.clone(), value.clone());
            }
        }
        for (key, value) in other.undefined_vars_conditional.iter(){
            if let Some(arr) = self.undefined_vars_conditional.get_mut(key){
                arr.extend(value.clone());
            } else {
                self.undefined_vars_conditional.insert(key.clone(), value.clone());
            }
        }
        self.syntax_errors.extend(other.syntax_errors.clone());
        self.does_not_return.extend(other.does_not_return.clone());
    }
}