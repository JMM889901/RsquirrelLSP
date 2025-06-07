
use std::fs::read_to_string;

use serde::{self, Deserialize};
#[derive(Debug, Deserialize)]
pub struct Mod{
    pub Name: String,
    pub Scripts: Vec<LoadedFile>

}

#[derive(Debug, Deserialize)]
pub struct LoadedFile{
    pub RunOn: String,
    pub Path: String
}

/* for vanilla scripts mod "name" is just going to be vanilla

When: "SERVER || CLIENT"
Scripts:
[
	sh_damage_history.gnut 
]

When: "SERVER"
Scripts:
[
	ai/_ai_pilots.gnut 
]

When: "(SERVER || CLIENT) && SP"
Scripts:
[
	sh_consts_sp.gnut
	sp/sh_sp_objective_strings.gnut
]

When: "(SERVER || CLIENT) && MP"
Scripts:
[
	//gamemodes/sh_dev_gamemodes_mp.gnut // DEVSCRIPTS REMOVE
	gamemodes/sh_gamemodes_mp.gnut
]
*/

peg::parser!{
	pub grammar rson(rsonname: &String) for str {
		rule ws() = quiet!{[' ' | '\t' | '\n' | '\r']*}
		rule ws1() = quiet!{[' ' | '\t']*}
		rule nl() = ws1() (comment()? ws1() "\n" ws1())+ ws1()
		rule comment() = quiet!{ws() "//" (!['\n'] [_])*} / quiet!{ws() "/*" (!"*/" [_])* "*/"}
		rule string() -> String
			= "\"" s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '/' | '_' | '.' | '-' ]*) "\"" {s.to_string()}
		rule when_scripts() -> Vec<LoadedFile>
			= "When:" ws() "\"" run_on:$(['a'..='z' | 'A'..='Z' | ' ' | '|' | '(' | ')' | '&' | '0'..='9' | '_']*) "\"" ws() 
				"Scripts:" ws() "[" ws() s:loaded_file(run_on.to_string())**( ws1() ("\n\r" / "\r\n" / "\n" / "\r") ws()) ws() "]" {s}
		rule loaded_file(run_on: String) -> LoadedFile
			= name:ident() ws1() comment()? {LoadedFile{RunOn: run_on, Path: name}}

		rule ident() -> String
			= a:$quiet!{['a'..='z' | 'A'..='Z' | '0'..='9' | '/' | '_' | '.' | '-' ]+}  {a.to_string()}
		
		pub rule rson() -> Mod
			= (ws() comment())* ws() files:when_scripts() ** (ws() (comment() ws() )*) ws() (comment() ws())* {
				let flat_files = files.into_iter().flatten().collect();
				Mod{Name: rsonname.clone().to_string(), Scripts: flat_files}
			}
	}
}

pub fn parse_rson(rson: String, name: &String) -> Mod{
	rson::rson(&rson, name).unwrap()
}

#[test]
fn test_rson(){
	let rson = read_to_string("scripts.rson").unwrap();
	let result = parse_rson(rson, &"test".to_string());
}
