use parse_element::{find_next_valid_text_block};
use peg::{ParseLiteral, RuleResult};
use variant::SqVariant;
use std::{sync::Arc, usize};

use peg::{str::LineCol, Parse};
use ConfigPredictor::state::{Evaluation, SqCompilerState};
use PreprocessorParser::ast::{If, Node, AST};

use super::*;

impl ParseLiteral for SqVariant{
    fn parse_string_literal(&self, pos: usize, literal: &str) -> RuleResult<()> {
        match self.match_string_literal(pos, literal){
            LiteralMatch::Here(a) => RuleResult::Matched(a, ()),
            LiteralMatch::Next => panic!("this is bad"),
            LiteralMatch::Failed => RuleResult::Failed,
            LiteralMatch::Nope(_) => panic!("this might be expected, remove this error if so"),
        }
    }
}
impl SqVariant{
    fn match_string_literal(&self, pos: usize, literal: &str) -> LiteralMatch{
        match_string_literal_internal(&self.content, &self.state, pos, literal)
    }
}
pub(crate) enum LiteralMatch{
    Here(usize),
    Next,
    Failed,
    Nope(usize)
}

fn match_string_literal_internal(node: &Node, state: &SqCompilerState, target_pos: usize, literal: &str) -> LiteralMatch{
    let (start, end) = node.range;
    match &node.ast{
        AST::If(ifs) => {
            let mut asts = &vec![];
            let if_block = asts.binary_search_by(|x| sort(x, target_pos));
            if let Ok(elem) = if_block{
                asts = ifs.get(elem).unwrap().get_nodes();
            } else {
                for if_block in ifs{
                    if if_block.get_endpos() > target_pos{
                        asts = if_block.get_nodes();
                        break
                    }
                }
            }
            let searched = asts.binary_search_by(|x| sort(x, target_pos));
            if let Ok(elem) = searched{
                let node = asts.get(elem).unwrap();
                let result = match_string_literal_internal(&node, state, target_pos, literal);
                match result{
                    LiteralMatch::Here(_) | LiteralMatch::Failed => return result,
                    LiteralMatch::Next => {
                        let result = find_next_valid_text_block(asts, elem+1, state);
                        if let Some(pos) = result{
                            return LiteralMatch::Here(pos)
                        } else{
                            return LiteralMatch::Next//Pass the search back up
                        }
                    },//999 is supposed to be a "we have parsed all ASTs so nothin left bud" but i doubt it will
                    LiteralMatch::Nope(a) => (),
                }
            } else {
                for (elem, node) in asts.iter().enumerate(){
                    let result = match_string_literal_internal(&node, state, target_pos, literal);
                    match result{
                        LiteralMatch::Here(_) | LiteralMatch::Failed => return result,
                        LiteralMatch::Next => {
                            let result = find_next_valid_text_block(asts, elem+1, state);
                            if let Some(pos) = result{
                                return LiteralMatch::Here(pos)
                            } else{
                                return LiteralMatch::Next//Pass the search back up
                            }
                        },//999 is supposed to be a "we have parsed all ASTs so nothin left bud" but i doubt it will
                        LiteralMatch::Nope(a) => (),
                    }
                }
            }
            return LiteralMatch::Nope(end)
        }
        AST::Text(text) => {
            //println!("target {} start {}, end {}, offset {}, real target {}, last_success {}", target, start, end, offset, target_pos, current_pos);
            if target_pos < start{
                panic!("target pos is {} but start is {}", target_pos, start)
            }
            if end > target_pos{
                let initial = target_pos - start;
                let l = literal.len();
                if end - start >= initial + l && &text.as_bytes()[initial..initial+l] == literal.as_bytes() {
                    if end - start > initial + l{
                        return LiteralMatch::Here(target_pos + l)
                    } else {
                        //println!("reached the end of string ending {}", end);
                        //panic!("matched");
                        return LiteralMatch::Next
                    }
                } else {
                    return LiteralMatch::Failed
                }
            } else {
                //println!("target {} not in string starting {} and ending {}", target_pos, start, end);
                return LiteralMatch::Nope(end)
            }
        },
        AST::RunOn(asts, condition) => {
            //println!("called for target {}", target_pos);
            let searched = asts.binary_search_by(|x| sort(x, target_pos));
            if let Ok(elem) = searched{
                let node = asts.get(elem).unwrap();
                let result = match_string_literal_internal(&node, state, target_pos, literal);
                match result{
                    LiteralMatch::Here(_) | LiteralMatch::Failed => return result,
                    LiteralMatch::Next => return LiteralMatch::Here(find_next_valid_text_block(asts, elem+1, state).unwrap_or(usize::MAX)),//999 is supposed to be a "we have parsed all ASTs so nothin left bud" but i doubt it will
                    LiteralMatch::Nope(_) => (),
                }
            } else {
                //println!("called for target {}", target_pos);
                for (elem, node) in asts.iter().enumerate(){
                    //println!("calling with last fail pos as {}", pos);
                    let result = match_string_literal_internal(&node, state, target_pos, literal);
                    match result{
                        LiteralMatch::Here(_) | LiteralMatch::Failed => return result,
                        LiteralMatch::Next => return LiteralMatch::Here(find_next_valid_text_block(asts, elem+1, state).unwrap_or(usize::MAX)),//999 is supposed to be a "we have parsed all ASTs so nothin left bud" but i doubt it will
                        LiteralMatch::Nope(_) => (),
                    }
                }
            }
            return LiteralMatch::Failed
        }
        _ => todo!()
    }
}

#[cfg(test)]
use PreprocessorParser::parse_file;
use ConfigPredictor::{get_states};
//This is effectively just an alternate string converter to the recursive function
//Logically it should return the same output - any [#] statements
peg::parser!{
    pub grammar variant_inout() for SqVariant{
        #[no_eof]
        pub rule out_the_in() -> String =
            "THING" rest:out_the_in() {vec!["I SAW A THING".to_string(), rest].concat()}
            / a:(!"THING" a:[_]{a})+ rest:out_the_in() {vec![a.into_iter().collect(), rest].concat()}
            / ![_] {"".to_string()}


    }
}

#[test]
fn text_parse(){
    use PreprocessorParser::condition::Condition;
    let len: usize = "123456789\n".to_string().len();
    let text = AST::Text("THING6789\n".to_string());
    let input = vec![Node::new((0, len), text.clone()), Node::new((len+3, (len*2) + 3), text.clone()),
        Node::new(((len*2)+6, (len*3) + 6), text)];
    let condition = Condition::term("DEBUG");
    let structure = Node::new((0, (len*3) + 6), AST::RunOn(input, condition));
    //println!("{}", result);
    let file = SqVariant::generate(Arc::new(structure), SqCompilerState::one("DEBUG".to_string(), true));
    let result = variant_inout::out_the_in(&file).unwrap();
    println!("{:?}", result);
    assert!(result.contains("I SAW A THING"))
    //let result = file.position_repr(15);
}