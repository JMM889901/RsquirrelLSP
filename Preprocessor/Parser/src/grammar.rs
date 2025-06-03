use std::fs::read_to_string;

use crate::ast::*;
use crate::condition::Condition;
peg::parser!{
    pub grammar preprocessor_grammar() for str{
        pub rule parse() -> Vec<Node> =
            a:node()* {a} //This is a bit sketchy since this would "pass" even if it parsed nothing


        rule node() -> Node = start:position!() ast:(
            "#if" a:if_statement() {a}
            / text:text() {AST::Text(text)} 
        ) end:position!() {Node{
            range: (start, end),
            ast
        }}
        
        rule text() -> String =
            comment:$("/*" (!("*/")[_])* "*/") rest:text()? { vec![comment, &rest.unwrap_or("".to_string())].concat() }
            / comment:$("//" [^'\n']* "\n") rest:text()? {vec![comment, &rest.unwrap_or("".to_string())].concat()} //This does nothing with the comment, its just here to catch it and ignore preproc statements 
            / text:$((!("#if"/"#else"/"#elseif"/"#endif"/"//"/"/*")[_])+) rest:text()? {vec![text, &rest.unwrap_or("".to_string())].concat()}

        rule if_statement() -> AST =
            r#if:(_ cond:to_condition_expression() node:parse() {If::If(cond, node)})
            elifs:("#elseif" _ elif_cond:to_condition_expression() elif_nodes:parse() {If::If(elif_cond, elif_nodes)})*
            r#else:("#else" _ else_nodes:parse(){If::Else(else_nodes)})?
            "#endif" endpos:position!()          //Todo: soft enforce endif
            {
                let mut cases = vec![r#if];
                cases.extend(elifs);
                cases.extend(r#else.map(|x| vec![x]).unwrap_or([].to_vec()));
                AST::If(cases)
            }

    pub rule to_condition_expression() -> Condition = precedence!{
        x:(@) _ "&&" _ y:@ { Condition::And(Box::new(x), Box::new(y)) }
        x:(@) _ "||" _ y:@ { Condition::Or(Box::new(x), Box::new(y)) }
        --
        "!" x:@ { Condition::Not(Box::new(x)) }
        "(" _ x:to_condition_expression() _ ")" { x }
        x:term() { Condition::Term(x) }
    }
    rule term() -> String = n:$quiet!{(['a'..='z' | 'A'..='Z' | '_']['a'..='z' | 'A'..='Z' | '_' | '0'..='9']*)} {n.to_string()}

    rule _ = quiet!{[' ' | '\t']*}
    rule __ = quiet!{[' ' | '\t' | '\n' ]+ / ![_]} // "![_]" to catch eof
    }
}

#[cfg(test)]
//fn test_many_condition(){
//    let text = 
//"#if SERVER && SOME_VAR && MP_BOX && MP || SOME_OTHER_VAR && SP && SP_BOX || UI
//hi
//#endif";
//    let result = preprocessor_grammar::node(text).unwrap();
//    let cond1 = Condition::and(Condition::and(Condition::and(Condition::term("SERVER"), Condition::term("SOME_VAR")), Condition::term("MP_BOX")), Condition::term("MP"));
//    let cond2 = Condition::and(Condition::and(Condition::term("SOME_OTHER_VAR"), Condition::term("SP")), Condition::term("SP_BOX"));
//    let cond3 = Condition::term("UI");
//    let expect = AST::If(vec![If::If(Condition::or(Condition::or(cond1, cond2), cond3), vec![AST::Text("\nhi\n".to_string())])]);
//    assert_eq!(result[0], expect);
//    println!("{:?}", result);
//}//just incase i royally mess up
/// 
#[test]
fn handle_comments(){
    let input =
"//This is some random comment
#if SERVER
//Another comment
#endif";
let result = preprocessor_grammar::parse(input).unwrap();
//let expect = vec![AST::Text("//This is some random comment\n".to_string()), AST::If(vec![If::If(Condition::term("SERVER"), vec![AST::Text("\n//Another comment\n".to_string())])], 65)];
let expect = 1;
println!("{:?} ==? {:?}", result, expect);
//assert_eq!(result, expect)
}

#[test]
fn big_file_parses(){
    //Due to the size of the file hardcording the right answer isnt practical, this is just to test that things look "about right"
    let input = read_to_string("../test/SingleFile.nut").unwrap();
    let result = preprocessor_grammar::parse(&input).unwrap();
    assert!(result.len() == 91);
    //println!("{:?}", result)
}