use core::panic;
use std::path::Display;

use chumsky::prelude::*;
use sqparse::{token::{TerminalToken, TokenType}, tokenize, Flavor, LexerError, TokenItem};

fn example_tokens() -> Vec<TokenItem<'static>> {
    let example_string = "
    #if SERVER && CLIENT && (MP ||  DEV)
	global function guh

	struct
	{
		table wuh;
        string guh
	} file
//regular comment
    #endif
    ";

    let result = tokenize(example_string, sqparse::Flavor::SquirrelRespawn).unwrap();
    result
}

pub enum PreprocessorAST<'ast>{
    If {
        condition: Vec<TokenItem<'ast>>,
        body: Vec<PreprocessorAST<'ast>>,
    },
    ElseIf {
        condition: Vec<TokenItem<'ast>>,
        body: Vec<PreprocessorAST<'ast>>,
    },
    Else {
        body: Vec<PreprocessorAST<'ast>>,
    },
    EndIf,
    Block(Vec<TokenItem<'ast>>),
}
#[derive(Debug, Clone, PartialEq)]
pub enum PreprocessorExpression<'ast>{
    Identifier(TokenItem<'ast>),
    Bracketed(Box<PreprocessorExpression<'ast>>),
    And(Box<PreprocessorExpression<'ast>>, Box<PreprocessorExpression<'ast>>),
    Or(Box<PreprocessorExpression<'ast>>, Box<PreprocessorExpression<'ast>>),
    Not(Box<PreprocessorExpression<'ast>>),
}
impl std::fmt::Display for PreprocessorExpression<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreprocessorExpression::Identifier(id) => write!(f, "{}", self.identifier_string()),
            PreprocessorExpression::Bracketed(expr) => write!(f, "({})", expr),
            PreprocessorExpression::And(left, right) => write!(f, "{} && {}", left, right),
            PreprocessorExpression::Or(left, right) => write!(f, "{} || {}", left, right),
            PreprocessorExpression::Not(expr) => write!(f, "!{}", expr),
        }
    }
}
impl PreprocessorExpression<'_>{
    pub fn identifier_string<'ast>(&self) -> String {
        match self {
            PreprocessorExpression::Identifier(id) => {
                if let TokenType::Identifier(name) = id.token.ty {
                    name.to_string()
                } else {
                    panic!("Expected identifier token, found: {:?}", id.token.ty);
                }
            },
            _ => panic!("This is not an identifier expression"),
        }
    }
}

pub struct SQParseTokenStreamPreprocessor<'src> {
    tokens: Vec<TokenItem<'src>>,
}
impl<'src> Input<'src> for SQParseTokenStreamPreprocessor<'src>{
    type Span = std::ops::Range<usize>;

    type Token = TokenItem<'src>;

    type MaybeToken = &'src Self::Token;

    type Cursor = usize;

    type Cache = ();

    fn begin(self) -> (Self::Cursor, Self::Cache) {
        todo!()
    }

    fn cursor_location(cursor: &Self::Cursor) -> usize {
        todo!()
    }

    unsafe fn next_maybe(
        cache: &mut Self::Cache,
        cursor: &mut Self::Cursor,
    ) -> Option<Self::MaybeToken> {
        todo!()
    }

    unsafe fn span(cache: &mut Self::Cache, range: std::ops::Range<&Self::Cursor>) -> Self::Span {
        todo!()
    }
}


fn identifier<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], TokenItem<'src>> {
    any().filter(|item: &TokenItem<'src>| {
        matches!(item.token.ty, TokenType::Identifier(_))
    })
}
fn terminal<'src>(token: TerminalToken) ->  Boxed<'src, 'src, &'src [TokenItem<'src>], TokenItem<'src>> {
    any().filter(move |item: &TokenItem| {
        matches!(item.token.ty, TokenType::Terminal(t) if t == token)
    }).boxed()
}
fn logical_op<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], TokenItem<'src>> {
    any().filter(|item: &TokenItem<'src>| {
        matches!(item.token.ty, TokenType::Terminal(TerminalToken::LogicalAnd) | 
                  TokenType::Terminal(TerminalToken::LogicalOr) | TokenType::Terminal(TerminalToken::Not))
    })
}
fn preprocessor_expression<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], PreprocessorExpression<'src>> {
    let identifier_parser = identifier().map(|item| PreprocessorExpression::Identifier(item)).boxed();
    let logical_op_parser = logical_op().boxed();
    //let bracketed = preprocessor_expression().delimited_by(
    //    terminal(TerminalToken::OpenBracket),
   //     terminal(TerminalToken::CloseBracket),
    //).map(|exprs| PreprocessorExpression::Bracketed(Box::new(exprs)));
    recursive(|expr|
        choice((
            identifier_parser.clone().then(logical_op_parser).then(expr.clone()).map(|((id, op), right)| {
                match op.token.ty {
                    TokenType::Terminal(TerminalToken::LogicalAnd) => PreprocessorExpression::And(Box::new(id), Box::new(right)),
                    TokenType::Terminal(TerminalToken::LogicalOr) => PreprocessorExpression::Or(Box::new(id), Box::new(right)),
                    TokenType::Terminal(TerminalToken::Not) => PreprocessorExpression::Not(Box::new(right)),
                    _ => unreachable!(),
                }
            }),
            identifier_parser,
            expr.delimited_by(
                terminal(TerminalToken::OpenBracket),
                terminal(TerminalToken::CloseBracket),
            ).map(|exprs| PreprocessorExpression::Bracketed(Box::new(exprs))),
        ))
    )
}
#[test]
fn test_expression() {
    let text = "#if SERVER & CLIENT && (MP || DEV)";
    let tokens = tokenize(text, Flavor::SquirrelRespawn);
    let tokens = tokens.into_tokens().into_iter().skip(1).collect::<Vec<_>>(); 
    println!("Tokens: {:#?}", tokens);
    let parser = preprocessor_expression();
    let result = parser.parse(tokens.as_slice());
    let result = result.into_output();
    if let Some(expr) = result {
        println!("\n\nParsed Expression: {}", expr);
    } else {
        println!("Failed to parse expression");
    }
    let sqparse_result = sqparse::parse(tokens.as_slice(), Flavor::SquirrelRespawn);
    if let Ok(ast) = sqparse_result {
        println!("SQParse AST: {:#?}", ast);
    } else {
        println!("Failed to parse with SQParse - {:?}", sqparse_result.err());
}}

#[test]
fn example_parse() {
    let tokens = example_tokens();
    let parser = parser();
    println!("Tokens: {:#?}", tokens);
    let result = parser.parse(tokens.as_slice());
    println!("{:#?}", result);

}


fn parser<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], Vec<String>> {
    any().to("hello!").map(|x| x.to_string()).repeated().collect::<Vec<_>>()
}

fn start_preproc<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], PreprocessorAST<'src>> {
    let directive = any().filter(|item: &TokenItem<'src>| {
        matches!(item.token.ty, TokenType::Terminal(TerminalToken::PreprocessorIf))
    }).map(|_| PreprocessorAST::If {
        condition: Vec::new(), // Placeholder for condition parsing
        body: vec![],
    });
    directive
    
}