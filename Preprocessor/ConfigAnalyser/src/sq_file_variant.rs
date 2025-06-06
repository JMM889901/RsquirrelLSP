use std::{sync::RwLock, time::{Duration, SystemTime}};

use peg::{Parse, ParseElem, ParseLiteral, ParseSlice, RuleResult};
use ConfigPredictor::state::{Evaluation, SqCompilerState};
use PreprocessorParser::ast::{If, Node, AST};

//This is a seperate struct so that I have the option of doing a custom parsing implementation, this could solve some problems later but its alot of (probably unneeded) work
#[derive(Debug)]//Cloning bad, it shouldnt
pub struct SqFileVariant{
    pub state: SqCompilerState,
    content: String, //If i can store the AST here instead of the string i can avoid a lot of unwanted copies
    //pub time_wasted: RwLock<Duration>
}
impl Parse for SqFileVariant{
    type PositionRepr = <str as Parse>::PositionRepr;

    fn start<'input>(&'input self) -> usize {
        self.content.start()
    }

    fn is_eof<'input>(&'input self, p: usize) -> bool {
        self.content.is_eof(p)
    }

    fn position_repr<'input>(&'input self, p: usize) -> Self::PositionRepr {
        self.content.position_repr(p)
    }
}
impl ParseLiteral for SqFileVariant{
    fn parse_string_literal(&self, pos: usize, literal: &str) -> RuleResult<()> {
        self.content.parse_string_literal(pos, literal)
    }
}
impl<'input> ParseElem<'input> for SqFileVariant{
    type Element = <str as ParseElem<'input>>::Element;

    fn parse_elem(&'input self, pos: usize) -> RuleResult<Self::Element> {
        //let time = SystemTime::now();

        let a= self.content.parse_elem(pos);
        //let time2 = SystemTime::now();
        //let mut write = self.time_wasted.write().unwrap();
        //*write = write.checked_add(time2.duration_since(time).unwrap()).unwrap();
        a
    }
}
impl<'input> ParseSlice<'input> for SqFileVariant {
    type Slice = <str as ParseSlice<'input>>::Slice;

    fn parse_slice(&'input self, p1: usize, p2: usize) -> Self::Slice {
        self.content.parse_slice(p1, p2)
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

impl SqFileVariant{
    pub fn generate(data_structure: &Node, state: SqCompilerState) -> Self{
        return SqFileVariant{
            content: ast_to_string(data_structure, &state),
            state,
            //time_wasted: RwLock::new(Duration::from_secs(0))

        }
    }
    pub fn from_text(text: String, state: SqCompilerState) -> Self{
        return SqFileVariant{
            content: text,
            state,
            //time_wasted: RwLock::new(Duration::from_secs(0))
        }
    }
    pub fn text(&self) -> &String{
        return &self.content
    }
    pub fn to_text(self) -> String{
        return self.content
    }
    pub fn stateless(text: String) -> Self{
        return SqFileVariant{
            content: text,
            state: SqCompilerState::empty()
        }
    }
}