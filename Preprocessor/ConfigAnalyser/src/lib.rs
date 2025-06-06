use std::{collections::HashMap, fs::{self, read_to_string}, hash::Hash, path::PathBuf, sync::Arc};

use common::{FileInfo, FileType};
use ConfigPredictor::{get_states, state::SqCompilerState};
use PreprocessorParser::*;
//use sq_file_variant::SqFileVariant;
//pub use variant_parser::variant::SqFileVariant;
mod variant_parser;
mod sq_file_variant;
pub use sq_file_variant::SqFileVariant;
pub fn get_file_varaints(file: FileInfo) -> Vec<SqFileVariant>{
    let node = parse_file(&file.text(), file.run_on());
    let states = get_states(&node.ast);
    //println!("states");
    //for state in &states{
    //    println!("{:?}", state.identifier());
    //    println!("{:?}\n\n", state)
    //}
    let variants:Vec<SqFileVariant> = states.into_iter().map(|x| SqFileVariant::generate(&node, x)).collect();
    

    //for variant in &variants {
    //    fs::create_dir_all(format!("./preprocessed{}", variant.state.to_path()));
//
    //    let file = format!("./preprocessed{}/{}.pnut", variant.state.to_path(), file.name());
    //    let text = variant.text();
    //    fs::write(file, text).expect("Unable to write file");
//
    //}
    return variants;
}
pub fn get_condition_variants(file: FileInfo) -> Vec<SqCompilerState>{
    let node = parse_file(&file.text(), file.run_on());
    let states = get_states(&node.ast);
    return states;
}


#[test]
fn integration_mp_only(){
    let file_info = FileInfo::new("".to_string(), PathBuf::from(""), "MP".to_string(), FileType::RSquirrel);
    file_info.set_text("".to_string());
    let result = get_file_varaints(file_info);
    println!("{:?}", result);
    assert_eq!(result.len(), 1);
}

pub fn force_get_states_statement(text: &String) -> Vec<HashMap<String, bool>>{
    //I cannot express how HORRENDOUSLY inefficient this is, but it works for now
    //Code\AST Generator\analyser\src\load_order.rs
    
    let condition = parse_condition_expression(&text);
    let node = ast::AST::RunOn(vec![], condition);
    let states = get_states(&node);
    let mut result = Vec::new();
    for state in states{
        result.push(state.0);
    }
    return result;
}


peg::parser!{
    pub grammar pos_conversion(pos: usize) for SqFileVariant{
        #[no_eof]
        pub rule global_to_relative_pos(currentpos: usize) -> Option<usize> = 
            check_global_to_relative(currentpos)
            / "[#Pos:" jump:to_num() "]" a:global_to_relative_pos(jump) {a}
            / anything:$(!("#Pos")[_]) a:global_to_relative_pos(currentpos + anything.len()){a}
            / ![_] {None}
        
        rule to_num() -> usize =
            term:$(['0'..='9']+) {term.parse().unwrap()}
        
        rule check_global_to_relative(globalPos: usize) -> Option<usize> = 
            () relativePos:position!() {?
                match globalPos.cmp(&pos){
                    std::cmp::Ordering::Less => Err("keep searching"),
                    std::cmp::Ordering::Equal => Ok(Some(relativePos)),
                    std::cmp::Ordering::Greater => Ok(None),
                }
            }
        
        rule check_relative_to_global(globalPos: usize) -> Option<usize> = 
            () relativePos:position!() {?
                match relativePos.cmp(&pos){
                    std::cmp::Ordering::Less => Err("keep searching"),
                    std::cmp::Ordering::Equal => Ok(Some(globalPos)),
                    std::cmp::Ordering::Greater => Ok(Some(globalPos)),//This should only occur with "#Pos:" chicanery
                }
            }

        #[no_eof] 
        pub rule relative_to_global_pos(currentpos: usize) -> Option<usize> = 
            check_relative_to_global(currentpos)
            / "[#Pos:" jump:to_num()"]" a:relative_to_global_pos(jump) {a}
            / anything:$(!("[#Pos")[_]) a:relative_to_global_pos(currentpos + anything.len()){a}
            / ![_] {None}

    }
}


fn global_to_relative_pos(variant: &SqFileVariant, pos: usize) -> Option<usize>{
    pos_conversion::global_to_relative_pos(variant, pos, 0).unwrap()
}

fn relative_to_global_pos(variant: &SqFileVariant, pos: usize) -> Option<usize>{
    pos_conversion::relative_to_global_pos(variant, pos, 0).unwrap()
}



#[cfg(test)]
#[test]
fn test_positions_match(){
    let input = r#"#if SERVER
    this should change the first line position
    #endif
    //This is a comment

    #if SERVER
    this is some server text
    #endif

    #if CLIENT
    this is some client text
    #else
    this is some not client text
    #endif
    
    #if !MP
    this is some impossible text
    #endif
    hi
    #if SOME_VALUE
    this is a random conditional
    #elseif SOME_OTHER_VALUE
    this is a different value
    #endif
    "#;
    let file_info = FileInfo::new("test_extract_conditions".to_string(), PathBuf::from("test"), "(SERVER || CLIENT) && MP".to_string(), FileType::RSquirrel);
    file_info.set_text(input.to_string());
    let result = get_file_varaints(file_info);
    //println!("{:?}", result);

    let global_convert = global_to_relative_pos(&result[0], 250);
    println!("global pos 250 in state {:?} is {:?} relative pos",result[0].state.identifier() , global_convert);

    let relative_convert = relative_to_global_pos(&result[0], global_convert.unwrap());
    println!("relative pos {:?} in state {:?} is {:?} global pos",global_convert, result[0].state.identifier() , relative_convert);
    assert_eq!(250, relative_convert.unwrap());
    let mut count = 0;
    for char in result[0].text().chars(){
        let relative_convert = relative_to_global_pos(&result[0], count);
        println!("{:?} : {:?}", char, input.chars().nth(relative_convert.unwrap()).unwrap());
        count += 1
    }
}

#[test]
fn no_branches(){
    let input = "#if SERVER || CLIENT || MP
    hi
    #endif";
    //Test run_on filtering
    let file_info = FileInfo::new("test_extract_conditions".to_string(), PathBuf::from("test"), "SERVER && MP".to_string(), FileType::RSquirrel);
    file_info.set_text(input.to_string());
    let result = get_file_varaints(file_info);
    println!("{:?}", result);
    assert_eq!(result.len(), 1);
}

#[test]
fn if_elseif(){
    let input = "#if SERVER || CLIENT || MP
    hi
    #else if SERVER
    hi2
    #endif";
    //Test run_on filtering
    let file_info = FileInfo::new("test_extract_conditions".to_string(), PathBuf::from("test"), "SERVER || CLIENT || UI".to_string(), FileType::RSquirrel);
    file_info.set_text(input.to_string());
    let result = get_file_varaints(file_info);
    println!("{:?}", result);
    //Maybe we can minimize this, *technically* the MP variant is irrelevant here
    assert_eq!(result.len(), 6);
}

#[test]
fn nested(){
    let input = "#if SERVER || CLIENT || MP
    hi
    #if SERVER
    hi2
    #endif
    #endif";
    //Test run_on filtering
    let file_info = FileInfo::new("test_extract_conditions".to_string(), PathBuf::from("test"), "SERVER || CLIENT || UI".to_string(), FileType::RSquirrel);
    file_info.set_text(input.to_string());
    let result = get_file_varaints(file_info);
    println!("{:?}", result);
    assert_eq!(result.len(), 6);
}

#[test]
fn if_statement(){
    let input = "#if SERVER || CLIENT
    hi
    #endif";
    //Test run_on filtering
    let file_info = FileInfo::new("test_extract_conditions".to_string(), PathBuf::from("test"), "SERVER || CLIENT || UI".to_string(), FileType::RSquirrel);
    file_info.set_text(input.to_string());
    let result = get_file_varaints(file_info);
    println!("{:?}", result);
    assert_eq!(result.len(), 3);
}


#[test]
fn big_file_parses(){
    //Due to the size of the file hardcording the right answer isnt practical, this is just to test that things look "about right"
    //let input = read_to_string("../test/SingleFile.nut").unwrap();
    let file_info = FileInfo::new("SingleFile.nut".to_string(), PathBuf::from("../test/SingleFile.nut"), "MP || UI".to_string(), FileType::RSquirrel);
    let result = get_file_varaints(file_info);
    println!("{:?}", result.len());
    for variant in &result {
        println!("{:?}", variant.state);
    }
    //CLIENT, UI, SP, SERVER
    assert!(result.len() == 8);
}
