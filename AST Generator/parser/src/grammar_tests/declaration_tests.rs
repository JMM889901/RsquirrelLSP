use std::cell::RefCell;

use ConfigAnalyser::SqFileVariant;
use crate::{grammar::squirrel_ast, SquirrelParse};
use super::generate_data_structure;
use crate::ast::*;
//Test simple literal declarations
#[test]
fn test_decl_simple(){
    let text = "int a = 5".to_string();
    let structure = generate_data_structure(text);
    let result = squirrel_ast::declaration(&structure, &RefCell::new(0), &SquirrelParse::empty());
    assert_eq!(result, Ok(Element::new(AST::Declaration { name: Element::new("a".to_string(), (4, 5)), vartype: Element::new(Type::Int, (0, 3)), value: Some(Element::new(AST::Literal(Type::Int), (8, 9)))}, (0, 9))));
}

//Test declaration with expression
#[test]
fn test_decl_expr(){
    let text = "int a = 5 + 3 * 2".to_string();
    let structure = generate_data_structure(text);
    let result = squirrel_ast::declaration(&structure, &RefCell::new(0), &SquirrelParse::empty());
    println!("{:?}", result);
    assert_eq!(result, 
        Ok(Element::new(AST::Declaration { name: Element::new("a".to_string(), (4, 5)), vartype: Element::new(Type::Int, (0,3)), value: Some(
            Element::new(AST::Add(Element::new(
                AST::Literal(Type::Int), (8, 9)), 
                Element::new(AST::Mul(Element::new(
                    AST::Literal(Type::Int), (12, 13)), Element::new(
                    AST::Literal(Type::Int), (16, 17))
                ), (12, 17))), (8, 17))) }, (0, 17)))
    )
}
//Test declaration as variable
#[test]
fn test_decl_var(){
    let text = "int a = b".to_string();
    let structure = generate_data_structure(text);
    let result = squirrel_ast::declaration(&structure, &RefCell::new(0), &SquirrelParse::empty());
    assert_eq!(result, 
        Ok(Element::new(AST::Declaration { name: Element::new("a".to_string(), (4, 5)), vartype: Element::new(Type::Int, (0, 3)), value: Some(Element::new(AST::Variable(Element::new("b".to_string(), (8, 9))), (8, 9)))}, (0, 9)))
    )
}

//Some identities are not valid
#[test]
fn test_idents_fail(){
    let banned = ["global", "expect" , "function" , "return" , "switch" , "if" , "typedef" , "else", "delete", "throw"];
    for ident in banned{
        let text = format!("int {} = 5", ident);
        let structure = generate_data_structure(text);
        let result = squirrel_ast::declaration(&structure, &RefCell::new(0), &SquirrelParse::empty());
        assert!(result.is_err(), "Expected error for ident: {}", ident);
    }
    let extra = "var";
    for ident in banned{
        let text = format!("int {}{} = 5", ident, extra);
        let structure = generate_data_structure(text);
        let result = squirrel_ast::declaration(&structure, &RefCell::new(0), &SquirrelParse::empty());
        assert!(matches!(result.unwrap().value.as_ref(), AST::Declaration {name, value, vartype }));
    }
}