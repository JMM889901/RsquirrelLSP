
use crate::ast::*;
use crate::condition::Condition;
peg::parser!{
    pub grammar preprocessor_grammar() for str{
        
        #[no_eof]
        pub rule parse_rls() -> Vec<Node> =
            a:node()* {a} 
            //This is a bit sketchy since this would "pass" even if it parsed nothing
        pub rule parse_dbg() -> Vec<Node> =
            a:node()* {a} 
            //This is a bit sketchy since this would "pass" even if it parsed nothing  

        rule node() -> Node = start:position!() ast:(
            "#if" a:if_statement() {a}
            / text:text() {AST::Text(text)} 
        ) end:position!() {Node{
            range: (start, end),
            ast
        }}

        rule text() -> String =
            text:text_internal()+ {text.concat()}
        
        rule text_internal() -> String =
            comment:$("/*" (!("*/")[_])* ("*/" / ![_])) { comment.to_string() }
            / comment:$("//" [^'\n']* ("\n" / ![_])) {comment.to_string()} 
            //This does nothing with the comment, 
            //its just here to catch it and ignore preproc statements 
            / text:$((!("#if"/"#else"/"#elseif"/"#endif"/"//"/"/*")[_])+) { text.to_string() }
            //TODO: Im not sure if tostring is costly here, i *think* its only type conversion

        rule if_statement() -> AST =
            r#if:(_ cond:to_condition_expression() node:parse_rls() {If::If(cond, node)})
            elifs:(
                "#elseif" _ elif_cond:to_condition_expression() elif_nodes:parse_rls() 
                    {If::If(elif_cond, elif_nodes)}
            )*
            r#else:("#else" _ else_nodes:parse_rls(){If::Else(else_nodes)})?
            ("#endif" / ![_]) endpos:position!()          //Todo: soft enforce endif
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
        rule term() -> String = n:$quiet!{(
            ['a'..='z' | 'A'..='Z' | '_']['a'..='z' | 'A'..='Z' | '_' | '0'..='9']*
        )} {n.to_string()}

        rule _ = quiet!{[' ' | '\t']*}
        rule __ = quiet!{[' ' | '\t' | '\n' ]+ / ![_]} // "![_]" to catch eof
    }
}

#[cfg(test)]
#[test]
fn test_many_condition(){
    let text = 
"#if SERVER && SOME_VAR && MP_BOX && MP || SOME_OTHER_VAR && SP && SP_BOX || UI
hi
#endif";
    let result = preprocessor_grammar::parse_dbg(text).unwrap();
    let cond1 = Condition::and(Condition::and(Condition::and(Condition::term("SERVER"), Condition::term("SOME_VAR")), Condition::term("MP_BOX")), Condition::term("MP"));
    let cond2 = Condition::and(Condition::and(Condition::term("SOME_OTHER_VAR"), Condition::term("SP")), Condition::term("SP_BOX"));
    let cond3 = Condition::term("UI");
    println!("{:?}", result);
}//just incase i royally mess up

#[test]
fn handle_comments(){
    let input =
"//This is some random comment
#if SERVER
//Another comment
#endif
// #if SERVER
//This is some random comment";
let result = preprocessor_grammar::parse_dbg(input).unwrap();
//let expect = vec![AST::Text("//This is some random comment\n".to_string()), AST::If(vec![If::If(Condition::term("SERVER"), vec![AST::Text("\n//Another comment\n".to_string())])], 65)];
let expect = 1;
//println!("{:?} ==? {:?}", result, expect);
println!("{:?}", result);
assert!(result.len() == 3);
//assert_eq!(result, expect)
}

#[test]
fn big_file_parses(){
    //Due to the size of the file hardcording the right answer isnt practical, this is just to test that things look "about right"
    let input = read_to_string("../test/SingleFile.nut").unwrap();
    let result = preprocessor_grammar::parse_dbg(&input).unwrap();
    println!("{:?}", result.len());
    //5 Distinct blocks, file uses a lot of #else
    assert!(result.len() == 5);
    //println!("{:?}", result)
}


#[test]
fn many_variants(){
    let runon = "SERVER || CLIENT || UI";
    let path = "../../TestSets/ValidStructure/ManyVariants/mod/scripts/vscripts/firstFile.gnut";
    let text = read_to_string(path).unwrap();
    let result = preprocessor_grammar::parse_dbg(&text).unwrap();
    let runon = preprocessor_grammar::to_condition_expression(runon).unwrap();
    println!("{:?}", result.len());
    assert!(result.len() == 9);
}

#[cfg(test)]
#[test]
fn fuzz(){
    use rand::Rng;

    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#abcdefghijklmnopqrstuvwxyz/*\\ \";,.!@#$%^&*()_+{}|:<>?~`-=[];',./\n";
    let mut rng = rand::rng();
    for _ in 0..100 {
        let len = rng.random_range(1..=1000);
        let s: String = (0..len).map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char).collect();
        
        let result = preprocessor_grammar::parse_dbg(&s);
        assert!(result.is_ok(), "Failed to parse fuzzed input: {} \n\n error because of: {:?}", s,  result.err());
    }
}