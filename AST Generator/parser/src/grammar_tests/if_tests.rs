use std::cell::RefCell;

use ConfigAnalyser::SqFileVariant;
use crate::{grammar::squirrel_ast, SquirrelParse};
use super::generate_data_structure;
use crate::ast::*;

//Test simple if
#[test]
fn test_if_simple(){
    let text = "if (a == 5){
        a = 5
    }".to_string();
    let structure = generate_data_structure(text);
    let parse = &SquirrelParse::empty();
    let result = squirrel_ast::if_statement(&structure, &RefCell::new(0), parse);
    println!("{:?}", result);
    println!("{:?}", parse);
}

//Test function if
#[test]
fn test_if_funct(){
    let text = "
		if(IsValid(drone.DroneMover))
        drone.DroneMover.ClearParent()".to_string();
    let structure = generate_data_structure(text);
    let parse = &SquirrelParse::empty();
    let result = squirrel_ast::if_statement(&structure, &RefCell::new(0), parse);
    println!("{:?}", result);
    println!("{:?}", parse);
}

