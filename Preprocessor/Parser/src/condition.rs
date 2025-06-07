

#[derive(Clone, PartialEq, Debug)]
pub enum Condition{
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
    Term(String),
}
//impl fmt::Debug for Condition{
//    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//        match self{
//            Condition::Term(term) => write!(f, "{}", term),
//            Condition::Not(cond) => write!(f, "!{:?}", cond),
//            Condition::And(conda, condb) => write!(f, "{:?} && {:?}", conda, condb),
//            Condition::Or(conda, condb) => write!(f, "{:?} || {:?}", conda, condb)
//
//        }
//    }
//}
impl Condition{
    pub fn and(a: Condition, b: Condition) -> Condition{
        Condition::And(Box::new(a), Box::new(b))
    }
    pub fn or(a: Condition, b: Condition) -> Condition{
        Condition::Or(Box::new(a), Box::new(b))
    }
    pub fn term(a: &str) -> Condition{
        Condition::Term(a.to_string())
    }

    //The following utils are used in other parts of the codebase
    pub fn get_terms(&self) -> Vec<String>{
        match self{
            Self::And(a, b) => [a.get_terms(), b.get_terms()].concat(),
            Self::Or(a, b) => [a.get_terms(), b.get_terms()].concat(),
            Self::Not(a) => a.get_terms(),
            Self::Term(term) => vec![term.clone()]
        }
    }

    fn get_impossible_terms(cond: &String) -> Vec<String>{
        match cond.as_str(){
            "SERVER" => vec!["CLIENT".to_string(), "UI".to_string()],
            "CLIENT" => vec!["SERVER".to_string(), "UI".to_string()],
            "UI" => vec!["SERVER".to_string(), "CLIENT".to_string()],
            _ => vec![]
        }
    }
    pub fn get_impossible_conditions(cond: &String) -> Vec<String>{
        Condition::get_impossible_terms(cond).iter().map(|term| term.to_string()).collect()
    }
    pub fn is_vm(cond: &String) -> bool{
        match cond.as_str(){
            "SERVER" | "CLIENT" | "UI" => true,
            _ => false
        }
    }
}
