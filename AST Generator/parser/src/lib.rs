use std::{cell::RefCell, fmt::Error, str, sync::{Arc, RwLock}};

use analysis_runner::{state_resolver::{CompiledState, HasState}, Analyser, AnalysisResult, AnalysisReturnType, AnalysisStep, HasVariantID, SQDistinctVariant};
use ast::{Element, AST};
use common::FileInfo;
use ConfigAnalyser::{get_file_varaints, SqFileVariant};

use crate::grammar::squirrel_ast;
pub mod ast;
pub mod visitor;
pub mod error;
pub mod grammar;
pub mod external_resources;

pub struct ASTParseStep{}
impl AnalysisStep for ASTParseStep{
    fn analyse(&self, variant: &SQDistinctVariant, analyser: &Analyser) -> AnalysisReturnType {
        let parse_data = SquirrelParse::empty();
        let offset = RefCell::new(0);
        let var = SqFileVariant::from_text(variant.text().clone(), variant.get_state().clone().into());//TODO: Expensive string clone
        let parse = squirrel_ast::file_scope_dbg(&var, &offset, &parse_data);
        //At this point this is a redundant type, but i dont feel like pulling all that out right now
        let run = RunPrimitiveInfo::new(
            variant.get_file().clone(),
            variant.get_state().clone().into(),
            parse.clone().unwrap()//TODO: Scary unwrap
        );//Really i shouldnt need to unwrap almost anywhere, error recovery should be possible (and is probably needed)
        Ok(Arc::new(
            ASTParseResult {
                parse_data,
                run: Arc::new(run)
            }
        ))
        
    }
}
pub struct ASTParseResult{
    pub parse_data: SquirrelParse,
    pub run: Arc<RunPrimitiveInfo>,//Again, this does not really need to be like this but i dont feel like yanking out all that code
}
impl AnalysisResult for ASTParseResult {}


#[derive(Debug, Clone)]
pub struct RunPrimitiveInfo{
    //Essentially the run after parsing, before any further steps
    //Should effectively be able to be used anywhere to say where, what and in what context
    pub file: FileInfo,
    pub context: CompiledState,
    pub ast: Vec<Element<AST>>,//TODO: See about removing this
    //It feels wrong to have it here
}
impl RunPrimitiveInfo{
    pub fn new(file: FileInfo, context: CompiledState, ast: Vec<Element<AST>>) -> Self{
        RunPrimitiveInfo{
            file,
            context,
            ast
        }
    }
}
impl HasState for RunPrimitiveInfo{
    fn get_state(&self) -> CompiledState {
        return self.context.clone();
    }
}
impl HasVariantID for RunPrimitiveInfo {
    fn get_state(&self) -> &CompiledState {
        &self.context
    }

    fn get_file(&self) -> &FileInfo {
        &self.file
    }
}

//Load tests
#[cfg(test)]
mod grammar_tests;

#[derive(Debug)]
pub struct SquirrelParse {
    pub errs: RwLock<Vec<Element<crate::error::Error>>>,
    globals: RwLock<Vec<Global>>,
    untyped: RwLock<bool>,
}
impl SquirrelParse{
    pub fn empty() -> Self{
        SquirrelParse{
            errs: RwLock::new(Vec::new()),
            globals: RwLock::new(Vec::new()),
            untyped: RwLock::new(false),
        }
    }
    pub fn register_error(&self, err: Element<crate::error::Error>){
        self.errs.write().unwrap().push(err);
    }
}
#[derive(Debug)]
pub enum Global {
    Function,
    Struct,
    Enum,
    Typedef,
    Const,
}

fn ast_vec_to_string(ast: Vec<Element<AST>>) -> String{
    let string = String::new();
    for ast in ast{
        let value = ast.value;

    }
    todo!()
}

fn ast_to_string(ast: AST) -> String{
    match ast {
        _ => todo!()
    }
    todo!()
}