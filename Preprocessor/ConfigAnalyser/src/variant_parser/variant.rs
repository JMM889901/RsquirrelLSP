use core::panic;
use std::{cmp::{max, min}, sync::{Arc, RwLock}, time::Duration};

use peg::{str::LineCol, Parse, ParseElem, ParseLiteral, ParseSlice, RuleResult};
use ConfigPredictor::state::{Evaluation, SqCompilerState};
use PreprocessorParser::ast::{If, Node, AST};

use crate::variant_parser::parse::{self, parse_element::find_next_valid_text_block_internal, sort};

use super::parse::parse_element::find_next_valid_text_block;

//This is meant to emulate a string for the purposes of parsing and avoiding having to clone the text file for every condition
#[derive(Debug)]//Cloning bad, it shouldnt
pub struct SqFileVariant{
    pub state: SqCompilerState,
    pub(crate) content: Arc<Node>, //AST may not be exposed to the squirrel parser, i want code seperation
    //pub time_wasted: RwLock<Duration>
}
impl Parse for SqFileVariant{
    type PositionRepr = <str as Parse>::PositionRepr;

    fn start<'input>(&'input self) -> usize {
        find_next_valid_text_block(self.content.ast.get_nodes(), 0, &self.state).unwrap()
    }

    fn is_eof<'input>(&'input self, p: usize) -> bool {
        self.content.range.1 <= p
    }

    fn position_repr<'input>(&'input self, p: usize) -> Self::PositionRepr {
        match Self::to_linecol_stateless(&self.content, &self.state, p, LineCol{line:0,column:0,offset:0}){
            Err(a) | Ok(a) => a//This is bad, but i honestly cba
        }
    }
}

impl<'input> ParseSlice<'input> for SqFileVariant {
    type Slice = String;

    fn parse_slice(&'input self, p1: usize, p2: usize) -> Self::Slice {
        //self.content.parse_slice(p1, p2)
        let result = parse_slice(&self.content, &self.state, p1, p2);
        if let Some(a) = result{
            return a.clone()
        } else {
            panic!("Failed to parse slice from {} to {} in structure", p1, p2)
        }
    }
}
//Where on earth should this function even go? Compiler state? ast? here? i do not know
pub fn parse_slice(data_structure: &Node, state: &SqCompilerState, p1: usize, p2: usize) -> Option<String>{
    match &data_structure.ast{
        AST::If(ifs) => {
            let mut asts: Vec<Node> = Vec::new();
            for r#if in ifs{
                //println!("IF: Start: {:?} End: {:?}", r#if.get_startpos(), r#if.get_endpos());
                if p2 <= r#if.get_endpos(){
                    asts.extend(r#if.get_nodes().clone());
                    break;
                }
                if p1 >= r#if.get_startpos(){
                    asts.extend(r#if.get_nodes().clone());
                }
            }
            //asts.sort_by(|a, b| sort(a, b.range.1));
            if asts.is_empty(){
                println!("No asts found in if {:?} for range {:?}", ifs, (p1, p2));
                return None
            }
            let node = asts.binary_search_by(|x| sort(x, p1));
            //println!("Found node {:?} in {:?}", node, asts);
            if let Ok(node) = node{
                let ast = asts.get(node).unwrap();
                let text = parse_slice(ast, state, p1, min(ast.range.1, p2)).unwrap();
                return Some(text)
            } else {
                return None
            }

            //let start = asts.binary_search_by(|x| sort(x, p1));
            //let end = asts.binary_search_by(|x| sort(x, p2));
            //println!("Start: {:?} End: {:?}", start, end);
            //if start == end{
            //    if let Ok(elem) = start{
            //        let ast = asts.get(elem).unwrap();
            //        return Some(parse_slice(ast, state, p1, p2).unwrap())
            //    }
            //} else if let Ok(start) = start{
            //    let ast = asts.get(start).unwrap();
            //    let text1 = parse_slice(ast, state, p1, ast.range.1).unwrap();
            //    if let Ok(end) = end{
            //        let ast2 = asts.get(end).unwrap();
            //        let text2 = parse_slice(ast2, state, ast2.range.0, p2).unwrap();
            //        let text = format!("{}{}", text1, text2);
            //        return Some(text)
            //    } else {
            //        //range is outside of acceptable, find next valid position
            //        let end_block_1 = ast.range.1;
            //        let remaining = p2 - end_block_1;
            //        let result = find_next_valid_text_block(&asts, start+1, state);
            //        if let Some(result) = result {
            //            let as2 = asts.get(result).unwrap();
            //            let part2_start = as2.range.0;
            //            let part2_end = part2_start + remaining;
            //            let text2 = parse_slice(as2, state, part2_start, part2_end).unwrap();
            //            let text = format!("{}{}", text1, text2);
            //            return Some(text)
            //        } else{
            //            panic!("Failed to find next valid text block after {:?}", ast.range.1)
            //        }
            //    }
            //}
            return None
        }
        AST::Text(text) => {
            let start = data_structure.range.0;
            let end = data_structure.range.1;
            let fixed_start = max(start, p1);
            let fixed_end = min(end, p2);
            if fixed_start >= fixed_end {
                return None
            }
            return Some(text[fixed_start-start..fixed_end-start].to_string())
        }
        AST::RunOn(asts, _) => {
            let mut start = p1;
            let mut end = p2;
            let mut buffer = String::new();
            println!("Start: {:?} End: {:?}", start, end);

            for ast in asts{
                //println!("AST: {:?}", ast);
                let ast_start = find_next_valid_text_block_internal(ast, state);
                if ast_start.is_none(){
                    println!("Failed to find next valid text block in {:?}", ast);
                    continue
                }
                let ast_start = ast_start.unwrap();
                let jump = (ast_start - ast.range.0);
                //let ast_end = ast.range.1 + jump;
                println!("Jump: {:?} Start: {:?} AST_start: {:?}", jump, start, ast_start);
                start += jump;
                end += jump;
                if ast_start > start{
                    //println!("Skipping text block in {:?}", ast);
                    continue
                }
                //if ast_start > start{
                //    //We assume there is a text skip here
                //    if let Some(newstart) = newstart{
                //        let remaining = ast_end - ast_start;
                //        let new_end = newstart + remaining;
                //        start = newstart;
                //        end = new_end;
                //        println!("Found new start: {:?} End: {:?}", newstart, new_end);
                //    } else {
                //        panic!("Failed to find next valid text block after {:?}", ast_start)
                //    }
                //    continue;
                //}
                let text = parse_slice(ast, state, start, end).unwrap_or("".to_string());
                let size = text.len();
                if size == 0{
                    //println!("Found empty text block in {:?}", ast);
                    continue
                }
                println!("Found text block {:?} with size {:?}", text, size);
                start += size;
                buffer.push_str(&text);
                if start >= end{
                    //println!("Found end of text block in {:?}", ast);
                    break
                }
            }
            println!("Buffer: {:?}, final range {:?}, full range {:?}", "", (start, end), (p1, p2));
            if buffer.is_empty(){
                panic!("Buffer: {:?}, final range {:?}, full range {:?}", buffer, (start, end), (p1, p2));
                return None
            } else {
                return Some(buffer)
            }
        }
    }

}
pub fn ast_to_string(data_structure: &Node, state: &SqCompilerState) -> String{
    match &data_structure.ast{
        AST::If(vec) => {
            if_to_string(vec, state, data_structure.range.1)
        },
        AST::Text(text) => return format!("[#Pos:{}]{}", data_structure.range.0, text),
        AST::RunOn(vec, condition) => {//We will just assume runon passes, if it doesnt thats a predictor issue
            return vec.iter().map(|x| ast_to_string(x, state)).collect()
        },
    }
}
pub fn if_to_string(vec: &Vec<If>, state: &SqCompilerState, pos: usize) -> String{
    for conditional in vec{
        match conditional{
            PreprocessorParser::ast::If::If(condition, vec) => {
                if state.evaluate_condition(condition) == Evaluation::Pass{
                    return vec.iter().map(|x| ast_to_string(x, state)).collect()
                }
            }
            PreprocessorParser::ast::If::Else(vec) => {
                return vec.iter().map(|x| ast_to_string(x, state)).collect()
            },
        }
    }
    return "".to_string()//format!("#Pos:{:?}", pos)
}

impl SqFileVariant{
    pub fn generate(data_structure: &Node, state: SqCompilerState) -> Self{
        let data_structure = Arc::new(data_structure.clone());
        return SqFileVariant{
            content: data_structure,
            state,
            //time_wasted: RwLock::new(Duration::from_secs(0))
        }
    }
    pub fn text(&self) -> String{
        return ast_to_string(&self.content, &self.state)
    }
    pub fn stateless(text: String) -> Self{
        return SqFileVariant{
            content: Arc::new(Node::new((0, text.len()), AST::Text(text))),
            state: SqCompilerState::empty()
        }
    }
}