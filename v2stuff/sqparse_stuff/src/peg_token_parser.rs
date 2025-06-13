use sqparse::{token::Comment, TokenItem};

use super::*;

//This is probably a waste of time
//SQParse is vastly better than my parser in most ways, but lacks error handling, and honestly not sure if i could add that easily if i forked it
//So my options are either manually continue from an error, or make my own parser from the  sqparse lexer

peg::parser!{
    pub grammar token_item_parser<'a>() for &'a SqParseTokenItemList {
        use token_parser::parse_token;

        pub rule preprocessor_token_parser() -> ()
            = [TokenItem{token, close_index}] #{
                |input, pos| 
                return RuleResult::Matched(pos, ())
            }
    }

}

peg::parser!{
    pub grammar token_parser<'a>() for &'a SqParseTokenList {
        use terminal_parser::example;

        pub rule parse_token() -> ()
            = {}
    }

}


peg::parser!{
    pub grammar terminal_parser() for [TerminalToken] {
        pub rule example() -> () 
            = [TerminalToken::OpenBrace] {}
        
    }

}

peg::parser!{
    pub grammar commment_parser<'a>() for [Comment<'a>] {
        pub rule example() -> () 
            = [Comment::ScriptStyle(content)] {}
        
    }


}