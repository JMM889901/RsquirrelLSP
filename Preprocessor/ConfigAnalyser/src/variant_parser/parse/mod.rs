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
pub(crate) fn sort(x: &Node, target_pos: usize) -> Ordering{ //somehow this is faster than sort2, like 3 times faster, not sure why though
    if x.range.1 > target_pos{
        if x.range.0 > target_pos{
            return std::cmp::Ordering::Greater
        }
        return std::cmp::Ordering::Equal
    }
    std::cmp::Ordering::Less
}

#[cfg(test)]
#[test]
fn benchmark_sort(){
    use PreprocessorParser::ast::AST;
   
    let list: std::ops::RangeInclusive<usize> = 1..=100000;
    let nodes = list.clone().step_by(10).map(|x| Node::new((x.clone(), x + 10), AST::Text("".to_string())));
    let list = list.collect::<Vec<usize>>();
    
    let nodes = nodes.collect::<Vec<Node>>();
    let mut sort1duration = 0;
    for target in &list{
        //Do a binary serach
        let sort1time = std::time::Instant::now();
        let result = nodes.binary_search_by(|x| sort(x, *target));
        sort1duration += sort1time.elapsed().as_nanos();
    }
    let mut sort2duration = 0;
    for target in &list{
        //Do a binary serach
        let sort2time = std::time::Instant::now();
        let result = nodes.binary_search_by(|x| sort2(x, *target));
        sort2duration += sort2time.elapsed().as_nanos();
    }
    let mut sort3duration = 0;
    for target in &list{
        //Do a binary serach
        let sort3time = std::time::Instant::now();
        let result = nodes.binary_search_by(|x| sort3(x, *target));
        sort3duration += sort3time.elapsed().as_nanos();
    }
    println!("sort1: {}ns, sort2: {}ns, sort3: {}ns", sort1duration, sort2duration, sort3duration);
}