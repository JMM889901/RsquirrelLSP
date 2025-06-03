use std::cell::RefCell;

use ConfigAnalyser::SqFileVariant;
use crate::{grammar::squirrel_ast, SquirrelParse};
use super::generate_data_structure;
use crate::ast::*;
//Test simple literal declarations
#[test]
fn test_expect_simple(){
    let text = "expect int(5.1)".to_string();
    let structure = generate_data_structure(text);
    let result = squirrel_ast::expression(&structure, &RefCell::new(0), &SquirrelParse::empty());
    println!("{:?}", result);
    if let AST::Expect(a, b) = result.unwrap().value.as_ref(){
        assert_eq!(a.value.as_ref(), &Type::Int);
        assert_eq!(b.value.as_ref(), &AST::Literal(Type::Float));
    }

}

#[test]
fn test_expect_custom(){
    let text = "expect sometypedef(5.1)".to_string();
    let structure = generate_data_structure(text);
    let result = squirrel_ast::expression(&structure, &RefCell::new(0), &SquirrelParse::empty());
    println!("{:?}", result);
    if let AST::Expect(a, b) = result.unwrap().value.as_ref(){
        assert_eq!(a.value.as_ref(), &Type::Named("sometypedef".to_string()));
        assert_eq!(b.value.as_ref(), &AST::Literal(Type::Float));
    }

}

#[test]
fn test_doubledot_floats(){
    //Floats can have 2 decimal points, this does nothing productive but it can happen
    let text = "5.1.1".to_string();
    let structure = generate_data_structure(text);
    let result = squirrel_ast::expression(&structure, &RefCell::new(0), &SquirrelParse::empty());
    println!("{:?}", result);
    assert!(&AST::Literal(Type::Float) == result.unwrap().value.as_ref())
}

#[test]
fn test_clone(){
    let text = "clone 5.1".to_string();
    let structure = generate_data_structure(text);
    let result = squirrel_ast::expression(&structure, &RefCell::new(0), &SquirrelParse::empty());
    println!("{:?}", result);
    assert!(matches!(result.unwrap().value.as_ref(), AST::Clone(_)));
}

#[test]
fn test_ternary(){
    let text = "5.1 ? 5.2 : 5.3".to_string();
    let structure = generate_data_structure(text);
    let result = squirrel_ast::expression(&structure, &RefCell::new(0), &SquirrelParse::empty());
    println!("{:?}", result);
    if let AST::Ternary(a, b, c) = result.unwrap().value.as_ref(){
        assert_eq!(a.value.as_ref(), &AST::Literal(Type::Float));
        assert_eq!(b.value.as_ref(), &AST::Literal(Type::Float));
        assert_eq!(c.value.as_ref(), &AST::Literal(Type::Float));
    }
}