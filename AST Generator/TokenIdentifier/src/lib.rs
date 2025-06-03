use std::sync::{Arc, RwLock};

use analysis_common::{spanning_search::{Traversable, TraversableMap}, variable::Variable, RunPrimitiveInfo};
use ASTParser::ast::{Element, AST};
mod external;



#[derive(Debug)]
pub struct Globals {//TOOD: Should this type even exist? theres unlikely to be any additional impls
    pub globals: TraversableMap<String, Variable>,
    //This causes something of an abstraction, a variable will "exist" twice, once locally and once globally
}
//#[derive(Debug)]
//pub struct Token {//Token is a primitive form of Variable, so lets just use variable instead duh
//    pub name: String,
//    pub value: Element<AST>,
//    pub references: RwLock<Vec<(Element<AST>, String)>>,
//}
impl Globals{
    //At this point globals can be read but not really written to
    pub fn get(&self, key: &String) -> Option<Arc<Variable>>{
        if let Some(var) = self.globals.index(key){
            return Some(var.clone());
        }
        return None;
    }
}

pub fn get_globals(run: Arc<RunPrimitiveInfo>) -> Globals {
    let mut globals = TraversableMap::new();
    for step in &run.ast {


        //let post: Option<Box<dyn Fn(&(), &Vec<(String, Arc<Variable>)>, &Element<AST>) -> Vec<(String, Arc<Variable>)>>> = None;//This should not be necessary
        ////Rusts type checker has a hissy fit
        //let result = AST::visit(&step, &(), &analyse_step, &post);
        //globals.add(key, item);
        let mut all_global = RwLock::new(None);
        let post: Option<Box<dyn Fn(&(Arc<RunPrimitiveInfo>, &RwLock<Option<Element<AST>>>), &Vec<(String, Arc<Variable>)>, &Element<AST>) -> Vec<(String, Arc<Variable>)>>> = None;//This should not be necessary
        let context = (run.clone(), &all_global);
        let res = AST::visit(step, &context, &analyse_step, &post);
        for (key, item) in res {
            globals.add_arc(key, item);
        }
    }
    Globals { globals }
}

pub fn analyse_step(ctx: &(Arc<RunPrimitiveInfo>, &RwLock<Option<Element<AST>>>), tokens: &Vec<(String, Arc<Variable>)>, step: &Element<AST>) -> Vec<(String, Arc<Variable>)>{
    let run = ctx.0.clone();
    let mut all_global = &ctx.1;
    let mut tokens = tokens.clone(); 
    match step.value.as_ref() {
        AST::GlobalizeAllFunctions => {
            all_global.write().unwrap().clone_from(&Some(step.clone()));
            return tokens
        }
        AST::ConstDeclaration { global, name, vartype, value } => {
            if *global {
                let var = Variable::external(step.clone(), run.clone());
                let token_name = name.value.clone();
                tokens.push((token_name.to_string(), Arc::new(var)));
            }
            return tokens
        }
        AST::EnumDeclaration { global, name } => {
            if *global {
                if *global {
                    let var = Variable::external(step.clone(), run.clone());
                    let token_name = name.value.clone();
                    tokens.push((token_name.to_string(), Arc::new(var)));
                }
            }
            return tokens
        }
        AST::StructDeclaration { global, name, attributes } => {
            if *global {
                let var = Variable::external(step.clone(), run.clone());
                let token_name = name.value.clone();
                tokens.push((token_name.to_string(), Arc::new(var)));
            }

            return tokens
        }
        AST::Global(name) => {
            let var = Variable::external(step.clone(), run.clone());
            tokens.push((name.clone(), Arc::new(var)));
            return tokens
        }
        AST::Function { name, args, returns, actions } => {
            if let Some(all_global) = all_global.read().unwrap().clone() {
                let var = Variable::external(all_global.clone(), run.clone());
                let token_name = name.value.clone();
                tokens.push((token_name.to_string(), Arc::new(var)));
            }
            return tokens
        }
        _ => (vec![]),
    }
}
    