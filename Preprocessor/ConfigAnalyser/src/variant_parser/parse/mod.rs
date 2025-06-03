use std::cmp::Ordering;

use PreprocessorParser::ast::Node;

use super::*;
pub mod parse_element;
pub mod parse_literal;

fn sort2(x: &Node, target_pos: usize) -> Ordering{
    if target_pos < x.range.0{
        Ordering::Less
    } else if target_pos >= x.range.1{
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

fn sort3(x: &Node, target_pos: usize) -> Ordering{ //somehow this is faster than sort2, like 3 times faster, not sure why though
    match x.range.1.cmp(&target_pos){
        std::cmp::Ordering::Greater => {
            match x.range.0.cmp(&target_pos){
                std::cmp::Ordering::Greater => return std::cmp::Ordering::Greater,
                _ => return std::cmp::Ordering::Equal,
            }
        },
        _ => std::cmp::Ordering::Less,
    }
}
fn sort(x: &Node, target_pos: usize) -> Ordering{ //somehow this is faster than sort2, like 3 times faster, not sure why though
    if x.range.1 > target_pos{
        if x.range.0 > target_pos{
            return std::cmp::Ordering::Greater
        }
        return std::cmp::Ordering::Equal
    }
    std::cmp::Ordering::Less
}