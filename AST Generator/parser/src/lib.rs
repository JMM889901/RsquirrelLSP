use std::{fmt::Error, str, sync::RwLock};

use ast::{Element, AST};
use ConfigAnalyser::get_file_varaints;
pub mod ast;
pub mod visitor;
pub mod error;
pub mod grammar;
pub mod external_resources;

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