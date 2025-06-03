use peg::{ParseElem, RuleResult};
use variant::SqFileVariant;
use std::{cmp::Ordering, sync::Arc, time::SystemTime, usize};

use peg::{str::LineCol, Parse};
use ConfigPredictor::state::{Evaluation, SqCompilerState};
use PreprocessorParser::ast::{If, Node, AST};

use super::*;
impl<'input> ParseElem<'input> for SqFileVariant{
    type Element = <str as ParseElem<'input>>::Element;

    fn parse_elem(&'input self, pos: usize) -> RuleResult<Self::Element> {
        let result = self.calculate_next_position(pos);
        if let Some((elem, pos)) = result{
            return RuleResult::Matched(pos, elem)
        } else {
            RuleResult::Failed
        }
    }
}
impl SqFileVariant{
    pub fn calculate_next_position(&self, target_pos: usize) -> Option<(char, usize)>{
        //calculate_next_position_internal(&self.content, &self.state, target_pos, 0, 0).ok()
        //let time = SystemTime::now();
        let a = match calculate_next_position_internal(&self.content, &self.state, target_pos){
            NextPos::Here(a, b) => Some((a, b)),
            NextPos::Next(_) => panic!("uh oh spaghetti-ohs"),
            NextPos::Nope(_) => None,
        };
        let time2 = SystemTime::now();
        //let mut write = self.time_wasted.write().unwrap();
        //*write = write.checked_add(time2.duration_since(time).unwrap()).unwrap();
        a
    }
}
pub(crate) enum NextPos{
    Here(char, usize),
    Next(char),
    Nope(usize)
}



fn calculate_next_position_internal(node: &Node, state: &SqCompilerState, target_pos: usize) -> NextPos{
    let (start, end) = node.range;
    match &node.ast{
        AST::If(ifs) => {//This time is largely wasted, we can assume the target position is valid based on the fact it was given by find_next
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
            if let Ok(elem) = asts.binary_search_by(|x| sort(x, target_pos)){
                let node = asts.get(elem).unwrap();
                let result = calculate_next_position_internal(&node, state, target_pos);
                match result{
                    NextPos::Here(_, _) => return result,
                    NextPos::Next(a) => {
                        let result = find_next_valid_text_block(asts, elem+1, state);
                        if let Some(pos) = result{
                            return NextPos::Here(a, pos)
                        } else{
                            return NextPos::Next(a)//Pass the search back up
                        }
                    },//999 is supposed to be a "we have parsed all ASTs so nothin left bud" but i doubt it will
                    NextPos::Nope(a) => println!("called nope from {:?} with target {}", node, target_pos),
                }
            } else {
                //println!("bad and slow :(, failed to find position {}", target_pos);

                for (elem, node) in asts.iter().enumerate(){
                    if node.range.1 < target_pos{
                        continue;
                    }
                    let result = calculate_next_position_internal(&node, state, target_pos);
                    match result{
                        NextPos::Here(_, _) => return result,
                        NextPos::Next(a) => {
                            let result = find_next_valid_text_block(asts, elem+1, state);
                            if let Some(pos) = result{
                                return NextPos::Here(a, pos)
                            } else{
                                return NextPos::Next(a)//Pass the search back up
                            }
                        },//999 is supposed to be a "we have parsed all ASTs so nothin left bud" but i doubt it will
                        NextPos::Nope(a) => (),
                    }
                }
            }
            return NextPos::Nope(end)
        }
        AST::Text(text) => {
            //println!("target {} start {}, end {}, offset {}, real target {}, last_success {}", target, start, end, offset, target_pos, current_pos);
            //if target_pos < start{
            //    panic!("target pos is {} but start is {}", target_pos, start)
            //}
            //if end > target_pos{
                let limit = target_pos - start;
                match text[limit..].chars().next(){
                    Some( c) => {
                        if limit + c.len_utf8() < end - start{
                            return NextPos::Here(c, target_pos + c.len_utf8())
                        } else {
                            //println!("reached the end of string ending {}", end);
                            return NextPos::Next(c)
                        }
                    },
                    None => panic!("not sure what situation this is a problem in")
                }
            //} else {
            //    println!("target {} not in string starting {} and ending {}", target_pos, start, end);//This shouldnt happen anymore
            //    return NextPos::Nope(end)
            //}
        },
        AST::RunOn(asts, condition) => {
            //println!("called for target {}", target_pos);
            let searched = asts.binary_search_by(|x| sort(x, target_pos));
            if let Ok(elem) = searched{
                let node = asts.get(elem).unwrap();
                let result = calculate_next_position_internal(&node, state, target_pos);
                match result{
                    NextPos::Here(_, _) => return result,
                    NextPos::Next(a) => return NextPos::Here(a, find_next_valid_text_block(asts, elem+1, state).unwrap_or(usize::MAX)),//999 is supposed to be a "we have parsed all ASTs so nothin left bud" but i doubt it will
                    NextPos::Nope(a) => println!("called nope from {:?} with target {}", node, target_pos),
                }
            } else {
                //println!("bad and slow :(, failed to find position {}", target_pos);
                let iter = asts.iter().enumerate();
                for (elem, node) in iter{
                    //println!("calling with last fail pos as {}", pos);
                    if node.range.1 < target_pos{
                        continue;
                    }
                    let result = calculate_next_position_internal(&node, state, target_pos);
                    match result{
                        NextPos::Here(_, _) => return result,
                        NextPos::Next(a) => return NextPos::Here(a, find_next_valid_text_block(asts, elem+1, state).unwrap_or(usize::MAX)),//999 is supposed to be a "we have parsed all ASTs so nothin left bud" but i doubt it will
                        NextPos::Nope(a) => (),
                    }
                }
            }

            return NextPos::Nope(end)
        }
        _ => todo!()
    }
}

pub(crate) fn find_next_valid_text_block(nodes: &Vec<Node>, past: usize, state: &SqCompilerState) -> Option<usize>{
    for node in nodes.iter().skip(past){
        let result = find_next_valid_text_block_internal(node, state);
        match result{
            Some(_) => return result,
            None => ()
        }
    }
    None
}

pub(crate) fn find_next_valid_text_block_internal(node: &Node, state: &SqCompilerState) -> Option<usize>{
    match &node.ast{
        AST::If(ifs) => {
            let mut asts = &vec![];
            for if_block in ifs{
                match if_block{
                    If::If(condition, vec) => {
                        if state.evaluate_condition(&condition) == Evaluation::Pass{
                            asts = vec;
                            break
                        }
                    },
                    If::Else(vec) => {
                        asts = vec;
                        break
                    },
                }
            }
            for (elem, node) in asts.iter().enumerate(){
                let result = find_next_valid_text_block_internal(&node, state);
                match result{
                    Some(_) => return result,
                    None => ()
                }
            }
            return None
        },
        AST::Text(_) => return Some(node.range.0),
        AST::RunOn(vec, condition) => panic!("top level runOn cannot be passed here"),
    }
}


#[cfg(test)]
use PreprocessorParser::parse_file;
use ConfigPredictor::{get_states};
#[test]
fn test_skipping(){
    let input = r#"
    #if !SERVER
    what
    #endif
    #if SERVER
    no idea
    #endif"#;
    let ast = parse_file(&input.to_string(), &"DEBUG".to_string());
    let states = get_states(&ast.ast);
    let mut state = SqCompilerState::one("DEBUG".to_string(), true);
    state.insert_term("SERVER".to_string(), true);

    let variant = SqFileVariant::generate(&ast, state);
    println!("state {:?}", &variant.state);
    let result = variant_inout::out_the_in(&variant).unwrap();
    println!("{}", result);
    println!("------------");
    println!("{}", variant.text())
}

#[test]
fn test_else(){
    let input = r#"startText
    #if !SERVER
    what
    #else
    no idea
    #endif
    endText"#;
    let ast = parse_file(&input.to_string(), &"DEBUG".to_string());
    let states = get_states(&ast.ast);
    let mut state = SqCompilerState::one("DEBUG".to_string(), true);
    state.insert_term("SERVER".to_string(), true);

    let variant = SqFileVariant::generate(&ast, state);
    println!("state {:?}", &variant.state);
    let result = variant_inout::out_the_in(&variant).unwrap();
    println!("{}", result);
    println!("------------");
    println!("{}", variant.text())
}

#[test]
fn text_parse(){
    use PreprocessorParser::condition::Condition;
    let len: usize = "123456789\n".to_string().len();
    let text = AST::Text("123456789\n".to_string());
    let input = vec![Node::new((0, len), text.clone()), Node::new((len+3, (len*2) + 3), text.clone()),
        Node::new(((len*2)+6, (len*3) + 6), text)];
    let condition = Condition::term("DEBUG");
    let structure = Node::new((0, (len*3) + 6), AST::RunOn(input, condition));
    //println!("{}", result);
    let file = SqFileVariant::generate(&structure, SqCompilerState::one("DEBUG".to_string(), true));
    let result = variant_inout::out_the_in(&file).unwrap();
    println!("{:?}", result)
    //let result = file.position_repr(15);
}

//This is effectively just an alternate string converter to the recursive function
//Logically it should return the same output - any [#] statements
peg::parser!{
    pub grammar variant_inout() for SqFileVariant{
        #[no_eof]
        pub rule out_the_in() -> String =
            a:(a:[_])* {a.into_iter().collect()}
    }
}

#[test]
fn test_pos_conversions(){
    let input = r#"
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
    let ast = parse_file(&input.to_string(), &"DEBUG".to_string());
    let states = get_states(&ast.ast);
    let mut state = SqCompilerState::one("DEBUG".to_string(), true);
    state.insert_term("SERVER".to_string(), true);

    let variant = SqFileVariant::generate(&ast, state);

    let mut count = 0;
    let mut stuff = vec![];
    while let Some((elem, newpos)) = variant.calculate_next_position(count){
        //println!("{:?} : {:?}", elem, input.chars().nth(count).unwrap());
        assert_eq!(elem, input.chars().nth(count).unwrap());
        count = newpos;
        stuff.push(elem);
    }
    let string: String = stuff.iter().collect();
    println!("{}", string);
    //let time = variant.time_wasted.read().unwrap();
    //println!("Time wasted: {:?}", time);
}