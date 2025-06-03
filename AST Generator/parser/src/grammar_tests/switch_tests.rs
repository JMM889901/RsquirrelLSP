
use std::cell::RefCell;

use ConfigAnalyser::SqFileVariant;
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
    //if let AST::Switch { condition, cases, default } = result.clone().unwrap().unwrap_v(){
    //    assert!(matches!(*condition.value, AST::Variable(_)));
    //    assert_eq!(cases.len(), 2);
    //    assert!(default.is_some());
    //}
    println!("{:?}", result);
    println!("{:?}", parse);
}

#[test]
fn trailing_semicolon_switch() {
    let text = "
    switch (chargeLevel){
        case 0: 
            if (chargeFrac > 0.266) {
                int x = 1
            }
            else{
                int x = 2
            }
            break;
    }".to_string();
    let structure = generate_data_structure(text);
    let parse = &SquirrelParse::empty();
    let result = squirrel_ast::switch(&structure, &RefCell::new(0), parse);
    println!("{:?}", result);
    let result = result .unwrap();
    let ast = result.unwrap_v();
    if let AST::Switch { condition, cases, default } = ast {
        assert!(matches!(*condition.value, AST::Variable(_)));
        assert_eq!(cases.len(), 1);
        assert!(default.is_none());
    } else {
        panic!("Expected AST::Switch, found {:?}", ast);
    }
}

#[test]
fn nested_cases() {
    let text = "
    switch (chargeLevel){
        case 0: 
        case 1: 
            int x = 1
            break
        case 2:
        default : 
            int x = 2
            break
    }".to_string();
    let structure = generate_data_structure(text);
    let parse = &SquirrelParse::empty();
    let result = squirrel_ast::switch(&structure, &RefCell::new(0), parse);
    println!("{:?}", result);
    let result = result .unwrap();
    let ast = result.unwrap_v();
    if let AST::Switch { condition, cases, default } = ast {
        assert!(matches!(*condition.value, AST::Variable(_)));
        assert_eq!(cases.len(), 2);
        for case in cases{
            if let AST::Case { condition, actions } = case.value.as_ref() {
                assert_eq!(condition.len(), 2);
            } else {
                panic!("Expected AST::Case, found {:?}", case);
            }
        }
        assert!(default.is_none()); //Default is now mostly redundant, its folded into cases
    } else {
        panic!("Expected AST::Switch, found {:?}", ast);
    }
}