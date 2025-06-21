use std::path::Path;

//TODO: This is mostly just a copy of the old version
use crate::ModScriptEntry;

peg::parser! {
    pub grammar rson() for str {
        rule _() = quiet!{[' ' | '\t' | '\n' | '\r']*}



        rule ws() = quiet!{ [' ' | '\t' | '\n' | '\r']*}
        rule ws1() = quiet!{[' ' | '\t']*}
        rule nl() = ws1() (comment()? ws1() ("\n\r" / "\r\n" / "\n" / "\r") ws1())+ ws1()
        rule comment() = quiet!{ws() "//" (!['\n'] [_])*} / quiet!{ws() "/*" (!"*/" [_])* "*/"}
        rule string() -> String
            = "\"" s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '/' | '_' | '.' | '-' ]*) "\"" {s.to_string()}
        rule when_scripts() -> Vec<ModScriptEntry>
            = "When:" ws() "\"" run_on:$(['a'..='z' | 'A'..='Z' | ' ' | '|' | '(' | ')' | '&' | '0'..='9' | '_']*) "\"" ws()
                "Scripts:" ws() "[" ws() s:loaded_file(run_on.to_string())**( nl() ) ws() "]" {s}
        rule loaded_file(run_on: String) -> ModScriptEntry
            = name:ident() ws1() comment()? {ModScriptEntry{rel_path: Path::new(name.strip_prefix("/").unwrap_or(&name)).to_path_buf(), run_on}}

        rule ident() -> String
            = a:$quiet!{['a'..='z' | 'A'..='Z' | '0'..='9' | '/' | '_' | '.' | '-' ]+}  {a.to_string()}


        pub rule rson() -> Vec<ModScriptEntry>
            = (ws() comment())* ws() files:when_scripts() ** (ws() (comment() ws() )*) ws() (comment() ws())* {
                files.into_iter().flatten().collect()
            }
    }
}
