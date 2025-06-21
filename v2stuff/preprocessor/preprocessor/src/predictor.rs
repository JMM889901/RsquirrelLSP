use core::panic;
use std::{sync::Arc, vec};

use compiler_state::{term::{arc_term::ArcTerm, CompareResult, KnownTerm}, CompilerState, CompilerTerm};
use preprocessor_parser::{parse_expression, statements::PreprocessorExpression, PreprocessedAST, PreprocessorAST};
use parse_common::parsed_element::Visitable;
use crate::Preprocessor;
use preprocessor_parser::statements::PreprocessorExpression::Identifier;
impl Preprocessor{
    pub fn get_term(&self, new_term: String) -> Arc<KnownTerm> {
        {
        if new_term.starts_with("!"){
            eprintln!("This shouldnt happen because lexer");
            panic!()//This is seperate so i can delete it later
        }
        for term in self.term_map.read().unwrap().iter() {
            if term.to_string() == new_term {
                return term.clone();
            }
        }
        }
        //Write locking then re-testing
        let mut write = self.term_map.write().unwrap();
        for term in write.iter() {
            if term.to_string() == new_term {
                return term.clone();
            }
        }
        let new = Arc::new(KnownTerm::Identifier(new_term));
        write.push(new.clone());
        new
    }


    pub fn identify_valid_states(&self, runon: &str, ast: &PreprocessedAST) -> Result<Vec<CompilerState>, String> {
        let tokens = sqparse::tokenize(runon, sqparse::Flavor::SquirrelRespawn).into_tokens();
        let res = parse_expression(&tokens).
            map_err(|e| format!("Failed to parse runon expression \"{}\": {}", runon, e))?;
        let all_states = self.identify_file_states(&res.item, ast)?;
        /*let filter_states = all_states.into_iter()
            .filter(|state| {
                evaluate_state(state, &res.item) == CompareResult::Accept
            })
            .collect::<Vec<_>>();*/
        //Cringe and bad dedup because i was a dumbass earlier
        let mut final_states = Vec::new();
        for state in all_states.into_iter() {
            if evaluate_state(&state, &res.item) == CompareResult::Accept && !final_states.contains(&state) {
                final_states.push(state);
            }
        }
        Ok(final_states)
    }

    //This is stupid, un-stupid this later ( I wont )
    //Theres no good reason to need to do this, its just because i store them as true/false rather than an enum
    pub fn get_vm_combs(&self) -> Vec<Vec<CompilerTerm>> {
        let server = vec![CompilerTerm::from_true(self.get_term("SERVER".to_string())), 
                        CompilerTerm::from_false(self.get_term("CLIENT".to_string())), CompilerTerm::from_false(self.get_term("UI".to_string()))];
        let client = vec![CompilerTerm::from_true(self.get_term("CLIENT".to_string())), 
                        CompilerTerm::from_false(self.get_term("SERVER".to_string())), CompilerTerm::from_false(self.get_term("UI".to_string()))];
        let ui = vec![CompilerTerm::from_true(self.get_term("UI".to_string())), 
                        CompilerTerm::from_false(self.get_term("CLIENT".to_string())), CompilerTerm::from_false(self.get_term("SERVER".to_string()))];
        vec![server, client, ui]
    }

    pub fn identify_file_states(&self, runon: &PreprocessorExpression, ast: &PreprocessedAST) -> Result<Vec<CompilerState>, String> {
        let mut terms = Vec::new();

        //This is bad, it means im literally doing condition coverage for terms that HAVE to be true
        runon.visit(&mut |e| {
            if let Identifier(term) = e {
                terms.push(self.get_term(term.to_string()));
            }
        }, &mut |_| true);


        for elem in ast {
            elem.item.visit(&mut |e| {
                match e {
                    PreprocessorAST::If { condition, ..} | PreprocessorAST::ElseIf { condition, ..} => {
                        condition.item.visit(&mut |e| {
                            if let Identifier(term) = e {
                                terms.push(self.get_term(term.to_string()));
                            }
                        }, &mut |_| true);
                    }
                    _ => {}
                }
            }, &mut |_| true);
        }
        terms.sort_by_key(|t| t.to_string());
        terms.dedup();
        let mut vms = Vec::new();
        let mut filtered_terms: Vec<Vec<ArcTerm>> = vec![];
        for term in terms {
            if term.is_vm() {
                /*for new in self.handle_vm_term(term.clone()) {
                    if !vms.contains(&new) {
                        vms.push(new);//TODO: This is bad and stupid, literally only need to run once but it does it every time
                        //^It shouldn't even need to run "once" really, since "once" is actually 3 times, which is tripply stupid
                    }
                }*/
                vms = self.get_vm_combs();
            } else {
                if filtered_terms.is_empty() {
                    filtered_terms = vec![vec![CompilerTerm::from_true(term.clone())], vec![CompilerTerm::from_false(term.clone())]];
                    continue;
                }
                filtered_terms = filtered_terms.into_iter()
                    .flat_map(|state| {
                        let mut new_state = state.clone();
                        new_state.push(CompilerTerm::from_true(term.clone()));
                        vec![new_state, 
                             state.into_iter().chain(vec![CompilerTerm::from_false(term.clone())]).collect()]
                    })
                    .collect();
            }
        };
        if vms.is_empty() {
            return Ok(filtered_terms.into_iter().map(CompilerState::new).collect());
        }
        if filtered_terms.is_empty() {
            return Ok(vms.into_iter().map(CompilerState::new).collect());
        }
        let mut all_states = Vec::new();
        for vm_state in vms {
            for state in &filtered_terms {
                let mut new_state = state.clone();
                new_state.extend(vm_state.clone());
                all_states.push(CompilerState::new(new_state));
            }
        }
        Ok(all_states)
    }
}


pub fn is_vm(term: &str) -> bool {
    matches!(term, "SERVER" | "CLIENT" | "UI")
}

pub fn evaluate_state(state: &CompilerState, expr: &PreprocessorExpression) -> CompareResult {
   match expr {
        PreprocessorExpression::Identifier(term) if is_vm(term) => {
            let res = state.cmp_string(term);
            if res == CompareResult::DontCare {
                let others = match term.as_str() {
                    "SERVER" => ("CLIENT", "UI"),
                    "CLIENT" => ("SERVER", "UI"),
                    "UI" => ("SERVER", "CLIENT"),
                    _ => panic!("This should be a vm term, but apparently isnt? tf"),
                };
                return match (state.cmp_string(others.0), state.cmp_string(others.1)) {
                    (CompareResult::Reject, CompareResult::Reject) => CompareResult::Accept,
                    (CompareResult::Accept, _) | (_, CompareResult::Accept) => CompareResult::Reject,
                    _ => CompareResult::DontCare,
                };
            }
            res

        },
        PreprocessorExpression::Identifier(term) => {
            state.cmp_string(term)
        },
        PreprocessorExpression::Not(expr) => {
            let result = evaluate_state(state, &expr.item);
            match result {
                CompareResult::Accept => CompareResult::Reject,
                CompareResult::Reject => CompareResult::Accept,
                _ => result,
            }
        },
        PreprocessorExpression::And(left, right) => {
            let left_result = evaluate_state(state, &left.item);
            if left_result == CompareResult::Reject {
                return CompareResult::Reject;
            }
            let right_result = evaluate_state(state, &right.item);
            if right_result == CompareResult::Reject {
                return CompareResult::Reject;
            }
            if left_result == CompareResult::Accept && right_result == CompareResult::Accept {
                CompareResult::Accept
            } else {
                CompareResult::DontCare
            }
        },
        PreprocessorExpression::Or(left, right) => {
            let left_result = evaluate_state(state, &left.item);
            if left_result == CompareResult::Accept {
                return CompareResult::Accept;
            }
            let right_result = evaluate_state(state, &right.item);
            if right_result == CompareResult::Accept {
                return CompareResult::Accept;
            }
            if left_result == CompareResult::Reject && right_result == CompareResult::Reject {
                CompareResult::Reject
            } else {
                CompareResult::DontCare
            }
        },
        PreprocessorExpression::Bracketed(expr) => {
            evaluate_state(state, &expr.item)
        },
        PreprocessorExpression::Placeholder => {
            CompareResult::DontCare
        },
    }
}