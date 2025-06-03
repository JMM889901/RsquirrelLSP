use std::{fs::{self, read_to_string}, sync::Arc};

use variant_parser::{variant::SqVariant, variant_inout, variant_inout_standard};
use ConfigPredictor::get_states;
use PreprocessorParser::*;
use sq_file_variant::SqFileVariant;
mod variant_parser;
pub mod sq_file_variant;
pub fn get_file_varaints(input: &String, run_on: &String, name: &String) -> Vec<SqFileVariant>{
    let node = parse_file(input, run_on);
    let states = get_states(&node.ast);
    println!("states");
    for state in &states{
        println!("{:?}", state.identifier());
        println!("{:?}\n\n", state)
    }
    let variants:Vec<SqFileVariant> = states.into_iter().map(|x| SqFileVariant::generate(&node, x)).collect();
    

    for variant in &variants {
        fs::create_dir_all(format!("./preprocessed{}", variant.state.to_path()));

        let file = format!("./preprocessed{}/{}.pnut", variant.state.to_path(), name);
        let text = variant.text();
        fs::write(file, text).expect("Unable to write file");

    }
    return variants;
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
fn test_extract_conditions(){
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
    let result = get_file_varaints(&input.to_string(), &"(SERVER || CLIENT) && MP".to_string(), &"simple_file".to_string());
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

}

#[test]
fn if_elseif(){

}

#[test]
fn nested(){
    
}

#[test]
fn if_statement(){

}


#[test]
fn big_file_parses(){
    //Due to the size of the file hardcording the right answer isnt practical, this is just to test that things look "about right"
    let input = read_to_string("../test/SingleFile.nut").unwrap();
    let result = get_file_varaints(&input.to_string(), &"MP || UI".to_string(), &"_items".to_string());
    println!("{:?}", result.len())
}


