use std::cell::RefCell;

use ConfigAnalyser::sq_file_variant::SqFileVariant;
use crate::{grammar::squirrel_ast, SquirrelParse};
use super::generate_data_structure;
use crate::ast::*;
//Test simple literal declarations
#[test]
fn test_decl_simple(){
    let text = "expect int(5.1)".to_string();
    let structure = generate_data_structure(text);
    let result = squirrel_ast::expression(&structure, &RefCell::new(0), &SquirrelParse::empty());
    println!("{:?}", result)
}