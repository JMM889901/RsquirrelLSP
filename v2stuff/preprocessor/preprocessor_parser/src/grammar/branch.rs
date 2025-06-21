use chumsky::{
    prelude::{choice, recursive},
    Boxed, IterParser, Parser,
};
use parse_common::{accept, ending_or_fn, parsed_element::Parsed, terminal};
use sqparse::{token::TerminalToken, TokenItem};

use crate::{
    any_preproc_token, grammar::expression::expression_parser, statements::PreprocessorExpression,
    PreprocessorAST,
};

fn start_block<'src>() -> impl Parser<
    'src,
    &'src [TokenItem<'src>],
    (
        Parsed<'src, TerminalToken>,
        Parsed<'src, PreprocessorExpression<'src>>,
    ),
> {
    let start_preproc = accept(TerminalToken::PreprocessorIf, any_preproc_token());
    start_preproc.then(expression_parser())
}

fn else_if_block<'src>() -> impl Parser<
    'src,
    &'src [TokenItem<'src>],
    (
        Parsed<'src, TerminalToken>,
        Parsed<'src, PreprocessorExpression<'src>>,
    ),
> {
    let else_preproc = accept(TerminalToken::PreprocessorElse, any_preproc_token());
    else_preproc.then(expression_parser())
}

/// Preprocessor branches
pub fn branch<'src>(
    recur: impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, PreprocessorAST<'src>>> + 'src,
) -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<'src, PreprocessorAST<'src>>> {
    let recur = recur.repeated().collect::<Vec<_>>().boxed();

    let start = start_block().boxed();
    let elif = else_if_block().boxed();
    let preproc_else = accept(TerminalToken::PreprocessorElse, terminal()).boxed();
    let end_preproc = accept(TerminalToken::PreprocessorEndIf, terminal()).boxed();

    let preproc_tail = recursive(|tail| {
        let tail = tail.boxed();
        let end = ending_or_fn(recur.clone(), tail.clone(), "endif", || {
            PreprocessorAST::EndIf
        })
        .boxed();
        choice((
            end_preproc.map(|end| end.map_item(|_| PreprocessorAST::EndIf)),
            preproc_else
                .then(end.clone())
                .map(|(start, (body, end))| {
                    //This is just to make type signatures not get confused, because recursive
                    let mut start: Parsed<TerminalToken> = start;
                    let end: Parsed<PreprocessorAST<'_>> = end;
                    let mut body: Vec<Parsed<'_, PreprocessorAST<'_>>> = body;
                    //TODO: This seems a bit sketchy, if i wind up with weird starts this is probably why
                    if let Some(b) = body.first_mut() {
                        b.range.start = start.range.end;
                    }
                    start.range.end = end.range.start;
                    start.map_item(|_| PreprocessorAST::Else {
                        body,
                        ends: Box::new(end),
                    })
                }),
            elif.then(end).map(|((mut start, condition), (mut body, end))| {
                if let Some(b) = body.first_mut() {
                    b.range.start = condition.range.end;
                }//I dont think i need to update last as well

                start.range.end = end.range.start;

                start.map_item(|_| PreprocessorAST::ElseIf {
                    condition,
                    body,
                    ends: Box::new(end),
                })
            }),
        ))
    })
    .boxed();
    start
        .then(ending_or_fn(
            recur.clone(),
            preproc_tail.clone(),
            "endif",
            || PreprocessorAST::EndIf,
        ))
        .map(|((start, condition), (mut body, end))| {
            let mut start = start;
            if let Some(b) = body.first_mut() {
                b.range.start = condition.range.end;
            }
            start.range.end = end.range.end;
            start.map_item(|_| PreprocessorAST::If {
                condition,
                body,
                ends: Box::new(end),
            })
        })
        .boxed()
}
