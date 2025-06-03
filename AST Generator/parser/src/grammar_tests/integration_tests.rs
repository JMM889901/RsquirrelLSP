use std::{cell::RefCell, fs::read_to_string};

use common::FileInfo;
use ConfigAnalyser::SqFileVariant;
use crate::{grammar::squirrel_ast, SquirrelParse};
use super::generate_data_structure;
use crate::ast::*;

use ConfigAnalyser::get_file_varaints;
#[cfg(test)]
#[test]
fn run_testfile(){
    use std::path::PathBuf;

    use common::FileType;

    let file_info = FileInfo::new( "name".to_string(), PathBuf::from("./squirrelFile.nut"), "MP || UI".to_string(), FileType::RSquirrel);
    let text = file_info.text();
    let results = get_file_varaints(file_info);
    for variant in results{
        println!("parsing {:?}", variant.state.identifier());
        let parse = &SquirrelParse::empty();
        let result = squirrel_ast::file_scope_dbg(&variant, &RefCell::new(0), parse);
        //for ast in result.unwrap(){
        //    println!("{:?}", ast.value);
        //}
        //for err in parse.errs.read().unwrap().iter(){
        //    println!("{:?} at \n {} \n", err, text[err.range.0 .. err.range.1].to_string());
        //}
        let result = result.unwrap();
        //Test that no errors occured.
        assert!(parse.errs.read().unwrap().is_empty(), "Errors found in parse: {:?}", parse.errs.read().unwrap());
        for ast in result{
            //Quick and dirty test for preprocessing tracking offsets
            if let AST::Function { name, args,  returns, actions } = ast.value.as_ref(){
                let pos = name.range;
                let name = name.value.as_ref();
                let text_portion = text[pos.0 .. pos.1].to_string();
                assert_eq!(&text_portion, name);
            }
        }
    }
}
