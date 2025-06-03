use state::{Evaluation, SqCompilerState};
use PreprocessorParser::ast::{AST};
use PreprocessorParser::condition::Condition;
pub mod state;
pub fn get_states(ast: &AST) -> Vec<SqCompilerState>{
    return get_states_dirty(ast);
}

fn get_states_dirty(ast: &AST) -> Vec<SqCompilerState>{
    let all_decisions = ast.get_decisions();
    //This function is slow and dirty, goes for condition coverage and not decision
    let all_conditions = all_decisions.iter().flat_map(|x| x.get_terms());

    let mut conditions_dedup = Vec::new();
    let mut vms = Vec::new();//Should probably be a struct with VM as a field, but it technically might not be known, and i'd like to avoid
    //A struct with 2 billion fields for every mutually exclusive condition
    for condition in all_conditions {

        match condition.as_str(){
            "SERVER" | "CLIENT" | "UI" => if !vms.contains(&condition) {
                vms.push(condition.clone());
            },
            _ => if !conditions_dedup.contains(&condition) {
                conditions_dedup.push(condition);
            }
        }
    };
    let mut permutations = Vec::new();
    //exactly 1 vm will be active at a time
    for vm in vms{
        let mut permutation = SqCompilerState::one(vm.clone(), true);
        permutation.insert_terms(Condition::get_impossible_conditions(&vm), false);
        permutations.push(permutation);
    };
    

    let condition_permutations = get_condition_permutations(conditions_dedup);
    if permutations.len() == 0{
        return condition_permutations
    }
    let mut result: Vec<SqCompilerState> = permutations.iter().flat_map(|permutation| {
        condition_permutations.iter().map(|condition_permutation| {
            permutation.clone().merge(condition_permutation)
        })
    }).collect();

    if result.len() == 0{
        result = permutations;
    }
    return filter_acceptable_states(ast.get_run_on().unwrap(), result)
}

pub fn get_condition_permutations(conditions: Vec<String>) -> Vec<SqCompilerState> {//Handle runOn conditions, probably just filter conditions that conflict idk
    if let Some((head, tail)) = conditions.split_first(){
        println!("head{:?}", head);
        let tail_permutations = get_condition_permutations(tail.to_vec());
        let and_permutation = SqCompilerState::one(head.to_string(), true);
        let not_permutation = SqCompilerState::one(head.to_string(), false);

        return tail_permutations.iter().flat_map(|tail_permutation| {
            vec![and_permutation.merge(tail_permutation), not_permutation.merge(tail_permutation)]
        }).collect();
    } else{
        vec![SqCompilerState::empty()]
    }
}


pub fn filter_acceptable_states(run_on: Condition, states: Vec<SqCompilerState>) -> Vec<SqCompilerState>{//This is very slow and inefficient
    let mut filtered_conditions = Vec::new();//Rather than all perms then filter to valid, probably just avoid generating invalid perms to begin with, see get_condition_permutations
    for state in states{
        if state.evaluate_condition(&run_on) == Evaluation::Pass{//Might need to be neutral? that seems wrong
            filtered_conditions.push(state);
        }
    }
    return filtered_conditions
}