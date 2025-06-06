use std::{clone, collections::{HashMap, HashSet}, fmt, hash::Hash};

use PreprocessorParser::condition::Condition;

#[derive(Clone, Debug, PartialEq)]
pub enum Evaluation{
    Pass,
    Neutral,
    Fail
}

#[derive(Clone, Debug, PartialEq, Eq)]//This probably shouldnt be clonable, not that it *hugely* matters
pub struct SqCompilerState(pub HashMap<String, bool>);//TODO: I should really just build an object for each condition, string comparison is expensive (but like, not THAT expensive)

impl SqCompilerState{
    pub fn has_multiple_states(&self) -> bool{
        let mut vm = None;
        for (key, value) in self.0.iter(){
            if Condition::is_vm(key) && *value{
                if let Some(vm) = vm{
                    if vm != key{
                        return true
                    }
                } else{
                    vm = Some(key);
                }
            }
        }
        false
    }
    pub fn identifier(&self) -> String{

        let mut vec: Vec<(String, bool)> = self.0.iter().map(|(x,y)| (x.clone(),y.clone())).collect();
        vec.sort();
        vec.iter().map(|(x, y)| {
            vec![ "-".to_string(), if !*y{
                format!("!{}", x)
            } else {
                x.clone()
            }].concat()
        }).collect()
    }

    pub fn to_path(&self) -> String{

        let mut vec: Vec<(String, bool)> = self.0.iter().map(|(x,y)| (x.clone(),y.clone())).collect();
        vec.sort();
        vec.iter().map(|(x, y)| {
            vec![ "/".to_string(), if !*y{//Holy shit i am dumb
                format!("!{}", x)
            } else {
                x.clone()
            }].concat()
        }).collect()
    }

    pub fn one(elem: String, bool: bool) -> Self{
        let mut map = HashMap::new();
        map.insert(elem, bool);
        return SqCompilerState(map);
    }
    pub fn insert_terms(&mut self, elems: Vec<String>, bool: bool){
        for elem in elems{
            self.0.insert(elem, bool);
        }
    }
    pub fn insert_term(&mut self, elem: String, bool: bool){
        self.0.insert(elem, bool);
    }
    pub fn merge(&self, other: &Self) -> Self{
        let mut new = HashMap::new();
        for (term, value) in &self.0{
            new.insert(term.clone(), value.clone());
        }
        for (term, value) in &other.0{
            new.insert(term.clone(), value.clone());
        }
        return SqCompilerState(new);
    }
    pub fn empty() -> Self{
        return SqCompilerState(HashMap::new())
    }

    pub fn evaluate_condition(&self, condition: &Condition) -> Evaluation{
        match condition{
            Condition::And(condition, condition1) => {
                match (self.evaluate_condition(condition), self.evaluate_condition(condition1)){
                    (Evaluation::Pass, Evaluation::Pass) => Evaluation::Pass,
                    (_, Evaluation::Fail) | (Evaluation::Fail, _)=> Evaluation::Fail,
                    (_, Evaluation::Neutral) => Evaluation::Neutral,
                    (Evaluation::Neutral, _) => Evaluation::Neutral,
                }
            },
            Condition::Or(condition, condition1) => {
                match (self.evaluate_condition(condition), self.evaluate_condition(condition1)){
                    (Evaluation::Pass, _) | (_, Evaluation::Pass) => Evaluation::Pass,
                    (Evaluation::Fail, Evaluation::Fail) => Evaluation::Fail,
                    (_, Evaluation::Neutral) => Evaluation::Neutral,
                    (Evaluation::Neutral, _) => Evaluation::Neutral,
                }
            },
            Condition::Not(condition) => {
                match self.evaluate_condition(condition){
                    Evaluation::Fail => Evaluation::Pass,
                    Evaluation::Neutral => Evaluation::Neutral,
                    Evaluation::Pass => Evaluation::Fail,
                }
            },
            Condition::Term(word) => {
                match self.0.get(word){
                    Some(true) => Evaluation::Pass,
                    Some(false) => Evaluation::Fail,
                    None => Evaluation::Neutral
                }
            },
        }
    }
}
impl Hash for SqCompilerState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut vec: Vec<(String, bool)> = self.0.iter().map(|(x,y)| (x.clone(),y.clone())).collect();
        vec.sort();
        for (key, value) in vec{
            key.hash(state);
            value.hash(state);
        }
    }
}