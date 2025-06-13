use peg::{Parse, ParseElem, RuleResult};
use sqparse::{token::TerminalToken, tokenize};

fn main() {
    let example_string = "
    #if SERVER
	global function guh

	struct
	{
		table wuh
        string guh;
	} file
//regular comment
    #endif
    ";

    let result = tokenize(example_string, sqparse::Flavor::SquirrelRespawn).unwrap();
    println!("{:#?}", result);


    let ast = sqparse::parse(&result, sqparse::Flavor::SquirrelRespawn).unwrap();
    println!("{:#?}", ast);
}
mod peg_token_parser;
mod preprocessor_ast;
mod chumsky;
struct SqParseTokenItemList(Vec<sqparse::TokenItem<'static>>);
impl<'a> Parse for &'a SqParseTokenItemList {
    type PositionRepr = usize;

    fn start<'input>(&'input self) -> usize {
        0
    }

    fn is_eof<'input>(&'input self, p: usize) -> bool {
        p >= self.0.len()
    }

    fn position_repr<'input>(&'input self, p: usize) -> Self::PositionRepr {
        p
    }
}
impl<'a> ParseElem<'a> for &'a SqParseTokenItemList {
    type Element = &'a sqparse::TokenItem<'static>;

    fn parse_elem(&self, pos: usize) -> RuleResult<Self::Element> {
        self.0.get(pos).map_or(
            RuleResult::Failed,
            |item| RuleResult::Matched(pos + 1, item),
        )
    }
}

struct SqParseTokenList(Vec<sqparse::token::Token<'static>>);
impl<'a> Parse for &'a SqParseTokenList {
    type PositionRepr = usize;

    fn start<'input>(&'input self) -> usize {
        0
    }

    fn is_eof<'input>(&'input self, p: usize) -> bool {
        p >= self.0.len()
    }

    fn position_repr<'input>(&'input self, p: usize) -> Self::PositionRepr {
        p
    }
}
impl<'a> ParseElem<'a> for &'a SqParseTokenList {
    type Element = &'a sqparse::token::Token<'static>;

    fn parse_elem(&self, pos: usize) -> RuleResult<Self::Element> {
        self.0.get(pos).map_or(
            RuleResult::Failed,
            |item| RuleResult::Matched(pos + 1, item),
        )
    }
}