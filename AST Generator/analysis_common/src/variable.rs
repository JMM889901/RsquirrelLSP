use std::{fmt::Debug, path::PathBuf, sync::{Arc, Weak, RwLock}};

use common::FileInfo;
use ASTParser::ast::{Element, AST};

use crate::{spanning_search::{SpanningSearch, Traversable}, CompiledState, RunPrimitiveInfo};



#[derive(Debug, Clone)]
pub enum VariableSearch{
    Single(Arc<Variable>),
    Multi(Vec<Arc<Variable>>),
    MultiIncomplete(Vec<Arc<Variable>>),
    None,
}//I really REALLY dont like bringing compiler context back into the analyser as thats a preprocessor thing
//^Well at least its abstracted away enough that i dont have to look at it, mostly anyway
#[derive(Debug)]
pub enum Variable{
    Variable(VariableInternal),
    Global(VariableExternal),
}
#[derive(Debug)]
pub struct VariableExternal{
    pub ast: Element<AST>,
    references: RwLock<Vec<Weak<VariableReference>>>,
    pub source_run: Arc<RunPrimitiveInfo>,
}
#[derive(Debug)]
pub struct VariableInternal{//These types are mostly here for later additional implementations, for now they are just ASTs
    pub ast: Element<AST>,//Bad and scary deep copy
    references: RwLock<Vec<Weak<VariableReference>>>
    //
    //More probably needed
}
impl Variable{
    pub fn new(ast: Element<AST>) -> Self{
        Variable::Variable(VariableInternal{
            ast,
            references: RwLock::new(Vec::new())
        })
    }
    pub fn external(ast: Element<AST>, source_run: Arc<RunPrimitiveInfo>) -> Self{
        Variable::Global(VariableExternal{
            ast,
            references: RwLock::new(Vec::new()),
            source_run
        })
    }
    pub fn ast(&self) -> &Element<AST>{
        match self {
            Variable::Variable(var) => {
                return &var.ast;
            },
            Variable::Global(var) => {
                return &var.ast;
            }
        }
    }
    pub fn try_add_reference(&self, reference: Arc<VariableReference>){
        match self {
            Variable::Variable(var) => {
                let mut references = var.references.write().unwrap();
                references.push(Arc::downgrade(&reference));
            },
            Variable::Global(var) => {
                //This is sketchy because VariableReference is defined in analyser but Token comes from TokenIdentifier
                let mut references = var.references.write().unwrap();
                references.push(Arc::downgrade(&reference));
            }
        }
    }
    pub fn get_all_references(&self) -> Vec<Weak<VariableReference>>{
        match self {
            Variable::Variable(var) => {
                let references = var.references.read().unwrap();
                return references.clone();
            },
            Variable::Global(var) => {
                let references = var.references.read().unwrap();
                return references.clone();
            }
        }
    }

    pub fn get_valid_references(&self) -> Vec<Arc<VariableReference>>{
        match self {
            Variable::Variable(var) => {
                let references = var.references.read().unwrap();
                let references = references.iter().filter_map(|x| x.upgrade()).collect::<Vec<_>>();
                return references.clone();
            },
            Variable::Global(var) => {
                //println!("Global variable {} has no references", var.ast.name.value);
                //panic!("Global variable {:?} is a phantom variable and thus has no references", var.references);
                let mut references = var.references.read().unwrap();
                let length = references.len();
                let filter = references.iter().filter_map(|x| x.upgrade()).collect::<Vec<_>>();
                if filter.len() != length{
                    drop(references);
                    let mut write = var.references.try_write().unwrap();
                    *write = write.iter().filter(|x| x.upgrade().is_some()).cloned().collect::<Vec<_>>();
                }
                return filter.clone();
            }
        }
    }
    pub fn try_get_context(&self) -> CompiledState{
        match self {
            Variable::Variable(var) => {
                panic!("Not possible currently, maybe someday")
            },
            Variable::Global(var) => {
                return var.source_run.context.clone();
            }
        }
    }
    pub fn file_path(&self) -> Option<PathBuf>{
        match self {
            Variable::Variable(var) => {
                return None; //Local variables dont bother to store stuff like that
            },
            Variable::Global(var) => {
                return Some(var.source_run.file.path().clone());
            }
        }
    }
    pub fn file(&self) -> Option<FileInfo>{
        match self {
            Variable::Variable(var) => {
                return None; //Local variables dont bother to store stuff like that
            },
            Variable::Global(var) => {
                return Some(var.source_run.file.clone());
            }
        }
    }
    pub fn get_range_precise(&self) -> (usize, usize){
        let ast = match self {
            Variable::Variable(var) => {
                &var.ast
            },
            Variable::Global(var) => {
                &var.ast
            }
        };
        match ast.value.as_ref() {
            AST::Function { name, args, returns, actions } => {
                return name.range;
            }
            AST::ExternalReference(refer) => {
                return refer.range();
            }
            _ => {
                return ast.range;
            }
        }
    }
}
impl SpanningSearch<Variable> for Variable{
    fn range(&self) -> (usize, usize) {
        panic!("we dont do that here")
    }
    fn range_passes(&self, pos: usize, leeway: usize) -> bool {
        match self {
            Variable::Variable(var) => {
                return var.range_passes(pos, leeway);
            },
            Variable::Global(var) => {
                //This is very not good and sketchy
                //We dont technically have any way to confirm that the position is even in this file
                //But because im trying to phase out the globalfuncbridge stuff i need to do this
                return var.range_passes(pos, leeway);
            }
        }
    }
}
impl SpanningSearch<VariableInternal> for VariableInternal{
    fn range(&self) -> (usize, usize) {
        return self.ast.range;
    }
}
impl SpanningSearch<VariableExternal> for VariableExternal{
    fn range(&self) -> (usize, usize) {
        return self.ast.range;
    }
}

#[derive(Clone)]
pub struct VariableReference{
    pub target: Arc<Variable>,
    pub source: Element<AST>,//Bad and scary deep copy
    pub source_run: Arc<RunPrimitiveInfo>//This is the file that the reference is in, not the target
}
impl VariableReference{
    pub fn new(target: Arc<Variable>, source: Element<AST>, source_run: Arc<RunPrimitiveInfo>) -> Self{
        VariableReference{
            target,
            source,
            source_run
        }
    }
    fn range_passes_target(&self, pos: usize, leeway: usize) -> bool {
        return self.target.range_passes(pos, leeway)
    }
    fn range_passes_source(&self, pos: usize, leeway: usize) -> bool {
        let range = self.source.range;
        if range.0 <= pos + leeway && range.1 + leeway >= pos{
            return true
        }
        return false
    }
    pub fn is_target_global(&self) -> bool{
        match self.target.as_ref() {
            Variable::Global(_) => {
                return true;
            },
            _ => {
                return false;
            }
        }
    }
    pub fn text_nice(&self) -> String{
        format!("referenced by: {:?}:\n\n references: {},\n\n from:{:#?}\n\n", self.source, self.target.ast().text_none_rec() , self.source_run.file.path())
    }
}
impl Debug for VariableReference{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VariableReference")
            .field("target", &self.target.get_range_precise())
            .field("source", &self.source.range)
            .field("source_run", &self.source_run.file)
            .finish()
    }
}
impl SpanningSearch<VariableReference> for VariableReference{
    fn range(&self) -> (usize, usize) {
        return self.source.range;
    }
    fn range_passes(&self, pos: usize, leeway: usize) -> bool {
        return self.range_passes_target(pos, leeway) || self.range_passes_source(pos, leeway);
    }
}
impl Traversable<VariableReference>{
    pub fn get_pos_target(&self, pos: usize, leeway: usize) -> Vec<Arc<VariableReference>>{
        let items = self.get();
        items.into_iter().filter(|x| x.range_passes_target(pos, leeway)).collect()
    }
    pub fn get_pos_source(&self, pos: usize, leeway: usize) -> Vec<Arc<VariableReference>>{
        let items = self.get();
        items.into_iter().filter(|x| x.range_passes_source(pos, leeway)).collect()
    }
}