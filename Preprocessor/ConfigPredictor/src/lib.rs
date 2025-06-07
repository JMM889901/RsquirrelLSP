
use state::{Evaluation, SqCompilerState};
use PreprocessorParser::ast::{AST};
use PreprocessorParser::condition::Condition;
pub mod state;
pub fn get_states(ast: &AST) -> Vec<SqCompilerState>{
    get_states_dirty(ast)
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
    //Caution: Super duper hacky and slow, but this is a fraction of the speed anyways
    if !vms.is_empty(){
        vms.clear();
        vms.push("SERVER".to_string());
        vms.push("CLIENT".to_string());
        vms.push("UI".to_string());
    }//This forces it to handle all 3 vms
    //*VERY BAD*/
    for vm in vms{
        let mut permutation = SqCompilerState::one(vm.clone(), true);
        permutation.insert_terms(Condition::get_impossible_conditions(&vm), false);
        permutations.push(permutation);
    };
    //println!("Permutations: {:?}", permutations);
    

    let condition_permutations = get_condition_permutations(conditions_dedup);
    if permutations.is_empty(){
        return filter_acceptable_states(ast.get_run_on().unwrap(), condition_permutations)
    }
    let mut result: Vec<SqCompilerState> = permutations.iter().flat_map(|permutation| {
        condition_permutations.iter().map(|condition_permutation| {
            permutation.clone().merge(condition_permutation)
        })
    }).collect();

    if result.is_empty(){
        result = permutations;
    }
    filter_acceptable_states(ast.get_run_on().unwrap(), result)
}

pub fn get_condition_permutations(conditions: Vec<String>) -> Vec<SqCompilerState> {//Handle runOn conditions, probably just filter conditions that conflict idk
    if let Some((head, tail)) = conditions.split_first(){
        //println!("head{:?}", head);
        let tail_permutations = get_condition_permutations(tail.to_vec());
        let and_permutation = SqCompilerState::one(head.to_string(), true);
        let not_permutation = SqCompilerState::one(head.to_string(), false);

        tail_permutations.iter().flat_map(|tail_permutation| {
            vec![and_permutation.merge(tail_permutation), not_permutation.merge(tail_permutation)]
        }).collect()
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
    filtered_conditions
}

#[cfg(test)]
#[test]
fn one_non_vm(){
    let runon = Condition::Term("MP".to_string());
    let mut state = SqCompilerState::one("MP".to_string(), true);
    let mut badstate = SqCompilerState::one("MP".to_string(), false);
    //Test that run_on MP will NOT result in MP and !MP as runon is mandatory
    let result = filter_acceptable_states(runon, vec![state.clone(), badstate.clone()]);
    assert_eq!(result.len(), 1);
    println!("Result: {:?}", result);
}

#[test]
fn filter_or(){
    let mp_or_ui = Condition::Or(Box::new(Condition::Term("MP".to_string())), Box::new(Condition::Term("UI".to_string())));
    let mut not_ui_but_mp = HashMap::new();
    not_ui_but_mp.insert("MP".to_string(), true);
    not_ui_but_mp.insert("UI".to_string(), false);
    
    let mut not_ui_but_mp = SqCompilerState(not_ui_but_mp);

    let result = filter_acceptable_states(mp_or_ui, vec![not_ui_but_mp]);
    assert_eq!(result.len(), 1);
    println!("Result: {:?}", result);
}

#[cfg(test)]
#[test]
fn or(){
    let server_or_client = Condition::Or(Box::new(Condition::Term("SERVER".to_string())), Box::new(Condition::Term("CLIENT".to_string())));
    let ast = AST::RunOn(vec![], server_or_client);
    let states = get_states(&ast);
    println!("States: {:?}", states);
    //These states contradict so 10, 01
    assert_eq!(states.len(), 2);
    let mp_or_ui = Condition::Or(Box::new(Condition::Term("MP".to_string())), Box::new(Condition::Term("UI".to_string())));
    let ast = AST::RunOn(vec![], mp_or_ui);
    let states = get_states(&ast);
    println!("States: {:?}", states);
    //These states dont contradict so 11, 10, 01, 00
    assert_eq!(states.len(), 4);
}