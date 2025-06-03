//This could be extended to implement parse, which would allow me to essentially parse this AST with a given compiler config, 
//this would allow me to avoid creating a "copy" of the file for each outcome

use peg::{str::LineCol, Parse};
use super::*;

#[derive(Clone, PartialEq, Debug)]
pub struct Node{
    pub range: (usize, usize),
    pub ast: AST
}
impl Node{
    pub fn new(range: (usize, usize), ast: AST) -> Self{
        return Node { range, ast }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum AST{
    If(Vec<If>),
    Text(String),
    RunOn(Vec<Node>, Condition) //Wrapper for including runOnData
    //There can only ever be one per input since this isnt from the text
}
impl AST{
    pub fn get_run_on(&self) -> Option<Condition>{
        //Ok this is lazy, but if i cannot see any way that a subvalue would be runon, so just assume this is called on the runon
        match self{
            Self::RunOn(_, b) => Some(b.clone()),
            _ => None
        }
    }
    ///Traverse the syntax tree and grab all decisions, includes duplicates
    pub fn get_decisions(&self) -> Vec<Condition>{//This is used in predictor, but i cant implement it as a method there
        match self{
            AST::If(vec) => vec.iter().flat_map(|x| x.get_decisions()).collect(),
            AST::Text(_) => vec![],
            AST::RunOn(vec, condition) => return vec![vec![condition.clone()], vec.iter().flat_map(|x| x.ast.get_decisions()).collect()].concat(),
        }
    }
    pub fn get_nodes(&self) -> &Vec<Node>{
        match self{
            AST::RunOn(vec, cond) =>{
                return vec
            }
            _ => todo!()
        }
    }

}

#[derive(Clone, PartialEq, Debug)] //TODO: Literally why are these seperate? just call the second if block or smthn, this is confusing
pub enum If{
    If(Condition, Vec<Node>),
    Else(Vec<Node>),
}
impl If{
    pub fn get_decisions(&self) -> Vec<Condition>{
        match self{
            If::If(condition, vec) => return vec![vec![condition.clone()], vec.iter().flat_map(|x| x.ast.get_decisions()).collect()].concat(),
            If::Else(vec) => vec.iter().flat_map(|x| x.ast.get_decisions()).collect(),
        }
    }
    pub fn get_nodes(&self) -> &Vec<Node>{
        match self{
            If::If(condition, vec) => return vec,
            If::Else(vec) => return vec,
        }
    }
    pub fn get_endpos(&self) -> usize{
        match self{
            If::If(_, vec) | If::Else(vec) => {
                return vec.last().unwrap().range.1 //bad
            },
        }
    }
    pub fn get_startpos(&self) -> usize{
        match self{
            If::If(_, vec) | If::Else(vec) => {
                return vec.first().unwrap().range.0 //bad
            },
        }
    }
}