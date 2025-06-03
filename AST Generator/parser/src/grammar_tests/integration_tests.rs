use std::{cell::RefCell, fs::read_to_string};

use ConfigAnalyser::sq_file_variant::SqFileVariant;
use crate::{grammar::squirrel_ast, SquirrelParse};
use super::generate_data_structure;
use crate::ast::*;

use ConfigAnalyser::get_file_varaints;
#[test]
fn run_testfile(){
    let text = read_to_string("./squirrelFile.nut").unwrap();
    let results = get_file_varaints(&text, &"MP || UI".to_string(), &"name".to_string());
    for variant in results{
        println!("parsing {:?}", variant.state.identifier());
        let parse = &SquirrelParse::empty();
        let result = squirrel_ast::file_scope(&variant, &RefCell::new(0), parse);
        for ast in result.unwrap(){
            println!("{:?}", ast.value);
        }
        for err in parse.errs.read().unwrap().iter(){
            println!("{:?} at \n {} \n", err, text[err.range.0 .. err.range.1].to_string());
        }
    }
}

