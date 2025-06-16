use crate::{grammar::branch::branch, statements::PreprocessorExpression};
use chumsky::{
    prelude::{any, choice, recursive, via_parser},
    span::{SimpleSpan, Span},
    IterParser, Parser,
};
use parse_common::{
    errors::ParseError,
    fallback,
    parsed_element::{ParseVisitable, Parsed},
    terminal,
};
use sqparse::{
    token::{TerminalToken, TokenType},
    TokenItem,
};

pub mod errors;
pub mod grammar;
pub mod statements;

#[derive(Debug, PartialEq, Clone)]
pub enum PreprocessorAST<'src> {
    If {
        condition: Parsed<'src, PreprocessorExpression<'src>>,
        body: Vec<Parsed<'src, PreprocessorAST<'src>>>,
        ends: Box<Parsed<'src, PreprocessorAST<'src>>>,
    },
    ElseIf {
        condition: Parsed<'src, PreprocessorExpression<'src>>,
        body: Vec<Parsed<'src, PreprocessorAST<'src>>>,
        ends: Box<Parsed<'src, PreprocessorAST<'src>>>,
    },
    Else {
        body: Vec<Parsed<'src, PreprocessorAST<'src>>>,
        ends: Box<Parsed<'src, PreprocessorAST<'src>>>,
    },
    EndIf,
    Block(Vec<TokenItem<'src>>),
    Error,
}
impl<'src> ParseVisitable<'src> for PreprocessorAST<'src> {
    fn visit_parse_children(
        &'src self,
        function: &mut impl FnMut(&'src Parsed<'src, Self>),
        pre_visit: &mut impl FnMut(&'src Parsed<'src, Self>) -> bool,
    ) {
        match self {
            PreprocessorAST::If { body, ends, .. } => {
                for item in body {
                    item.visit_parse(function, pre_visit);
                }
                ends.visit_parse(function, pre_visit);
            }
            PreprocessorAST::ElseIf { body, ends, .. } => {
                for item in body {
                    item.visit_parse(function, pre_visit);
                }
                ends.visit_parse(function, pre_visit);
            }
            PreprocessorAST::Else { body, ends } => {
                for item in body {
                    item.visit_parse(function, pre_visit);
                }
                ends.visit_parse(function, pre_visit);
            }
            PreprocessorAST::EndIf | PreprocessorAST::Block(_) => {}
            PreprocessorAST::Error => {}
        }
    }
    fn get_errors(this: &'src Parsed<'src, Self>) -> Vec<Box<dyn ParseError + 'src>> {
        let mut child_errs = match &this.item {
            PreprocessorAST::If { condition, .. } => condition.find_errors(),
            PreprocessorAST::ElseIf { condition, .. } => condition.find_errors(),
            _ => vec![],
        };
        child_errs.extend(this.errors.clone());
        child_errs
    }
}

fn any_preproc_token<'src>(
) -> impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, TerminalToken>> {
    terminal().filter(|item: &Parsed<TerminalToken>| {
        matches!(
            item.item,
            TerminalToken::PreprocessorIf
                | TerminalToken::PreprocessorElseIf
                | TerminalToken::PreprocessorElse
        )
    })
}
fn not_preproc<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], TokenItem<'src>> {
    any().filter(|item: &TokenItem<'src>| {
        !matches!(
            item.token.ty,
            TokenType::Terminal(TerminalToken::PreprocessorIf)
                | TokenType::Terminal(TerminalToken::PreprocessorElseIf)
                | TokenType::Terminal(TerminalToken::PreprocessorElse)
                | TokenType::Terminal(TerminalToken::PreprocessorEndIf)
        )
    })
}
///Generic body, skip tokens because we dont care about them yet
fn body<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, PreprocessorAST<'src>>> {
    not_preproc()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .map(|items| {
            let start = items.first().unwrap().token.range.start;
            let end = items.last().unwrap().token.range.end;
            Parsed::new(
                SimpleSpan::new((), start..end),
                PreprocessorAST::Block(items),
            )
        })
}

pub fn preprocessor_parser<'src>(
) -> impl Parser<'src, &'src [TokenItem<'src>], Vec<Parsed<'src, PreprocessorAST<'src>>>> {
    recursive(|recur| choice((branch(recur), body())).boxed())
        .recover_with(via_parser(fallback(PreprocessorAST::Error)))
        .repeated()
        .collect::<Vec<_>>()
        .boxed()
}

#[cfg(test)]
mod testing {

    use crate::grammar::expression::expression_parser;

    use super::*;
    pub fn test_expression() -> &'static str {
        "SERVER && CLIENT && (MP || DEV)"
    }
    #[test]
    fn test_expression_parser() {
        let text = test_expression();
        let tokens = sqparse::tokenize(text, sqparse::Flavor::SquirrelRespawn).into_tokens();
        let parsed = expression_parser().parse(&tokens).unwrap();
        println!("Parsed Expression: {:#?}", parsed);

        let test_text = "#if SERVER && CLIENT & (MP | DEV)
            #if DEV
            int y = 3;
            #else
            int x = 1;
            #endif
            #endif
        #endif";
        let test_tokens =
            sqparse::tokenize(test_text, sqparse::Flavor::SquirrelRespawn).into_tokens();
        let test_parsed = preprocessor_parser().parse(&test_tokens);
        println!("Parsed Preprocessor: {:#?}", test_parsed);
        let chumsky_errs = test_parsed.errors();
        let mut errs = vec![];

        if let Some(output) = test_parsed.output() {
            for item in output {
                errs.extend(item.find_errors());
            }
        }
        for err in chumsky_errs {
            println!("Chumsky Error: {:?}", err);
        }
        for err in errs {
            println!("Error: {}", err.text());
            println!("Range: {:?}", err.range());
        }
    }
}
