use ast::{Node, AST};
use grammar::preprocessor_grammar;
use condition::Condition;
pub mod condition;
pub mod ast;
mod grammar;

pub fn parse_file(text: &String, run_on: &String) -> Node{
    let run_on = preprocessor_grammar::to_condition_expression(&run_on).unwrap();
    #[cfg(debug_assertions)]
    let result = preprocessor_grammar::parse_dbg(text).unwrap();
    #[cfg(not(debug_assertions))]
    let result = preprocessor_grammar::parse_rls(text).unwrap();
    return Node{
        ast: AST::RunOn(result, run_on),
        range: (0, text.len())
    }
}
pub fn parse_condition_expression(text: &String) -> Condition{
    let result = preprocessor_grammar::to_condition_expression(text).unwrap();
    return result;
}