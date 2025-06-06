
use crate::{ast::*, SquirrelParse};
use ConfigAnalyser::SqFileVariant;
use crate::error::Error;
peg::parser!{
    //This just builds an AST, no validation logic is done here
    pub grammar squirrel_ast(offset: &std::cell::RefCell<isize>, parse_data: &crate::SquirrelParse) for SqFileVariant{

        #[no_eof]
        pub rule file_scope_rls() -> Vec<Element<AST>> = 
            ___* untyped()? a:file_scope_statement() _ b:(newline_then(<file_scope_statement()>))* whitespace()* {
                vec![vec![a], b].concat()
            } / ___* {Vec::new()}

        pub rule file_scope_dbg() -> Vec<Element<AST>> = 
            ___* untyped()? a:file_scope_statement() _ b:(newline_then(<file_scope_statement()>))* whitespace()* {
                vec![vec![a], b].concat()
            } / ___* {Vec::new()}
            
        rule untyped() = _ "untyped" ___* {
            *parse_data.untyped.write().unwrap() = true;
        }

        rule global_func() -> Element<AST> = start:position() "global function" __ name:ident() end:position() {
            Element::new(AST::Global(name), (start, end))
        }

        rule globalize_all_functions() -> Element<AST> = start:position() "globalize_all_functions" end:position() {
            Element::new(AST::GlobalizeAllFunctions, (start, end))
        }

        rule file_scope_statement() -> Element<AST> = 
            a:global_func() {a}
            / a:globalize_all_functions() {a}
            / a:const_declaration() {a}
            / a:singleton_struct() {a}
            / a:struct_def() {a}
            / a:enum_def() {a}
            / a:typedef() {a}
            / a:function() {a}
            / a:declaration() {a} //This is the most common, so it would be nice to have this first, but its also the most vague, meaning it will catch a lot of things it shouldnt
            / a:assignment() {a}
            / a:global_declaration() {a}//I hate that this is even a feature of the language, its such a weird thing
            / a:catchall() {a}

        rule function_scope() -> Vec<Element<AST>> = 
            ___* a:function_scope_statement() _ b:(freeze_state(<token_then(<function_scope_statement()>, <function_scope_whitespace()>)>))* whitespace()* {
                vec![vec![a], b].concat()
            } / ___* {Vec::new()}
        
        pub rule function_scope_statement() -> Element<AST> =
            a:keyword() {a}
            / a:const_declaration() {a} //This sucks, i hate this
            / a:declaration() {a}
            / a:assignment() {a}
            / a:control_flow() {a}
            / a:expression() {a} //should raise errors if this doesnt "do" something
            / element(<(a:block() {AST::AnonymousScope(a)})>) //Arbitrary block, sometimes used as a reuslt of conditional compilation shenanigans
            / a:catchall() {a}

        //Control flow

        rule control_flow() -> Element<AST> =
            a:if_statement() {a}
            / a:try_catch() {a}
            / a:switch() {a}
            / a:else_statement() {a}
            / a:while_statement() {a}
            / a:for_statement() {a}
            / a:foreach() {a}
            / a:return_statement() {a}
            / a:unreachable() {a}
            / a:r#break() {a}
            / a:r#continue() {a}

        rule keyword() -> Element<AST> =
            throw()
            / a:thread()
            / a:waitthread()
            / a:wait()
            / delete()

        rule throw() -> Element<AST> = ___* s:position() "throw" __ a:expression() e:position() {
            let ast = AST::Throw(a);
            return Element::new(ast, (s, e));
        }

        rule delete() -> Element<AST> = ___* s:position() "delete" __ a:expression() e:position() {
            let ast = AST::Delete(a);
            return Element::new(ast, (s, e));
        }
        
        rule thread() -> Element<AST> = ___* s:position() "thread" __ a:expression() e:position() {
            let ast = AST::Thread(a);
            return Element::new(ast, (s, e));
        }

        rule waitthread() -> Element<AST> = ___* s:position() "waitthread" __ a:expression() e:position() {
            let ast = AST::Thread(a);
            return Element::new(ast, (s, e));
        }

        rule wait() -> Element<AST> = ___* s:position() "wait" __ a:$(expression()) e:position() {
            let ast = AST::Wait(a.parse().unwrap());
            return Element::new(ast, (s, e));
        }//TODO: this used to ensure it was a number, but that should be in analysis
        //Also you can have addition and such here so it needs to be expression

        pub rule if_statement() -> Element<AST> = ___* s:position() "if" _ "(" ___* condition:expression()? ___* ")" _ block:block_or_oneliner() e:position() {
            let ast = AST::If { condition: condition, actions: block };
            return Element::new(ast, (s, e));
        }

        pub rule else_statement() -> Element<AST> = ___* s:position() "else" _ block:block_or_oneliner() e:position() {
            let ast = AST::If { condition:None, actions: block };
            return Element::new(ast, (s, e));
        }//Do i need else if? its literally just a one-liner else with an if
        //while(true){}

        rule while_statement() -> Element<AST> =
            a:while_statement_standard() / a:do_while_statement() {a}

        pub rule while_statement_standard() -> Element<AST> = ___* s:position() "while" _ "(" ___* condition:expression()? ___* ")" _ block:block_or_oneliner() e:position() {
            let ast = AST::While { condition: condition, actions: block };
            return Element::new(ast, (s, e));
        }
        pub rule do_while_statement() -> Element<AST> = ___* s:position() "do" _ block:block_or_oneliner() _ "while" _ "(" _ condition:expression()? _ ")" e:position() {
            let ast = AST::While { condition: condition, actions: block };
            return Element::new(ast, (s, e));
        }
        pub rule try_catch() -> Element<AST> = ___* s:position() "try" ___* tstart:position() block1:block_or_oneliner() tend:position() ___* "catch" ___* "(" ___* condition:expression()? ___* ")" ___* estart:position() block2:block_or_oneliner() eend:position() e:position() {
            let catch = AST::Catch { exception: condition, actions: block2 };
            let try_scope = AST::AnonymousScope(block1.clone());//bit weird, i can get rid of this if i want to
            //This just makes everything play a little nicer with visitor
            let ast = AST::Try { actions: Element::new(try_scope, (tstart, tend)), catch: Element::new(catch, (estart, eend)) };
            return Element::new(ast, (s, e));
        }
        //for(int x; x < 5; x++){}
        pub rule for_statement() -> Element<AST> = ___* s:position() "for" _ "(" ___* init:condition_setter() ** (_ "," _) ___* ";" ___* condition:expression()? ___* ";" ___* increment:condition_setter() ** (_ "," _) ___* ")" _ block:block_or_oneliner() e:position() {
            let ast = AST::For { init: init, condition: condition, increment: increment, actions: block };
            return Element::new(ast, (s, e));
        }
        //foreach(entity player in GetPlayerArray())
        pub rule foreach() -> Element<AST> = ___* s:position() "foreach" _ "(" _ iterators:foreach_conditions() ** (___* "," ___*) _ "in" _ iterable:expression() _ ")" _ block:block_or_oneliner() e:position() {
            let ast = AST::ForEach { iterators: iterators, iterable: iterable, actions: block };//This is sketchy
            return Element::new(ast, (s, e));
        }

        pub rule switch() -> Element<AST> = ___* s:position() "switch" _ "(" _ value:expression() _ ")" ___* "{" _ cases:switch_case() ** (___*) ___* ds:position() default:("default" _ ":" _ block:switch_block() ___*{block})? de:position() ___* "}" e:position() {
            let default = default.map(|x| {
                Element::new(AST::Case { condition: vec![], actions: x }, (ds, de))
            });
            let ast = AST::Switch{ condition: value, cases: cases, default};
            return Element::new(ast, (s, e));
        }//"default" case no longer gets hit, its been absorbed into switch

        rule switch_case() -> Element<AST> = ___* s:position() values:(value:("case" _ value:expression() {value} / ( default() )) _ ":" _ {value}) ++ (___+) _ block:switch_block() e:position() {
            //let values = values.into_iter().filter_map(|x| x).collect::<Vec<_>>();//Bit hacky
            Element::new(AST::Case { condition: values, actions: block }, (s, e))
        }

        rule default() -> Element<AST> = s:position() "default" e:position() {
            let ast = AST::Empty;
            return Element::new(ast, (s, e));
        }

        rule switch_block() -> Vec<Element<AST>> =
            ___* a:(!("case" / "default") a:function_scope_statement(){a}) _ b:(newline_then(<!("case" / "default") a:function_scope_statement(){a}>))* whitespace()* {//I really REALLY dont like this
                vec![vec![a], b].concat()
            } / {Vec::new()}



        rule return_statement() -> Element<AST> = ___* s:position() "return" _ value:expression()? e:position() {
            let ast = AST::Return(value);
            return Element::new(ast, (s, e));
        }

        rule unreachable() -> Element<AST> = ___* s:position() "unreachable" e:position() {
            let ast = AST::Unreachable;
            return Element::new(ast, (s, e));
        }

        rule r#break() -> Element<AST> = ___* s:position() "break" e:position() {
            let ast = AST::Break;
            return Element::new(ast, (s, e));
        }

        rule r#continue() -> Element<AST> = ___* s:position() "continue" e:position() {
            let ast = AST::Continue;
            return Element::new(ast, (s, e));
        }

        pub rule anonymous_function() -> Element<AST> = start:position() (sqtype:vartype() _)? "function" _ "(" _ args:declaration() ** (___* "," ___*) _ ")" included_vars:( _ ":" _
	        "(" _ included_vars:element(<ident()>) ** (___* "," ___*) _ ")" {included_vars} )? _ things:block() end:position() {
                let ast = AST::AnonymousFunction(args, included_vars.unwrap_or_default(), things);
                return Element::new(ast, (start, end));
            }

        //^Control flow^

        //General
        pub rule condition_setter() -> Element<AST> = //This is for for loops and such
            a:declaration(){?
                let (val, range) = a.unwrap();
                match val{
                    AST::Declaration { name, vartype, value} => {
                        if *name.value == "in"{
                            return Err("Cannot use 'in' as a variable name");//This is a hack for foreach loops
                        }
                        return Ok(Element::new(AST::Declaration { name: name, vartype: vartype, value: value }, range));
                    }
                    _ => panic!()
                }
            } / assignment() / expression() / element(<a:element(<ident()>) {AST::Declaration { name: a, vartype: Element::new(Type::Any, (0,0)), value: None }}>)

        pub rule foreach_conditions() -> Element<AST> = 
        a:declaration(){?
            let (val, range) = a.unwrap();
            match val{
                AST::Declaration { name, vartype, value} => {
                    if *name.value == "in"{
                        return Err("Cannot use 'in' as a variable name");//This is a hack for foreach loops
                    }
                    return Ok(Element::new(AST::Declaration { name: name, vartype: vartype, value: value }, range));
                }
                _ => panic!()
            }
        }
        / element(<a:element(<ident()>) {AST::Declaration { name: a, vartype: Element::new(Type::Any, (0,0)), value: None }}>)
        pub rule declaration() -> Element<AST> = 
            s:position() sqtype:element(<vartype()>) __ n:element(<ident()>) val:(_ "=" _ val:expression() {val})? e:position() {
                //Untyped declaration is bad, it is supported so 
                Element::new(AST::Declaration { name: n, vartype: sqtype, value: val }, (s, e))
            } / singleton_struct() //VERY bad and VERY scary

        pub rule assignment() -> Element<AST> = 
			s:position() n:expression() _ assignment_symbol() _ v:expression() e:position() {
				let ast = AST::Assignment { var:n, value: v };
				let elem = Element::new(ast, (s, e));
				return elem;
			}

        pub rule assignment_symbol() =
            "="
            / "+="
            / "-="
            / "*="
            / "/="

        //^General^

        pub rule function() -> Element<AST> = start:position() functype:element(<vartype()>)? _ "function" _ name:element(<ident()>) _ "(" _ args:(declaration()/assignment()/untyped_declaration()) ** (___* "," ___*) _ ")" _ block:block() end:position() {
            let ast = AST::Function { name: name, args: args, returns: functype, actions: block };
            //return Element::new(ast, (0, 0)); Leaving this comment here as a shrine to my stupidity
            return Element::new(ast, (start, end));
        }

        pub rule untyped_declaration() -> Element<AST> = ___* s:position() name:element(<ident()>) e:position() {
            let ast = AST::Declaration { name, vartype: Element::new(Type::Any, (s,e)), value: None };
            return Element::new(ast, (s, e));
        }

        pub rule block_or_oneliner() -> Vec<Element<AST>> = ___* "{" ___* a:function_scope() ___* "}" {a} / ___* a:function_scope_statement() {vec![a]}

        pub rule block() -> Vec<Element<AST>> = ___* s:position() "{" ___* a:function_scope() ___* "}" e:position() {
            a
        }

        //File scope exclusive
        pub rule const_declaration() -> Element<AST> = _ s:position() global:("global" _)? "const" _ typename:(sqtype:element(<vartype()>) _ n:element(<ident()>) {(Some(sqtype), n)}/ n:element(<ident()>) {(None, n)}) _ "=" _ v:(v:expression() {v} / a:element(<catchall_instant()>) {Element::new(AST::fallback_literal(), a.range)}) e:position() {
            let ast = AST::ConstDeclaration { global:global.is_some(), name: typename.1, vartype: typename.0, value: v };
            //Need to pre-register variable, at least if i want to do multithreading later
            return Element::new(ast, (s, e));
        }

        pub rule global_declaration() -> Element<AST> = //This is super duper bad right now
            _ s:position() "global" __ decl:declaration() e:position() {
                //for now just treat it like a const, they arent constant anyways
                let (val, range) = decl.unwrap();
                if let AST::Declaration { name, vartype, value } = val {
                    let value = match value {
                        Some(value) => value,
                        None => Element::new(AST::Literal(vartype.clone().unwrap_v()), range),
                    };
                    let ast = AST::ConstDeclaration { global: true, name: name, vartype: Some(vartype), value: value };
                    return Element::new(ast, (s, e));
                }
                todo!()
            }

        pub rule struct_def() -> Element<AST> = ___* s:position() global:("global" _)? "struct" _ name:element(<ident()>) ___* "{" ___* values:struct_attributes() ___* "}" e:position() {
            let ast = AST::StructDeclaration { global: global.is_some(), name: name, attributes: values };
            //Need to pre-register if global, at least if i want to do multithreading later
            return Element::new(ast, (s, e));
            
        }

        pub rule singleton_struct() -> Element<AST> = ___* s:position() global:("global" _)? "struct" ___* "{" ___* values:struct_attributes() ___* "}" ___* name:element(<ident()>) e:position() {
            let struct_ast = Element::new(AST::StructDeclaration { global: global.is_some(), name: name.clone(), attributes: Vec::new() }, (s, e));
            let decleration = AST::Declaration { name: name.clone(), vartype: Element::new(Type::Named(name.unwrap_v()), (s, e)), value: Some(struct_ast) };
            //Need to pre-register if global, at least if i want to do multithreading later
            return Element::new(decleration, (s, e));
        }

        rule enum_def() -> Element<AST> = ___* s:position() global:("global" _)? "enum" _ name:element(<ident()>) ___* values:enum_values() e:position() {
            let ast = AST::EnumDeclaration { global: global.is_some(), name: name };
            //Need to pre-register if global, at least if i want to do multithreading later
            return Element::new(ast, (s, e));
        }

        rule enum_values() -> Vec<Element<String>> =  s:position() "{" ___* values:(value:element(<ident()>) ( _ "=" _ int() )? {value}) ** (((___* "," ) / (_ newline())) ___*) _ ","? ___* "}" e:position() {values}

        rule typedef() -> Element<AST> = ___* s:position() global:("global" _)? "typedef" _ name:element(<ident()>) _ sqtype:element(<vartype()>) e:position() {
            let ast = AST::Typedef { global: global.is_some(), name: name, sqtype: sqtype };
            //Need to pre-register if global, at least if i want to do multithreading later
            return Element::new(ast, (s, e));
        }

        rule struct_attributes() -> Vec<Element<AST>> =
            first:struct_def_statement() _ rest:(freeze_state(<token_then(<struct_def_statement()>, <struct_def_whitespace()>)>))* {
                let mut vec = Vec::new();
                if let Some(first) = first{
                    vec.push(first);
                }
                for item in rest.into_iter().filter(|x| x.is_some()).map(|x| x.unwrap()){
                    vec.push(item);
                }
                vec
            } / {Vec::new()}
        
        rule struct_def_statement() -> Option<Element<AST>> =
            a:declaration() {Some(a)}
            / catchall_instant() {None}
        //^File scope exclusive^
        pub rule expression() -> Element<AST> = precedence!{
            a:(@) b:freeze_state(<___* "||" ___* b:expression() {b}>) {Element::newb( (a.range.0, b.range.1), AST::Or(a, b))}
            a:(@) b:freeze_state(<___* "&&" ___* b:expression() {b}>) {Element::newb( (a.range.0, b.range.1), AST::And(a, b))}
            --//I think this is the right order of precidence
            //Bitwise operators, these ones should be different to the above but since i dont have type resolving yet its fine for now
            a:(@) _ "|" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Or(a, b))}
            a:(@) _ "&" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::And(a, b))}
            a:(@) _ "^" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Xor(a, b))}
            --
            a:(@) _ "==" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Eq(a, b))}
            a:(@) _ "!=" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Neq(a, b))}
            a:(@) _ "<" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Lt(a, b))}
            a:(@) _ ">" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Gt(a, b))}
            a:(@) _ "<=" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Lte(a, b))}
            a:(@) _ ">=" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Gte(a, b))}
            --
            a:(@) _ "+" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Add(a, b))}
            a:(@) _ "-" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Sub(a, b))}
            --
            a:(@) _ "*" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Mul(a, b))}
            a:(@) _ "/" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Div(a, b))}
            a:(@) _ "%" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Mod(a, b))}
            --//Im not 100% sure IF this needs to be seperate, but eh
            a:(@) _ "**" _ b:@ {Element::newb( (a.range.0, b.range.1), AST::Pow(a, b))}
            --
            a:(@) "++" {Element::newb( (a.range), AST::Increment(a))}
            "++" _ a:(@) {Element::newb( (a.range), AST::Increment(a))}
            a:(@) "--" {Element::newb( (a.range), AST::Decrement(a))}
            "--" _ a:(@) {Element::newb( (a.range), AST::Decrement(a))}
            //TODO: is += supported? ive never used it < its an assignment, even if it is supported it doesnt go here
            a:(@) "[" _ b:expression() _ "]" {Element::newb( (a.range.0, b.range.1), AST::Index(a, b))}
            a:(@) "."  b:ident() e:position() {Element::newb( (a.range.0, e), AST::Member(a, b))} //I can do this for string tables as well as structs
            a:(@) _ "(" ___* b:expression() ** (___* "," ___*) ___* c:("," _ c:call_explicit_values(){c})? ___* ")" e:position() {
                let mut args = b;
                args.extend(c.unwrap_or(vec![]));
                Element::newb( (a.range.0, e), AST::FunctionCall{function: a, args})
            }
            --
            a:element(<"expect" _ t:element(<vartype()>) _ "(" _ expr:expression() _ ")" {AST::Expect(t, expr)}>) {a}//Yeah im just lazy
            "clone" _ a:@ {Element::newb( a.range.clone(), AST::Clone(a))}
            "(" ___* a:expression() ___* ")" {a}
            "!" _ a:@ {Element::newb( a.range.clone(), AST::Not(a))}
            "~" _ a:@ {Element::newb( a.range.clone(), AST::Neg(a))}//TODO: see previous on bitwise
            "-" _ a:@ {Element::newb( a.range.clone(), AST::Neg(a))}
            a:(@) _ "in" _ b:@ {Element::newb( (a.range.0, b.range.1), (AST::In(a, b)))}
            a:(@) _ "?" _ b:expression() _ ":" _ c:expression() {Element::newb( (a.range.0, c.range.1), AST::Ternary(a, b, c))}
            a:anonymous_function() {a} //Very scary
            a:atom() {a}
        }

		rule vartype() -> Type =
		a:precedence!{
			a:(@) __ "functionref" _ "(" _ args:(arg:vartype() (__ name:ident())? {arg}) ** (___* "," ___*) _ ")" {Type::Functionref(Box::new(a), args.into_iter().map(|item| Argument{ arg_type: item, default: false}).collect())}
			"void" __ "functionref" _ "(" _ args:(arg:vartype() (__ name:ident())? {arg}) ** (___* "," ___*) _ ")" {Type::Functionref(Box::new(Type::Void), args.into_iter().map(|item| Argument{ arg_type: item, default: false}).collect())}
			//The above are destructive, they lose the name of the variable, but right now thats unimportant
            a:(@) "&" {Type::Ref(Box::new(a))}
			a:(@) "[" _ b:$(int() / ident()) _ "]" {(Type::FixedArray(Box::new(a), b.parse().unwrap_or_default()))}
			--
            a:(@) _ "ornull" {Type::OrNull(Box::new(a))}
            --
			a:complextype() {a}
			a:primitivetype() {a}
			--
			string:ident() {Type::Named(string)}
		} 

        rule primitivetype() -> Type =
		"int" {Type::Int}
		/ "string" {Type::String}
		/ "float" {Type::Float}
		/ "bool" {Type::Bool}
		/ "array" {Type::SimpleArray}
		/ "table" {Type::SimpleTable}
		/ "asset" {Type::Asset}
		/ "entity" {Type::Entity}
		/ "vector" {Type::Vector}
		/ "var" {Type::Var}
        / "void" {Type::Void}//bad, this technically means i could have a void variable, but realistically thats on you for doing that

        rule complextype() -> Type =
        "array<" _ a:vartype() _ ">" {Type::Array(Box::new(a))}
        / "table<" _ a:vartype() _ "," _ b:vartype() _ ">" {Type::Table(Box::new(a), Box::new(b))}
        / "table<" _ a:vartype() _ ">" {Type::Table(Box::new(a), Box::new(Type::Var))}


        rule atom() -> Element<AST> =
        a:element(<literal()>) {a}
		/ variable()

        rule variable() -> Element<AST> = var:element(<ident()>) {
			Element::new(AST::Variable(var.clone()), var.range)
        }

        rule element<A>(a: rule<A>) -> Element<A> = start:position() thing:a() end:position() {
            Element::new(thing, (start,end)) //All contextwrapper things are shoved into an Arc
        }

        rule int() =  ['0'..='9']+
        rule float() = ['0'..='9']+ "." ['0'..='9']+ s:position() trailing:$(("." ['0'..='9']+)*) e:position() { 
            if trailing.len() > 0 {
                let error = Error::UnwantedTokenWarning(trailing.to_string());
                let elem = Element::new(error, (s, e));
                parse_data.register_error(elem);
            }
        }
        rule exponential() = ['0'..='9']+ ("." ['0'..='9']+)? "e" "-"? ['0'..='9']+
        //There is literally only 1 usage of this in my entire test set
        rule bool() = "true"
        / "false"
		rule string() = "\"" (escaped_quote() / (!"\""[_]))* "\""
        rule escaped_quote() = "\\" "\"" //gross
        rule character() = "'" (escaped_quote() / (!"'"[_]))* "'"
        rule asset() = "$" string()
        //rule vector() = "<" _ ("-"? float() / int()) _ "," _ ("-"? float() / int()) _ "," _ ("-"? float() / int()) _ ">" 
        rule vector() = "<" _ expression() _ "," _ expression() _ "," _ expression() _ ">" //TODO: These can only be numbers
        //Ill sort this later, it should be analysis stuff anyways
        
        rule array() -> AST = s:position() newline()? _ "[" ___* vars:expression() ** (((___* "," ) / (_ newline())) ___*) ___* ","? ___* "]" e:position() {
			let ast = AST::Array(vars);
			return ast
		}
        rule table() -> AST = s:position() "{" ___* value:(key:expression() _ (":"/"=") _ value:expression() {(key, value)}) ** (___* "," ___* !"..." ) (___* "," ___* "...")? ___* "}" e:position() {
			AST::Table(value)
		}//This is basically a duplicate of struct_values by rule, but they do slightly different things later 
		rule struct_values() -> Vec<(Element<String>, Element<AST>)> = s:position() "{" _ value:(key:element(<ident()>) _ ":" _ value:expression() {(key, value)}) ** (___* "," ___*) _ "}" e:position() {value}


		rule call_explicit_values() -> Vec<(Element<AST>)> =  s:position() "{" _ value:(key:element(<ident()>) _ "=" _ value:expression() {(value)}) ** (___* "," ___*) _ "}" e:position() {value}//This is destructive,  bad!


        rule newline_delimited<T>(statement: rule<T>) -> Vec<T> = (a:statement() _ b:newline_then(<statement()>)* {let mut a = vec![a]; a.extend(b); a}) / {Vec::new()}//I dont much like this, most places i would use this have specific edge cases

        rule struct_def_whitespace() -> Vec<Element<Error>> = start:position() space:((
            [' ' | '\t' | '\n' | '\r' | ',' | ';'] / inline_comment() {' '} / comment() {' '}// / catchall()
        )*) end:position() {
            let mut errs = Vec::new();
            for c in &space{
                if c == &';'{
                    let err = Error::UnwantedTokenWarning(";".to_string());
                    let elem = Element::new(err, (start, end));
                    errs.push(elem);
                } else if c == &','{
                    let err = Error::UnwantedTokenWarning(",".to_string());
                    let elem = Element::new(err, (start, end));
                    errs.push(elem);
                }
            }
            if space.len() == 0{
                let err = Error::ExpectedNewlineError;
                let elem = Element::new(err, (start, end));
                errs.push(elem);
            }
            errs
        }

        rule function_scope_whitespace() -> Vec<Element<Error>> = s:position() space:(
            [' ' | '\t' | '\n' | '\r' | ';'] / inline_comment() {' '} / comment() {' '}
        )* has_else:&"else"? e:position() {
            let mut errs = Vec::new();
            for c in &space{
                if c == &';'{
                    let err = Error::UnwantedTokenWarning(";".to_string());
                    let elem = Element::new(err, (s, e));
                    errs.push(elem);
                }
            }
            if space.len() == 0 && !has_else.is_some(){
                let err = Error::ExpectedNewlineError;
                let elem = Element::new(err, (s, e));
                errs.push(elem);
            }
            errs
        }

        rule freeze_state<T>(statement: rule<T>) -> T = old_offset:(() {offset.borrow().clone()}) the:(statement:statement() {statement}
        / nothing:(() {
            *offset.borrow_mut() = old_offset;
        }) "This tern really totally should not match which is why its so long and stupid" statement:statement() {statement}
    ) {the}


        rule token_then<T>(statement: rule<T>, token: rule<Vec<Element<Error>>>) -> T = start:position() token:token()? end:position() thing:statement() {
            if token.is_some(){
                let err = token.unwrap();
                for e in err{
                    parse_data.register_error(e);
                }
            }
            return thing
        }

        rule newline_then<T>(statement: rule<T>) -> T= start:position() whitespace:( _ comment()? _ a:$("\n\r" / "\r\n" / "\n" / ";") _ comment()? _ {a})* end:position() thing:statement()
		{
            //So this is, kind of scuffed
            //The only reason we need this kind of stuff is for good error handling
            //if i used just require-newline it would raise errors on the final line of things, since unlike matching rules it cant really "undo" if a later part fails to match
			if whitespace.join("").len() == 0{
                let err;

                err = Error::ExpectedNewlineError;
                let elem = Element::new(err, (start, end));
                parse_data.register_error(elem);
			}
            if whitespace.contains(&";"){
                let err = Error::UnwantedTokenWarning(";".to_string());
                let elem = Element::new(err, (start, end));
                parse_data.register_error(elem);
            }
            return thing
		}

        rule require_newline() =
        s:position() _ a:(_ comment()? _ a:$("\n\r" / "\r\n" / "\n") _ comment()? _ {a})* e:position() {
            //unused in favor of newline_then
            if a.join("").len() == 0{
                //todo!()
            }
        }

        rule literal() -> AST = (
            exponential() {AST::Literal(Type::Float)}//This might technically be int in squirrel but i dont care
            / float() {AST::Literal(Type::Float)}
            / int() {AST::Literal(Type::Int)}
            / bool() {AST::Literal(Type::Bool)}
            / asset() {AST::Literal(Type::Asset)}
            / string() {AST::Literal(Type::String)}
            / character() {AST::Literal(Type::String)} //TODO: this should be a char type, but i dont care right now
            / vector() {AST::Literal(Type::Vector)}
			/ a:struct_values() {AST::KeyValues(a)}
            / "null" {AST::Literal(Type::Any)}
            / array()
            / table()
        )

        rule ident_internal() -> String = n:$quiet!{(['a'..='z' | 'A'..='Z' | '_']['a'..='z' | 'A'..='Z' | '_' | '0'..='9']*)} {n.to_string()}

        rule ident() -> String = a:ident_internal() {?
            let banned = ["global", "expect" , "function" , "return" , "switch" , "if" , "typedef" , "else", "delete", "throw"];
            if banned.contains(&a.as_str()){
                return Err("Cannot use this as a variable name")
            }
            Ok(a)
        } 

        rule position() -> usize = a:position!() {(a as isize + *offset.borrow()) as usize}

        rule newline() = quiet!{(['\n' | '\r'] / "\n\r")}

        rule _ = quiet!{([' ' | '\t'] / inline_comment())*}

		rule __ = ([' ' | '\t'] / inline_comment())+

		rule optional_newline() = quiet!{
            (_ (['\n' | '\r'] / "\n\r" )? _)}
		rule ___ = quiet!{[' ' | '\t' | '\n' | '\r']} / inline_comment() / comment() 
        rule whitespace() = ___ / semicolon_newline_instant()

		rule comment() = "//" (!newline()[_])* / update_offset() //This is a little sketchy, but technically update offset can go anywhere a comment can go(well, TECHNICALLY it can go anywhere)
		rule inline_comment() = "/*" (!"*/"[_])* "*/" //This is destructive, i would like to retain comments for docstring support
        //I should probably shift to a RWLock vector instead of recursive returning

        rule catchall_instant() -> () = start:position() string:(a:(!("#"/"\r"/"\n"/"{"/"}")char:[_] {char})+ {a.iter().collect::<String>()}) end:position() {
            //Stick an error straight in the error list
            let err;
            if string == ";"{
                err = Error::UnwantedTokenWarning(string.clone());
            } else {
                err = Error::UnknownTokenError(string.clone());
            }
            parse_data.register_error(Element::new(err, (start, end)));
           // println!("Unknown token: {:?}", string);
		}

        //[#Pos:334]
        rule update_offset() = "[#Pos:" filepos:$int() "]" parsepos:position!() { let diff = filepos.parse::<isize>().unwrap() - parsepos as isize; *offset.borrow_mut() = diff; }

        rule catchall() -> Element<AST> = start:position() string:(a:(!("#"/"\r"/"\n"/"{"/"}"/"//"/"/*")char:[_] {char})+ {a.iter().collect::<String>()}) end:position() ( {? 
        if string.chars().all(|c| c.is_whitespace()) {
            return Err("Whitespace only token");//Replacing this entire parser soon, cba doing a proper fix
        } else {
            return Ok(());
        }}) {
            //return the error as an ast element
            //println!("Unknown token: {:?}", string);

            let err;
            if string == ";"{
                //err = Error::UnwantedTokenWarning(string.clone());
                err = Error::UnwantedTokenWarning(string);

            } else {
                err = Error::UnknownTokenError(string.clone());
            };
            let ast = AST::Error(err);
            let elem = Element::new(ast, (start, end));
            return elem;
		}

        rule semicolon_newline_instant() = start:position() ";" end:position() {
            let err = Error::UnwantedTokenWarning(";".to_string());
            parse_data.register_error(Element::new(err, (start, end)));
        }
            
    }
}



#[cfg(test)]
#[test]
fn fuzz(){
    use std::cell::RefCell;

    use rand::{distr::Alphanumeric, Rng};
    use squirrel_ast::{file_scope_dbg, file_scope_rls};

    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#abcdefghijklmnopqrstuvwxyz/*\\ \";,.!@#$%^&*()_+{}|:<>?~`-=[];',./\n";
    let mut rng = rand::rng();
    for _ in 0..100 {
        let len = rng.random_range(1..=1000);
        let s: String = (0..len).map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char).collect();
        let offset = RefCell::new(0 as isize);
        let parse_data = &SquirrelParse::empty();
        let variant = SqFileVariant::stateless(s);
        let result = file_scope_rls(&variant, &offset, parse_data);
        assert!(result.is_ok(), "Failed to parse fuzzed input: {} \n\n error because of: {:?}", variant.text(),  result.err());
    }
}