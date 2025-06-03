use std::{sync::{Arc, RwLock}, time::Duration};

use peg::{str::LineCol, Parse, ParseElem, ParseLiteral, ParseSlice, RuleResult};
use ConfigPredictor::state::{Evaluation, SqCompilerState};
use PreprocessorParser::ast::{If, Node, AST};

use super::parse::parse_element::find_next_valid_text_block;

//This is meant to emulate a string for the purposes of parsing and avoiding having to clone the text file for every condition
#[derive(Debug)]//Cloning bad, it shouldnt
pub struct SqVariant{
    pub state: SqCompilerState,
    pub(crate) content: Arc<Node>, //AST may not be exposed to the squirrel parser, i want code seperation
    pub time_wasted: RwLock<Duration>
}
impl Parse for SqVariant{
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

impl<'input> ParseSlice<'input> for SqVariant {
    type Slice = <str as ParseSlice<'input>>::Slice;

    fn parse_slice(&'input self, p1: usize, p2: usize) -> Self::Slice {
        //self.content.parse_slice(p1, p2)
        todo!()
    }
}
//Where on earth should this function even go? Compiler state? ast? here? i do not know
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

impl SqVariant{
    pub fn generate(data_structure: Arc<Node>, state: SqCompilerState) -> Self{
        return SqVariant{
            content: data_structure,
            state,
            time_wasted: RwLock::new(Duration::from_secs(0))
        }
    }
    pub fn text(&self) -> String{
        return ast_to_string(&self.content, &self.state)
    }
}