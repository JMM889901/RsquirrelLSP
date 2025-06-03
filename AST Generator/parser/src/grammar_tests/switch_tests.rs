
use std::cell::RefCell;

use ConfigAnalyser::sq_file_variant::SqFileVariant;
use crate::{grammar::squirrel_ast, SquirrelParse};
use super::generate_data_structure;
use crate::ast::*;
//Simple switch test
#[test]
fn simple_switch(){
    let text = "
    switch(a){
        case 1: 
            return 1
        case 2:
            int x = 3
            return x
        default:
            return 3
    }".to_string();
    let structure = generate_data_structure(text);
    let parse = &SquirrelParse::empty();
    let result = squirrel_ast::switch(&structure, &RefCell::new(0), parse);
    assert!(result.is_ok());
    if let AST::Switch { condition, cases, default } = *result.clone().unwrap().value{
        assert!(matches!(*condition.value, AST::Variable(_)));
        assert_eq!(cases.len(), 2);
        assert!(default.is_some());
    }
    println!("{:?}", result);
    println!("{:?}", parse);
}