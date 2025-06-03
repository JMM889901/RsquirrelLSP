use std::cell::RefCell;
use crate::SquirrelParse;
use ConfigAnalyser::sq_file_variant::SqFileVariant;
use crate::grammar::squirrel_ast;
use super::generate_data_structure;
use crate::ast::*;
//Test simple assignment
#[test]
fn test_assign_simple(){
    let text = "a = 5".to_string();
    let structure = generate_data_structure(text);
    let result = squirrel_ast::assignment(&structure, &RefCell::new(0), &SquirrelParse::empty());
    assert_eq!(result, Ok(Element::new(AST::Assignment { var: Element::new(AST::Variable(Element::new("a".to_string(), (0, 1))), (0, 1)), value: Element::new(AST::Literal(Type::Int), (4, 5))}, (0, 5))));
}

//Test assignment with expression
#[test]
fn test_assign_expr(){
    let text = "a = 5 + 3 * 2".to_string();
    let structure = generate_data_structure(text);
    let result = squirrel_ast::assignment(&structure, &RefCell::new(0), &SquirrelParse::empty());
    assert_eq!(result, 
        Ok(Element::new(AST::Assignment { var: Element::new(AST::Variable(Element::new("a".to_string(), (0, 1))), (0, 1)), value: Element::new(AST::Add(Element::new(
            AST::Literal(Type::Int), (4, 5)), 
            Element::new(AST::Mul(Element::new(
                AST::Literal(Type::Int), (8, 9)), Element::new(
                AST::Literal(Type::Int), (12, 13))
            ), (8, 13))), (4, 13))} , (0, 13))
    ))
}

//Test assignment as variable
#[test]
fn test_assign_var(){
    let text = "a = b".to_string();
    let structure = generate_data_structure(text);
    let result = squirrel_ast::assignment(&structure, &RefCell::new(0), &SquirrelParse::empty());
    assert_eq!(result, 
        Ok(Element::new(AST::Assignment { var: Element::new(AST::Variable(Element::new("a".to_string(), (0, 1))), (0, 1)), value: Element::new(AST::Variable(Element::new("b".to_string(), (4, 5))), (4, 5))}, (0, 5))
    ))
}
