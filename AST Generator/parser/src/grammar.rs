
use crate::{ast::*, SquirrelParse};
use ConfigAnalyser::sq_file_variant::SqFileVariant;
use crate::error::Error;
peg::parser!{
    //This just builds an AST, no validation logic is done here
    pub grammar squirrel_ast(offset: &std::cell::RefCell<isize>, parse_data: &crate::SquirrelParse) for SqFileVariant{


        pub rule file_scope() -> Vec<Element<AST>> = 
            ___* untyped()? a:file_scope_statement() b:(newline_then(<file_scope_statement()>))* ___* {
                vec![vec![a], b].concat()
            }
            
        rule untyped() = _ "untyped" ___* {
            *parse_data.untyped.write().unwrap() = true;
        }

        rule global_func() -> Element<AST> = start:position() "global function" __ name:ident() end:position() {
            Element::new(AST::Global(name), (start, end))
        }

        rule file_scope_statement() -> Element<AST> = 
            a:global_func() {a}
            / a:const_declaration() {a}
            / a:singleton_struct() {a}
            / a:struct_def() {a}
            / a:enum_def() {a}
            / a:typedef() {a}
            / a:function() {a}
            / a:declaration() {a} //This is the most common, so it would be nice to have this first, but its also the most vague, meaning it will catch a lot of things it shouldnt
            / a:assignment() {a}
            / a:catchall() {a}

        rule function_scope() -> Vec<Element<AST>> = 
            ___* a:function_scope_statement() b:(newline_then(<function_scope_statement()>))* ___* {
                vec![vec![a], b].concat()
            } / ___* {Vec::new()}
        
        pub rule function_scope_statement() -> Element<AST> = 
            a:thread() {a}
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
            / a:wait() {a}
            / a:switch() {a}
            / a:else_statement() {a}
            / a:while_statement() {a}
            / a:for_statement() {a}
            / a:foreach() {a}
            / a:return_statement() {a}
            / a:unreachable() {a}
            / a:r#break() {a}
            / a:r#continue() {a}
        
        rule thread() -> Element<AST> = ___* s:position() "thread" _ a:expression() e:position() {
            let ast = AST::Thread(a);
            return Element::new(ast, (s, e));
        }

        rule wait() -> Element<AST> = ___* s:position() "wait" _ a:$(float()/int()) e:position() {
            let ast = AST::Wait(a.parse().unwrap());
            return Element::new(ast, (s, e));
        }

        pub rule if_statement() -> Element<AST> = ___* s:position() "if" _ "(" _ condition:expression()? _ ")" _ block:block_or_oneliner() e:position() {
            let ast = AST::If { condition: condition, actions: block };
            return Element::new(ast, (s, e));
        }

        pub rule else_statement() -> Element<AST> = ___* s:position() "else" _ block:block_or_oneliner() e:position() {
            let ast = AST::If { condition:None, actions: block };
            return Element::new(ast, (s, e));
        }//Do i need else if? its literally just a one-liner else with an if
        //while(true){}
        pub rule while_statement() -> Element<AST> = ___* s:position() "while" _ "(" _ condition:expression()? _ ")" _ block:block_or_oneliner() e:position() {
            let ast = AST::While { condition: condition, actions: block };
            return Element::new(ast, (s, e));
        }
        //for(int x; x < 5; x++){}
        pub rule for_statement() -> Element<AST> = ___* s:position() "for" _ "(" _ init:condition_setter()? _ ";" _ condition:expression()? _ ";" _ increment:condition_setter()? _ ")" _ block:block_or_oneliner() e:position() {
            let ast = AST::For { init: init, condition: condition, increment: increment, actions: block };
            return Element::new(ast, (s, e));
        }
        //foreach(entity player in GetPlayerArray())
        pub rule foreach() -> Element<AST> = ___* s:position() "foreach" _ "(" _ iterators:foreach_conditions() ** (___* "," ___*) _ "in" _ iterable:expression() _ ")" _ block:block_or_oneliner() e:position() {
            let ast = AST::ForEach { iterators: iterators, iterable: iterable, actions: block };//This is sketchy
            return Element::new(ast, (s, e));
        }

        pub rule switch() -> Element<AST> = ___* s:position() "switch" _ "(" _ value:expression() _ ")" ___* "{" _ cases:switch_case() ** (___*) ___* default:("default" _ ":" _ block:switch_block() ___*{block})? "}" e:position() {
            let ast = AST::Switch{ condition: value, cases: cases, default};
            return Element::new(ast, (s, e));
        }

        rule switch_case() -> (Vec<Element<AST>>, Vec<Element<AST>>) = ___* s:position() values:("case" _ value:expression() _ ":"{value}) ++ (___+) _ block:switch_block() e:position() {
            (values, block)
        }

        rule switch_block() -> Vec<Element<AST>> =
            ___* a:(a:function_scope_statement(){a}) b:(newline_then(<!("case" / "default") a:function_scope_statement(){a}>))* ___* {//I really REALLY dont like this
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

        pub rule anonymous_function() -> Element<AST> = start:position() "function" _ "(" _ args:declaration() ** (___* "," ___*) _ ")" _ ":" _
	        "(" _ included_vars:element(<ident()>) ** (___* "," ___*) _ ")" _ things:block() end:position() {
                let ast = AST::AnonymousFunction(args, included_vars, things);
                return Element::new(ast, (start, end));
            }

        //^Control flow^

        //General
        pub rule condition_setter() -> Element<AST> = //This is for for loops and such
            a:declaration(){?
                match *a.value{
                    AST::Declaration { name, vartype, value} => {
                        if *name.value == "in"{
                            return Err("Cannot use 'in' as a variable name");//This is a hack for foreach loops
                        }
                        return Ok(Element::new(AST::Declaration { name: name, vartype: vartype, value: value }, a.range));
                    }
                    _ => panic!()
                }
            } / assignment() / expression() / element(<a:element(<ident()>) {AST::Declaration { name: a, vartype: Element::new(Type::Any, (0,0)), value: None }}>)

        pub rule foreach_conditions() -> Element<AST> = 
        a:declaration(){?
            match *a.value{
                AST::Declaration { name, vartype, value} => {
                    if *name.value == "in"{
                        return Err("Cannot use 'in' as a variable name");//This is a hack for foreach loops
                    }
                    return Ok(Element::new(AST::Declaration { name: name, vartype: vartype, value: value }, a.range));
                }
                _ => panic!()
            }
        }
        / element(<a:element(<ident()>) {AST::Declaration { name: a, vartype: Element::new(Type::Any, (0,0)), value: None }}>)
        pub rule declaration() -> Element<AST> = 
            s:position() sqtype:element(<vartype()>) __ n:element(<ident()>) val:(_ "=" _ val:expression() {val})? e:position() {
                //Untyped declaration is bad, it is supported so 
                Element::new(AST::Declaration { name: n, vartype: sqtype, value: val }, (s, e))
            }

        pub rule assignment() -> Element<AST> = 
			s:position() n:expression() _ assignment_symbol() _ v:expression() e:position() {
				let ast = AST::Assignment { var:n, value: v };
				let elem = Element::new(ast, (s, e));
				return elem;
			}

        pub rule assignment_symbol() =
            "="
            / "+="
            / "*="
            / "/="

        //^General^

        pub rule function() -> Element<AST> = functype:element(<vartype()>)? _ "function" _ name:element(<ident()>) _ "(" _ args:(declaration()/assignment()/untyped_declaration()) ** (___* "," ___*) _ ")" _ block:block() {
            let ast = AST::Function { name: name, args: args, returns: functype, actions: block };
            return Element::new(ast, (0, 0));
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
            let ast = AST::ConstDeclaration { name: typename.1, vartype: typename.0, value: v };
            //Need to pre-register variable, at least if i want to do multithreading later
            return Element::new(ast, (s, e));
        }

        pub rule struct_def() -> Element<AST> = ___* s:position() global:("global" _)? "struct" _ name:element(<ident()>) ___* "{" ___* values:struct_attributes() ___* "}" e:position() {
            
            let ast = AST::StructDeclaration { global: global.is_some(), name: name, attributes: values };
            //Need to pre-register if global, at least if i want to do multithreading later
            return Element::new(ast, (s, e));
            
        }

        pub rule singleton_struct() -> Element<AST> = ___* s:position() global:("global" _)? "struct" ___* "{" ___* values:struct_attributes() ___* "}" ___* name:element(<ident()>) ___* e:position() {
            let struct_ast = Element::new(AST::StructDeclaration { global: global.is_some(), name: name.clone(), attributes: Vec::new() }, (s, e));
            let decleration = AST::Declaration { name: name.clone(), vartype: Element::new(Type::Named(*name.value.clone()), (s, e)), value: Some(struct_ast) };
            //Need to pre-register if global, at least if i want to do multithreading later
            return Element::new(decleration, (s, e));
        }

        rule enum_def() -> Element<AST> = ___* s:position() global:("global" _)? "enum" _ name:element(<ident()>) ___* values:enum_values() ___* e:position() ___* {
            let ast = AST::EnumDeclaration { global: global.is_some(), name: name };
            //Need to pre-register if global, at least if i want to do multithreading later
            return Element::new(ast, (s, e));
        }

        rule enum_values() -> Vec<Element<String>> =  s:position() "{" ___* values:element(<ident()>) ** (___* "," ___*) _ ","? ___* "}" e:position() {values}

        rule typedef() -> Element<AST> = ___* s:position() "typedef" _ name:element(<ident()>) _ sqtype:element(<vartype()>) e:position() {
            let ast = AST::Typedef { name: name, sqtype: sqtype };
            //Need to pre-register if global, at least if i want to do multithreading later
            return Element::new(ast, (s, e));
        }

        rule struct_attributes() -> Vec<Element<AST>> =
            first:struct_def_statement() rest:(newline_then(<struct_def_statement()>))* {
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
            a:(@) _ "||" ___* b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Or(a, b))}}
            a:(@) _ "&&" ___* b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::And(a, b))}}
            --//I think this is the right order of precidence
            a:(@) _ "==" _ b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Eq(a, b))}}
            a:(@) _ "!=" _ b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Neq(a, b))}}
            a:(@) _ "<" _ b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Lt(a, b))}}
            a:(@) _ ">" _ b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Gt(a, b))}}
            a:(@) _ "<=" _ b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Lte(a, b))}}
            a:(@) _ ">=" _ b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Gte(a, b))}}
            --
            a:(@) _ "+" _ b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Add(a, b))}}
            a:(@) _ "-" _ b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Sub(a, b))}}
            --
            a:(@) _ "*" _ b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Mul(a, b))}}
            a:(@) _ "/" _ b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Div(a, b))}}
            a:(@) _ "%" _ b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Mod(a, b))}}
            --//Im not 100% sure IF this needs to be seperate, but eh
            a:(@) _ "**" _ b:@ {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Pow(a, b))}}
            --
            a:(@) "++" {Element{range: (a.range), value: Box::new(AST::Increment(a))}}
            "++" _ a:(@) {Element{range: (a.range), value: Box::new(AST::Increment(a))}}
            a:(@) "--" {Element{range: (a.range), value: Box::new(AST::Decrement(a))}}
            "--" _ a:(@) {Element{range: (a.range), value: Box::new(AST::Decrement(a))}}
            //TODO: is += supported? ive never used it < its an assignment, even if it is supported it doesnt go here
            a:(@) "[" _ b:expression() _ "]" {Element{range: (a.range.0, b.range.1), value: Box::new(AST::Index(a, b))}}
            a:(@) "."  b:ident() e:position() {Element{range: (a.range.0, e), value: Box::new(AST::Member(a, b))}} //I can do this for string tables as well as structs
            a:(@) _ "(" ___* b:expression() ** (___* "," ___*) ___* c:("," _ c:call_explicit_values(){c})? ___* ")" e:position() {
                let mut args = b;
                args.extend(c.unwrap_or(vec![]));
                Element{range: (a.range.0, e), value: Box::new(AST::FunctionCall{function: a, args})}
            }
            --
            a:element(<"expect" _ t:element(<vartype()>) _ "(" _ expr:expression() _ ")" {AST::Expect(t, expr)}>) {a}//Yeah im just lazy
            "clone" _ a:@ {Element{range: a.range.clone(), value: Box::new(AST::Clone(a))}}
            "(" _ a:expression() _ ")" {a}
            "!" _ a:@ {Element{range: a.range.clone(), value: Box::new(AST::Not(a))}}
            "-" _ a:@ {Element{range: a.range.clone(), value: Box::new(AST::Neg(a))}}
            a:(@) _ "in" _ b:@ {Element{range: (a.range.0, b.range.1), value:Box::new(AST::In(a, b))}}
            a:(@) _ "?" _ b:expression() _ ":" _ c:expression() {Element{range: (a.range.0, c.range.1), value: Box::new(AST::Ternary(a, b, c))}}
            a:anonymous_function() {a} //Very scary
            a:atom() {a}
        }

		rule vartype() -> Type =
		a:precedence!{
			a:(@) __ "functionref" _ "(" _ args:vartype() ** (___* "," ___*) _ ")" {Type::Functionref(Box::new(a), args.into_iter().map(|item| Argument{ arg_type: item, default: false}).collect())}
			"void" __ "functionref" _ "(" _ args:vartype() ** (___* "," ___*) _ ")" {Type::Functionref(Box::new(Type::Void), args.into_iter().map(|item| Argument{ arg_type: item, default: false}).collect())}
			a:(@) "&" {Type::Ref(Box::new(a))}
			a:(@) "[" _ b:$int() _ "]" {(Type::FixedArray(Box::new(a), b.parse().unwrap()))}
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

        rule int() = ['0'..='9']+
        rule float() = ['0'..='9']+ "." ['0'..='9']+
        rule bool() = "true"
        / "false"
		rule string() = "\"" (!"\""[_])* "\""
        rule asset() = "$" string()
        rule vector() = "<" _ (float() / int()) _ "," _ (float() / int()) _ "," _ (float() / int()) _ ">" 
        rule array() -> AST = s:position() "[" ___* vars:expression() ** (___* "," ___*) ___* ","? ___* "]" e:position() {
			let ast = AST::Array(vars);
			return ast
		}
        rule table() -> AST = s:position() "{" _ value:(key:expression() _ ":" _ value:expression() {(key, value)}) ** (___* "," ___*) _ "}" e:position() {
			AST::Table(value)
		}
		rule struct_values() -> Vec<(Element<String>, Element<AST>)> =  s:position() "{" _ value:(key:element(<ident()>) _ ":" _ value:expression() {(key, value)}) ** (___* "," ___*) _ "}" e:position() {value}

		rule call_explicit_values() -> Vec<(Element<AST>)> =  s:position() "{" _ value:(key:element(<ident()>) _ "=" _ value:expression() {(value)}) ** (___* "," ___*) _ "}" e:position() {value}//This is destructive,  bad!


        rule newline_delimited<T>(statement: rule<T>) -> Vec<T> = (a:statement() b:newline_then(<statement()>)* {let mut a = vec![a]; a.extend(b); a}) / {Vec::new()}//I dont much like this, most places i would use this have specific edge cases

        rule newline_then<T>(statement: rule<T>) -> T= start:position() whitespace:( _ comment()? _ a:$("\n\r" / "\r\n" / "\n") _ comment()? _ {a})* end:position() thing:statement()
		{
            //So this is, kind of scuffed
            //The only reason we need this kind of stuff is for good error handling
            //if i used just require-newline it would raise errors on the final line of things, since unlike matching rules it cant really "undo" if a later part fails to match
			if whitespace.join("").len() == 0{
				let err = Error::ExpectedNewlineError;
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
            float() {AST::Literal(Type::Float)}
            / int() {AST::Literal(Type::Int)}
            / bool() {AST::Literal(Type::Bool)}
            / asset() {AST::Literal(Type::Asset)}
            / string() {AST::Literal(Type::String)}
            / vector() {AST::Literal(Type::Vector)}
			/ a:struct_values() {AST::KeyValues(a)}
            / array()
            / table()
        )

        rule ident_internal() -> String = n:$quiet!{(['a'..='z' | 'A'..='Z' | '_']['a'..='z' | 'A'..='Z' | '_' | '0'..='9']*)} {n.to_string()}

        rule ident() -> String = !("expect" / "function" / "return" / "switch" / "if" / "typedef" / "else") a:ident_internal() {a} 

        rule position() -> usize = a:position!() {(a as isize + *offset.borrow()) as usize}

        rule _ = quiet!{([' ' | '\t'] / inline_comment())*}

		rule __ = ([' ' | '\t'] / inline_comment())+

		rule optional_newline() = quiet!{
            (_ (['\n' | '\r'] / "\n\r")? _)}
		rule ___ = quiet!{[' ' | '\t' | '\n' | '\r']} / inline_comment() / comment() 

		rule comment() = "//" (!['\n'][_])* / update_offset() //This is a little sketchy, but technically update offset can go anywhere a comment can go(well, TECHNICALLY it can go anywhere)
		rule inline_comment() = "/*" (!"*/"[_])* "*/" //This is destructive, i would like to retain comments for docstring support
        //I should probably shift to a RWLock vector instead of recursive returning

        rule catchall_instant() -> () = start:position() string:(a:(!("#"/"\r"/"\n"/"{"/"}")char:[_] {char})+ {a.iter().collect::<String>()}) end:position() {
            //Stick an error straight in the error list
            let err = Error::UnknownTokenError(string.clone());
            parse_data.register_error(Element::new(err, (start, end)));
            println!("Unknown token: {:?}", string);
		}

        //[#Pos:334]
        rule update_offset() = "[#Pos:" filepos:$int() "]" parsepos:position!() { let diff = filepos.parse::<isize>().unwrap() - parsepos as isize; *offset.borrow_mut() = diff; }

        rule catchall() -> Element<AST> = start:position() string:(a:(!("#"/"\r"/"\n"/"{"/"}")char:[_] {char})+ {a.iter().collect::<String>()}) end:position() {
            //return the error as an ast element
            println!("Unknown token: {:?}", string);
            let err = Error::UnknownTokenError(string.clone());
            let ast = AST::Error(err);
            let elem = Element::new(ast, (start, end));
            return elem;
		}
            
    }
}

