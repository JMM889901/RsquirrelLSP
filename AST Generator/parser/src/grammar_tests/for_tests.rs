use std::cell::RefCell;

use ConfigAnalyser::sq_file_variant::SqFileVariant;
use crate::{grammar::squirrel_ast, SquirrelParse};
use super::generate_data_structure;
use crate::ast::*;

//Test untyped foreach

#[test]
fn test_foreach_untyped(){
    let text = "foreach (a in b){
        a = 5
    }".to_string();
    let structure = generate_data_structure(text);
    let parse = &SquirrelParse::empty();
    let result = squirrel_ast::foreach(&structure, &RefCell::new(0), parse);
    println!("{:?}", result);
    println!("{:?}", parse);
}

//Test for 
#[test]
fn test_for(){
    let text = "for (a = 0; a < 5; a++){
        a = 5
    }".to_string();
    let structure = generate_data_structure(text);
    let parse = &SquirrelParse::empty();
    let result = squirrel_ast::for_statement(&structure, &RefCell::new(0), parse);
    println!("{:?}", result);
    println!("{:?}", parse);
}