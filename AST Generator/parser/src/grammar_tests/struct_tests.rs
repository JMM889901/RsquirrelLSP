use std::cell::RefCell;

use ConfigAnalyser::SqFileVariant;
use crate::{grammar::squirrel_ast, SquirrelParse};
use super::generate_data_structure;
use crate::ast::*;
//Test simple struct declaration
#[test]
fn test_struct_simple(){
    let text = "
    struct A { 
        int a 
        int b 
    }".to_string();
    let structure = generate_data_structure(text);
    let parse = &SquirrelParse::empty();
    let result = squirrel_ast::struct_def(&structure, &RefCell::new(0), parse);
    println!("{:?}", result);
    println!("{:?}", parse);
}//There are no abstract structs or anything, so not many more "funky" things here that arent covered by declaration tests