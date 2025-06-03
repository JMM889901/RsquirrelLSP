use std::{sync::Arc, usize};

use peg::{str::LineCol, Parse};
use ConfigPredictor::state::{Evaluation, SqCompilerState};
use PreprocessorParser::ast::{If, Node, AST};


use super::variant::SqFileVariant;
//The basic premise here is that all parsing is done using global positions, thanks to beg being surprisingly good with custom parsers, we can freely set the next position
//each time one is checked, so instead of just going 1,2,3 etc. we check to see if we have reached the end of the current text block, and if so find the next "valid" one 
//for this config, and skip to it, IE 1,2,3,6,7,8
//This means that positions returned by calculate_next_position, and as such any AST constructed using the position!() macro (which is all of them) will use the "global" position of that element
//essentially making conversion between relative locations and global locations not actually be a conversion at all
impl SqFileVariant{
    pub fn to_linecol_stateless(node: &Node, state: &SqCompilerState, target_pos: usize, current_pos: LineCol) -> Result<LineCol, LineCol>{
        //>stateless
        //>look inside function definition
        //>state
       //this function does not skip
       let mut nodes: Vec<&Node> = vec![];
        match &node.ast{
            PreprocessorParser::ast::AST::If(vec) => {
                nodes = vec.iter().flat_map(|x| x.get_nodes()).collect();
            },
            PreprocessorParser::ast::AST::Text(text) => {
                //Currentpos is calculated which causes strange behaviour here
                let (startpos, endpos) = node.range;
                if endpos > target_pos{
                    let limit = target_pos - startpos;
                    let posrep = text.position_repr(limit); //This feels a bit sketchy
                    let newpos = LineCol{
                        line: current_pos.line + posrep.line,
                        column: posrep.column, //redundant
                        offset: startpos + posrep.offset,
                    };
                    return Ok(newpos);
                } else {
                    let line = text.as_bytes().iter().filter(|&&c| c == b'\n').count();
                    return Err(LineCol{
                        line: current_pos.line + line,
                        column: current_pos.column, //redundant
                        offset: endpos, //This might need to use the node instead of calculated offsets
                    })
                }
                todo!()
            },
            PreprocessorParser::ast::AST::RunOn(vec, condition) => {
                nodes = vec.iter().collect();//bad and not good
            },
        }
        let mut pos = current_pos;
        for node in nodes{
            let result = SqFileVariant::to_linecol_stateless(node, state, target_pos, pos);
            match result{
                Ok(_) => return result,
                Err(a) => {
                    pos = a;
                },
            }
        }
        return Err(pos)
    }
}

