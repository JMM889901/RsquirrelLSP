use core::panic;
use std::cell::RefCell;
use std::clone;
use std::collections::HashMap;
use std::fmt::{format, Debug};
use std::hash::Hash;
use std::io::Write;
use std::sync::{Arc, RwLock, Weak};

use analysis_runner::comp_tree::{VariantData};
use analysis_runner::state_resolver::CompiledState;
use analysis_runner::{Analyser, AnalysisResult, FileFilter, HasVariantID, SQDistinctVariant};
use common::FileInfo;
use load_order::{FilePreAnalysis};
use single_file::{analyse, AnalysisState};
use ASTParser::error::Error;
use analysis_common::spanning_search::{SpanningSearch, Traversable, TraversableMap};
use analysis_common::variable::{Variable, VariableReference, VariableSearch};
use ConfigAnalyser::get_file_varaints;
use ASTParser::ast::{Element, Type, AST};
use ASTParser::grammar::squirrel_ast;
use ASTParser::{ASTParseResult, RunPrimitiveInfo, SquirrelParse};
use rayon::prelude::*;
use TokenIdentifier::{GlobalSearchStep, Globals};

pub mod load_order;
pub mod single_file;

pub struct ReferenceAnalysisStep{}
impl analysis_runner::AnalysisStep for ReferenceAnalysisStep {
    fn analyse(&self, run: &analysis_runner::SQDistinctVariant, analyser: &analysis_runner::Analyser) -> analysis_runner::AnalysisReturnType {
        let asts: Arc<ASTParseResult> = analyser.get_prior_result(run).unwrap();
        let globals: Arc<Globals> = analyser.get_prior_result(run).unwrap();

        let scope = Scope::new((0, asts.run.file.len()));
        let steps = &asts.run.ast;
        find_funcs(scope.clone(), &steps);
        let state = Arc::new(RwLock::new(AnalysisState::new(asts.run.clone(), globals, scope.clone())));
        analyse(analyser, state.clone(), &steps, false);//TODO: Untyped identification
        return Ok(Arc::new(ReferenceAnalysisResult{
            scope: scope.clone(),
        }));
        //return scope.clone();
    }
}

pub struct ReferenceAnalysisResult{
    pub scope: Arc<Scope>,
}
impl AnalysisResult for ReferenceAnalysisResult {
    fn get_errors(&self, context: &SQDistinctVariant) -> Vec<(usize, usize, String)> {
        let mut errors = Vec::new();
        for error in self.scope.errors.read().unwrap().iter(){
            let err = error.value.render(&context.get_state());
            errors.push((error.range.0, error.range.1, err));
        }
        let children = self.scope.all_children_rec();
        for child in children.iter(){
            let child_errors = child.errors.read().unwrap();
            for error in child_errors.iter(){
                let err = error.value.render(&context.get_state());
                errors.push((error.range.0, error.range.1, err));
            }
        }
        return errors;
    }
}

pub struct Scope{//TODO: make most of these Traversable
    pub range: (usize, usize),
    pub vars: TraversableMap<String, Variable>,
    pub references: Traversable<VariableReference>,
    pub types: TraversableMap<String, TypeDef>,
    pub children: Traversable<Scope>,
    pub errors: RwLock<Vec<Element<LogicError>>>,
    pub parent: Option<Weak<Scope>>,//also used to refer to previous file in load order
    pub global: TraversableMap<String, GlobalBridge>,
    //Global functions are stored here, as they are not in the scope of the file
    //It is far too late in the project to be asking this
    //But why the fuck are globals even stored here
    pub has_return: RwLock<bool>,
}
impl Debug for Scope{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scope")
            .field("range", &self.range)
            .field("vars", &self.vars)
            .field("types", &self.types)
            .field("children", &self.children)
            .field("errors", &self.errors)
            .field("global", &self.global)
            .field("has_return", &self.has_return)
            .finish()
    }
}

impl SpanningSearch<Scope> for Scope{
    fn range(&self) -> (usize, usize) {
        return self.range
    }
}

impl Scope{
    pub fn new(range: (usize, usize)) -> Arc<Self>{
        Arc::new(Scope{
            range,
            vars: TraversableMap::new(),
            types: TraversableMap::new(),
            children: Traversable::new(),
            references: Traversable::new(),
            errors: RwLock::new(Vec::new()),
            parent: None,
            global: TraversableMap::new(),
            has_return: RwLock::new(false),
        })
    }
    pub fn new_parent(range: (usize, usize), parent: Weak<Scope>) -> Arc<Self>{
        Arc::new(Scope{
            range,
            vars: TraversableMap::new(),
            types: TraversableMap::new(),
            children: Traversable::new(),
            references: Traversable::new(),
            errors: RwLock::new(Vec::new()),
            parent: Some(parent),
            global: TraversableMap::new(),
            has_return: RwLock::new(false),
        })
    }
    pub fn add_child(parent: Arc<Self>, range: (usize, usize)) -> Arc<Self>{
        let child = Scope::new_parent(range, Arc::downgrade(&parent));
        parent.children.add_arc(child.clone());       
        return child;
    }
    pub fn find_declaration(&self, pos: usize) -> Vec<Arc<VariableReference>>{
        if pos + 1 < self.range.0 || pos > self.range.1 + 1{
            panic!("Position out of range: {} not in {:?}", pos, self.range);
            return vec![];
        }
        //Check if its in a child
        let res: Vec<Arc<VariableReference>> = self.references.get_pos_source(pos, 2);
        if res.len() > 0{
            return res;
        }
        for child in self.children.get_pos(pos, 2){
            let res = child.find_declaration(pos);
            if res.len() > 0{
                return res;
            }//I dont like that this performs excess searches, but somethings up
        }
        return vec![];
    }
    pub fn find_uses(&self, pos: usize) -> Vec<Arc<VariableReference>>{
        //Check if we are hovering over a global definition
        let global = self.global.get_pos(pos, 2);
        let mut res = Vec::new();
        if global.len() > 0{//Global shoudl be a single element, but just in case
            for GlobalBridge{ func, global } in global.iter().map(|x| x.as_ref()){
                //Grab all references to the base function
                let func_refs = func.get_valid_references();
                res.extend(func_refs.iter().map(|x| x.clone()));
                //Grab all references to the global function
                let glob_refs = global.get_valid_references();
                res.extend(glob_refs.iter().map(|x| x.clone()));
            }
            if res.len() > 0{
                return res;
            }
            //This is kinda sketchy, I should phase out GlobalBridge entirely now that
            //Token identifier correctly uses variables
        }


        //let mut res: Vec<Arc<VariableReference>> = self.references.get_pos_target(pos, 2);
        //Yeah so i forgot you can like, reference something in another scope, because you know, functions exist

        let res = self.vars.get_pos(pos, 2);
        let mut res = res.iter().map(|x| x.get_valid_references()).flatten().collect::<Vec<_>>();
        //TODO: Nesting the search is expensive since logically if we find something here there almost certainly wont be anything in the children
        if res.len() > 0{
            return res;
        }
        for child in self.children.get_pos(pos, 2) {
            res.extend(child.find_uses(pos));
            if res.len() > 0{
                return res;
            }//I dont like that this performs excess searches, but somethings up
        }
        if res.len() > 0{
            return res;
        }
        //Shits fucked bud
        //let all_refs = self.all_references();
        //for ref_ in all_refs.iter(){
        //    if ref_.target.range_passes(pos, 2){
        //        res.push(ref_.clone());
        //    }
        //}
        return res;

    }
    pub fn all_references(&self) -> Vec<Arc<VariableReference>>{
        let mut references = Vec::new();
        for child in self.children.get(){
            references.extend(child.all_references());
        }
        references.extend(self.references.get());
        return references;
    }

    pub fn all_children_rec(&self) -> Vec<Arc<Scope>>{
        let mut children = Vec::new();
        for child in self.children.get(){
            children.push(child.clone());
            children.extend(child.all_children_rec());
        }
        return children;
    }

    //pub fn find_declaration_slow(&self, pos: usize) -> Vec<Element<AST>>{
    //    //Its slow but i am more confident in it
    //    let all_references = self.all_references();
    //    all_references.iter().filter(|x| x.source.range.0 <= pos + 1 && x.source.range.1 + 1 >= pos).map(|x| x.target.ast().clone()).collect()
    //}
    
    pub fn debuginfo(&self) -> String{
        //Get all references
        //let references = format!("References: {}", self.all_references().iter().map(|x| x.text_nice()).collect::<String>());
        let children = self.children.get();
        let mut references = String::new();
        for child in children{
            references.push_str(&format!("from {} to {} ", child.range.0, child.range.1));
            for ref_ in child.references.get(){
                references.push_str(&format!("\n{:?} ", ref_.text_nice()));
            }
        }
        //Get all children ranges
        let mut children_ranges = String::new();
        for child in self.children.get(){
            children_ranges.push_str(&format!("{:?} ", child.range));
        }
        //Get all vars
        let mut vars = String::new();
        for (_, var) in self.vars.get().iter(){
            vars.push_str(&format!("{:?} ", var.ast().range));
        }

        return format!("Scope: {:?} \n Vars: {} \n Children: {} \n References: {}", self.range, vars, children_ranges, references);
    }
}
#[cfg(test)]
mod scope_tests;

#[derive(Debug, Clone)]
pub enum LogicError{
    UndefinedVariableError(String),
    UndefinedVariableErrorConditional(Vec<CompiledState>, String),
    DoesNotReturnError,
    SyntaxError(Error),
    SyntaxWarning(Error),
}

impl LogicError{
    pub fn render(&self, run: &CompiledState) -> String {
        let mut text = format!("Error in {}\n", run.string_out_simple());
        match self{
            LogicError::UndefinedVariableError(name) => {
                text.push_str(&format!("Undefined variable: {}", name));
            }
            LogicError::UndefinedVariableErrorConditional(cond, name) => {
                let cond_text = cond.iter().map(|x| x.string_out_simple()).collect::<Vec<_>>().join(" OR ");
                //TODO: bit dumb init
                let cond_text = cond_text.replace("&", " && ");
                text.push_str(&format!("Variable {} may not be defined, try specify one of {}", name, cond_text ));
            }
            LogicError::SyntaxError(err_int) => {
                text.push_str(&format!("{:?}", err_int));
            }
            LogicError::SyntaxWarning(err_int) => {
                text.push_str(&format!("{:?}", err_int));
            }
            LogicError::DoesNotReturnError => {
                text.push_str(&format!("Function does not return"));
            }
        }
        return text;
    }
}

#[derive(Debug)]
pub struct GlobalBridge{
    func: Arc<Variable>,//We cant exactly globalise another files function
    global: Arc<Variable>,
}//Globals are registered elsewhere, this allows me to map the abstract "global" accross files to the specific function in the file

impl SpanningSearch<GlobalBridge> for GlobalBridge{
    fn range(&self) -> (usize, usize) {
        return self.global.ast().range;
    }
    fn range_passes(&self, pos: usize, leeway: usize) -> bool {
        //Pass if either the global declaration or the function passes
        let range = self.global.ast().range;
        if range.0 <= pos + leeway && range.1 + leeway >= pos{
            return true
        }
        if let AST::Function { name, args, returns, actions } = self.func.ast().value.as_ref(){
            let range = name.range;
            if range.0 <= pos + leeway && range.1 + leeway >= pos{
                return true
            }
        } else if range.0 <= pos + leeway && range.1 + leeway >= pos{
            return true//I think externals from json files hit this
        }
        return false
    }
}

#[derive(Debug)]
pub struct TypeDef{
    pub ast: Element<AST>,//Bad and scary deep copy
    pub name: String,//Not handling types properly yet, just a "does it exist"
}
impl SpanningSearch<TypeDef> for TypeDef{
    fn range(&self) -> (usize, usize) {
        return self.ast.range;
    }
}



pub fn generate_asts(file: FileInfo) -> Vec<RunPrimitiveInfo> {
    #[cfg(feature = "timed")]
    let start = std::time::Instant::now();
    let variants = get_file_varaints(file.clone());
    #[cfg(feature = "timed")]
    {
        let duration = std::time::Instant::now().checked_duration_since(start).unwrap_or_default();
        file.set_preproc_time(duration);
    }
    #[cfg(feature = "timed")]
    let start = std::time::Instant::now();
    //for variant in variants{
    let asts = variants.into_par_iter().map(|variant| {
        let parse_data = SquirrelParse::empty();
        let offset = RefCell::new(0);
        //println!("parsing variant: {:?}", variant.state.0);
        #[cfg(debug_assertions)]
        let parse = squirrel_ast::file_scope_dbg(&variant, &offset, &parse_data);
        #[cfg(not(debug_assertions))]
        let parse = squirrel_ast::file_scope_rls(&variant, &offset, &parse_data);
        let mut parse = parse.unwrap();
        let errs = parse_data.errs.read().unwrap();
        for err in errs.iter(){
            let ast = AST::Error(*err.value.clone());
            let elem = Element::new(ast, err.range.clone());
            parse.push(elem);
        }

        //asts.push((variant.state.0, parse.unwrap())); //BAD NOT GOOD
        //asts.push(RunPrimitiveInfo::new(file.clone(), variant.state.0.into(), id, parse));
        let mut ast = RunPrimitiveInfo::new(file.clone(), variant.state.0.into(), parse);
        ast
    }).collect::<Vec<_>>();
    #[cfg(feature = "timed")]
    {
        let duration = std::time::Instant::now().checked_duration_since(start).unwrap_or_default();
        file.set_sq_parse_time(duration);
    }
    return asts;
}

pub fn get_variable_local(scope: Arc<Scope>, name: &String) -> Option<Arc<Variable>>{
    //Will traverse scopes, will not traverse globals
    let var = scope.vars.index(name);
    if let Some(var) = var{
        return Some(var.clone());
    }
    if let Some(parent) = &scope.parent{
        if let Some(parent) = parent.upgrade(){
            return get_variable_local(parent.clone(), name);
        }
    }

    return None;
}

pub fn get_variable(analyser: &Analyser, scope: Arc<Scope>, state: Arc<RwLock<AnalysisState>>, name: &String) -> VariantData<Arc<Variable>>{
    match scope.vars.index(name){
        Some(var) => {
            let variant_id = analyser.get_distinct_variant(state.read().unwrap().file.as_ref()).unwrap();
            return VariantData::Single(variant_id.clone(), var.clone())//Todo: I should differ based on WHAT the variable is
        },
        None => {}
    }
    
    if let Some(parent) = &scope.parent{
        if let Some(parent) = parent.upgrade(){
            return get_variable(analyser, parent.clone(), state, name);
        }
    }
    //Begin searching in the globals
    //let token = state.read().unwrap().file.get_global_internal(name.to_string());
    let distinct = state.read().unwrap().file.clone();
    let token: VariantData<Arc<Variable>> = analyser.query_step(distinct.as_ref(), FileFilter::All(true), 
    &mut |variant: &SQDistinctVariant, globals: &Arc<Globals>| {
        globals.get(name)
    } ).unwrap();
    //println!("Searching for {} in globals ", name);
    //return token;
    return token;
    //Below is now redundant since GlobalSeardh no longer exists
    //match token{
    //    GlobalSearch::Single(var, source) => {
    //        return VariableSearch::Single(var);
    //    },
    //    GlobalSearch::Multi(vars) => {
    //        let mut result = Vec::new();
    //        for (condition, var, source) in vars{
    //            result.push(var);
    //        }
    //        return VariableSearch::Multi(result);
    //    },
    //    GlobalSearch::MultiIncomplete(vars) => {
    //        let mut result = Vec::new();
    //        for (condition, var, source) in vars{
    //            result.push(var);
    //        }
    //        return VariableSearch::MultiIncomplete(result);
    //    },
    //    _ => {}
    //}
    //return VariableSearch::None;
}


pub fn get_type(scope: Arc<Scope>, name: &String) -> Option<Arc<TypeDef>>{
    if let Some(var) = scope.types.index(name){
        return Some(var.clone());
    }
    if let Some(parent) = &scope.parent{
        if let Some(parent) = parent.upgrade(){
            return get_type(parent.clone(), name);
        }
    }
    return None;
}

pub fn find_funcs(scope: Arc<Scope>, steps: &Vec<Element<AST>>) {
    for step in steps{
        match step.value.as_ref(){
            AST::Function { name, args, returns, actions } => {
                scope.vars.add(name.value.to_string(), Variable::new(step.clone()));
            },
            _ => {}
        }
    }
}


#[cfg(test)]
#[test]
fn run_testfile(){
    use std::{fs::read_to_string, path::PathBuf};

    use single_file::parse_file;
    //let text = read_to_string("squirrelFile.nut").unwrap();
    let file = FileInfo::new("squirrelFile.nut".to_string(), PathBuf::from("squirrelFile.nut"), "MP || UI".to_string(), common::FileType::RSquirrel);
    println!("File: {:?}", file);
    

}

