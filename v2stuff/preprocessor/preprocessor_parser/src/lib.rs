use chumsky::{
    prelude::{any, choice, end, recursive, skip_then_retry_until, via_parser, Recursive}, span::{SimpleSpan, Span}, Boxed, IterParser, Parser
};
use parse_common::{accept, ending_or_default, ending_or_fn, errors::ErrorRank, identifier, parsed_element::{default_parsed, ParseVisitable, Parsed, Substitute, Visitable}, terminal, then_expect, then_expect_named, then_or_default, try_both};
use sqparse::{
    ast::Identifier, token::{TerminalToken, TokenType}, TokenItem
};
use crate::{errors::PreprocessorError, statements::PreprocessorExpression};

pub mod errors;
pub mod statements;
pub mod grammar;

#[derive(Debug, PartialEq)]
pub enum PreprocessorAST<'src> {
    If {
        condition: Parsed<PreprocessorExpression>,
        body: Vec<Parsed<PreprocessorAST<'src>>>,
        ends: Box<Parsed<PreprocessorAST<'src>>>,
    },
    ElseIf {
        condition: Parsed<PreprocessorExpression>,
        body: Vec<Parsed<PreprocessorAST<'src>>>,
        ends: Box<Parsed<PreprocessorAST<'src>>>,
    },
    Else {
        body: Vec<Parsed<PreprocessorAST<'src>>>,
        ends: Box<Parsed<PreprocessorAST<'src>>>,
    },
    EndIf,
    Block(Vec<TokenItem<'src>>),
    Error(PreprocessorError<'src>),
}
impl<'src> ParseVisitable for PreprocessorAST<'src>{
    fn visit_parse_children(
        &self,
        function: &mut impl FnMut(&Parsed<Self>),
        pre_visit: &mut impl FnMut(&Parsed<Self>) -> bool,
    ) {
        match self {
            PreprocessorAST::If { condition, body, ends } => {
                for item in body {
                    item.visit_parse(function, pre_visit);
                }
                ends.visit_parse(function, pre_visit);
            }
            PreprocessorAST::ElseIf { condition, body, ends } => {
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
            PreprocessorAST::Error(_) => {}
        }
    }
    fn get_errors(this: &Parsed<Self>) -> Vec<Box<dyn parse_common::errors::ParseError>> {
        let mut child_errs = match &this.item {
            PreprocessorAST::If { condition, body, ends } => {
                condition.find_errors()
            }
            PreprocessorAST::ElseIf { condition, body, ends } => {
                condition.find_errors()
            }
            PreprocessorAST::Error(err) => {
                vec![Box::new(err.clone()) as Box<dyn parse_common::errors::ParseError>]
            }
            _ => vec![]
        };
        child_errs.extend(this.errors.clone());
        child_errs
    }
}
fn any_preproc_token<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], Parsed<TerminalToken>> {
    terminal().filter(
        |item: &Parsed<TerminalToken>| {
            matches!(
                item.item,
                TerminalToken::PreprocessorIf
                    | TerminalToken::PreprocessorElseIf
                    | TerminalToken::PreprocessorElse
            )
        },
    )
}
fn not_preproc<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], TokenItem<'src>> {
    any().filter(
        |item: &TokenItem<'src>| {
            !matches!(
                item.token.ty,
                TokenType::Terminal(TerminalToken::PreprocessorIf)
                    | TokenType::Terminal(TerminalToken::PreprocessorElseIf)
                    | TokenType::Terminal(TerminalToken::PreprocessorElse)
                    | TokenType::Terminal(TerminalToken::PreprocessorEndIf)
            )
        },
    )
}
fn body<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], Parsed<PreprocessorAST<'src>>> {
    not_preproc()
        .repeated().at_least(1).collect::<Vec<_>>()
        .map(|items| {
            let start = items.first().unwrap().token.range.start;
            let end = items.last().unwrap().token.range.end;
            Parsed::new(SimpleSpan::new((), start..end), PreprocessorAST::Block(items))
        })
}

fn branch<'src>(
    recur: impl Parser<'src, &'src [TokenItem<'src>], Parsed<PreprocessorAST<'src>>> + 'src,
) -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<PreprocessorAST<'src>>> {
    let recur = recur.repeated().collect::<Vec<_>>().boxed();
    let start_preproc = accept(TerminalToken::PreprocessorIf, any_preproc_token());
    let start_preproc = start_preproc.then(expression_parser());
    let preproc_else_if = accept(TerminalToken::PreprocessorElseIf, terminal())
        .then(expression_parser());
    let preproc_else = accept(TerminalToken::PreprocessorElse, terminal());
    let end_preproc = accept(TerminalToken::PreprocessorEndIf, terminal()).boxed();
    let preproc_tail = recursive(|tail| {
        let tail = tail.boxed();
        let end = ending_or_fn(recur.clone(), tail.clone(), "endif", || PreprocessorAST::EndIf).boxed();
        choice((
            end_preproc.map(
                |end| end.map_item(|end| PreprocessorAST::EndIf)
            ),
            preproc_else.then(end.clone()).map(
                |(mut start, (body, end))| {
                    let end: Parsed<PreprocessorAST<'_>> = end;
                    start.range.end = end.range.start;
                    start.map_item(
                        |start| PreprocessorAST::Else {
                            body,
                            ends: Box::new(end),
                        }
                    )
                }
            ).boxed(),
            preproc_else_if.then(end).map(
                |((mut start, condition), (body, end))| {
                    start.range.end = end.range.start;
                    start.map_item(
                        |start| PreprocessorAST::ElseIf {
                            condition,
                            body,
                            ends: Box::new(end),
                        }
                    )
                }
            ).boxed(),

        ))})
    .boxed();
    start_preproc.then(ending_or_fn(recur.clone(), preproc_tail.clone(), "endif", || PreprocessorAST::EndIf)).map(
        |(((start, condition)), (body, end))| {
            let mut start = start;
            start.range.end = end.range.start;
            start.map_item(
                |start| PreprocessorAST::If {
                    condition,
                    body,
                    ends: Box::new(end),
                }
            )
        }
    ).boxed()

}

pub fn fallback<'src>() -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<PreprocessorAST<'src>>> {
    any().map(
        |item: TokenItem<'src>| {
            let span = SimpleSpan::from(item.token.range.clone());
            let err = PreprocessorError::UnexpectedToken(item);
            Parsed::new(span, PreprocessorAST::Error(err))
        }
    ).boxed()
}

pub fn preprocessor_parser<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], Vec<Parsed<PreprocessorAST<'src>>>> {
    recursive(|recur| {
        choice((
            branch(recur),
            body()
        )).boxed()
    }).recover_with(via_parser(fallback())).repeated().collect::<Vec<_>>().boxed()
}

fn old_identifier<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], TokenItem<'src>> {
    any().filter(|item: &TokenItem<'src>| matches!(item.token.ty, TokenType::Identifier(_)))
}
fn old_terminal<'src>(
    token: TerminalToken,
) -> Boxed<'src, 'src, &'src [TokenItem<'src>], TokenItem<'src>> {
    any()
        .filter(
            move |item: &TokenItem| matches!(item.token.ty, TokenType::Terminal(t) if t == token),
        )
        .boxed()
}
fn old_logical_op<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], TokenItem<'src>> {
    any().filter(|item: &TokenItem<'src>| {
        matches!(
            item.token.ty,
            TokenType::Terminal(TerminalToken::LogicalAnd)
                | TokenType::Terminal(TerminalToken::LogicalOr)
                | TokenType::Terminal(TerminalToken::Not)
        )
    })
}

/*
End goal is to be able to parse (and report as errors): 
    #if THIS & THAT && (MISSING_BRACKET
    #if THIS THAT
*/



pub fn term_parser<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], Parsed<PreprocessorExpression>> {
    identifier().map(|item| item.map_item(PreprocessorExpression::Identifier)).boxed()
}
pub fn combinator_parser<'src>() -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<TerminalToken>> {
    let subs = vec![
        Substitute::one(TerminalToken::LogicalAnd, TerminalToken::BitwiseAnd),
        Substitute::one(TerminalToken::LogicalOr, TerminalToken::BitwiseOr),
    ];
    terminal().map(
        move |mut item: Parsed<TerminalToken>| {
            item.substitute(subs.clone());
            item
        },
    ).filter(
        |item: &Parsed<TerminalToken>| {
            matches!(item.item, TerminalToken::LogicalAnd | TerminalToken::LogicalOr)
        },
    ).boxed()
}
pub fn combinator_then<'src>(
    recursive: Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<PreprocessorExpression>>,
) -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<PreprocessorExpression>> {
    //Jesus christ
    identifier().then(
    //try_both(combinator_parser(), "Combinator", 
    //    recursive, "Expression").map(|result| {
    then_expect_named(combinator_parser(), "Expression", recursive).map(|(left, right)| {
            /*match result{
                Ok((left, right)) => {
                    let range = left.range;
                    (left, default_parsed(right, PreprocessorExpression::Placeholder, range))
                }
                Err(right) => {
                    (Parsed::new(right.range, TerminalToken::LogicalAnd), right)
                }
            }*/
            let right = default_parsed(right, PreprocessorExpression::Placeholder, left.range);
            (left, right)
        })
    ).map(
        |(left, (comb, expr))| {
            let left = left.map_item(PreprocessorExpression::Identifier);
            comb.map_item(
                |token| match token {
                    TerminalToken::LogicalAnd => PreprocessorExpression::And(Box::new(left), Box::new(expr)),
                    TerminalToken::LogicalOr => PreprocessorExpression::Or(Box::new(left), Box::new(expr)),
                    _ => unreachable!(),
                }
            )
        }
    ).boxed()

}

pub fn expression_parser<'src>() -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<PreprocessorExpression>> {
    let identifier = identifier().map(
        |item| {
            item.map_item(PreprocessorExpression::Identifier)
        }
    ).boxed();
    let open_bracket = accept(TerminalToken::OpenBracket, terminal());
    let close_bracket = accept(TerminalToken::CloseBracket, terminal());
    recursive(|expr| {
        let expr = expr.boxed();
        choice((
            combinator_then(expr.clone()),
            identifier.clone(),
            then_expect_named(accept(TerminalToken::Not, terminal()), "Expression", expr.clone()).map(
                |(not, expr)| {
                    let expr = default_parsed(expr, PreprocessorExpression::Placeholder, not.range);
                    Parsed::new(not.range, PreprocessorExpression::Not(Box::new(expr)))
                }
            ).boxed(),
            open_bracket.then(then_expect(expr, vec![TerminalToken::CloseBracket], close_bracket)).map(|(mut start, (inner, end))| {
                let end = default_parsed(end, TerminalToken::CloseBracket, inner.range);
                start.range.end = end.range.start;
                start.map_item(|start| {
                    PreprocessorExpression::Bracketed(Box::new(inner))
                })
            }).boxed(),
        ))
    }).boxed()
}

#[cfg(test)]
mod testing{

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
        
        let test_text = "#if SERVER && CLIENT && (MP || DEV)
            #if DEV
            int y = 3;
            #else
            int x = 1;
            #endif
            #endif
        #endif";
        let test_tokens = sqparse::tokenize(test_text, sqparse::Flavor::SquirrelRespawn).into_tokens();
        let test_parsed = preprocessor_parser().parse(&test_tokens);
        println!("Parsed Preprocessor: {:#?}", test_parsed);
        let chumsky_errs = test_parsed.errors();
        let mut errs = vec![];

        if let Some(output) = test_parsed.output(){
            for item in output {
                errs.extend(item.find_errors());

                if let PreprocessorAST::Error(err) = &item.item {
                    errs.push(Box::new(err.clone()) as Box<dyn parse_common::errors::ParseError>);
                }

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