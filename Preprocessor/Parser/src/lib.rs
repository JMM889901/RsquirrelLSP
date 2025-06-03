use ast::{Node, AST};
use grammar::preprocessor_grammar;
use condition::Condition;
pub mod condition;
pub mod ast;
mod grammar;

pub fn parse_file(text: &String, run_on: &String) -> Node{
    let run_on = preprocessor_grammar::to_condition_expression(&run_on).unwrap();
    let result = preprocessor_grammar::parse(text).unwrap();
    return Node{
        ast: AST::RunOn(result, run_on),
        range: (0, text.len())
    }
}