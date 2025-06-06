use std::{cmp::min_by, collections::{btree_set::Difference, HashMap}, hash::Hash};

use analysis_runner::state_resolver::CompiledState;
use common::FileInfo;
use ASTParser::ast::{Element, AST};
use ConfigPredictor::state::SqCompilerState;


pub mod spanning_search;
pub mod variable;
pub mod modjson;




#[derive(Debug, Clone, PartialEq)]
enum MergeResults{
    Merged(CompiledState),//The 2 conditions are now one
    CancelledElem(CompiledState, CompiledState), //One element was cancelled out
    Consumed{
        consumed: CompiledState,
        remaining: CompiledState,
    },
    Cancelled,//Both conditions merged perfectly, IE !A && A
    None,//This arguably should return the original inputs, but its technically more memory efficient to just return none
}
//This has all been moved to analysis_runner