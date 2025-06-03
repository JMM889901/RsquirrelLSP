use std::cell::RefCell;

use ConfigAnalyser::sq_file_variant::SqFileVariant;
use crate::{grammar::squirrel_ast, SquirrelParse};
use super::generate_data_structure;
use crate::ast::*;



#[test]
fn test_filescope(){
    let text = "
    typedef funny int
    funny C = 1
    cosnt arbitrary = 5
    const int typed = 5
    int A = 5
    struct B { 
        int a 
        int b 
    }".to_string();

    let structure = generate_data_structure(text);
    let parse = &SquirrelParse::empty();
    let result = squirrel_ast::file_scope(&structure, &RefCell::new(0), parse);
    println!("{:?}", result);
    println!("{:?}", parse);
}


#[test]
fn test_filescope_function(){
    let text = "
    B function createStruct(){
        B b
        b.a = 5
        b.b = 6
        return b
    }
    struct B { 
        int a 
        int b 
    }".to_string();

    let structure = generate_data_structure(text);
    let parse = &SquirrelParse::empty();
    let result = squirrel_ast::file_scope(&structure, &RefCell::new(0), parse);
    println!("{:?}", result);
    for ast in result.unwrap(){
        println!("{}", ast.value);
    }
    println!("{:?}", parse);
}