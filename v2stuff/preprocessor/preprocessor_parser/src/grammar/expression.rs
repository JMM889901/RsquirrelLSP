/*
End goal is to be able to parse (and report as errors):
    #if THIS & THAT && (MISSING_BRACKET
    #if THIS THAT
*/

use chumsky::{
    prelude::{choice, recursive},
    Boxed, Parser,
};
use parse_common::{
    accept, identifier,
    parsed_element::{default_parsed, Parsed, Substitute},
    terminal, then_expect, then_expect_named,
};
use sqparse::{token::TerminalToken, TokenItem};

use crate::statements::PreprocessorExpression;

pub fn term_parser<'src>(
) -> impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, PreprocessorExpression<'src>>> {
    identifier()
        .map(|item| item.map_item(PreprocessorExpression::Identifier))
        .boxed()
}
pub fn combinator_parser<'src>(
) -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<'src, TerminalToken>> {
    let subs = vec![
        Substitute::one(TerminalToken::LogicalAnd, TerminalToken::BitwiseAnd),
        Substitute::one(TerminalToken::LogicalOr, TerminalToken::BitwiseOr),
    ];
    terminal()
        .map(move |mut item: Parsed<TerminalToken>| {
            item.substitute(subs.clone());
            item
        })
        .filter(|item: &Parsed<TerminalToken>| {
            matches!(
                item.item,
                TerminalToken::LogicalAnd | TerminalToken::LogicalOr
            )
        })
        .boxed()
}
pub fn combinator_then<'src>(
    recursive: Boxed<
        'src,
        'src,
        &'src [TokenItem<'src>],
        Parsed<'src, PreprocessorExpression<'src>>,
    >,
) -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<'src, PreprocessorExpression<'src>>> {
    //Jesus christ
    identifier()
        .then(
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
            }),
        )
        .map(|(left, (comb, expr))| {
            let left = left.map_item(PreprocessorExpression::Identifier);
            comb.map_item(|token| match token {
                TerminalToken::LogicalAnd => {
                    PreprocessorExpression::And(Box::new(left), Box::new(expr))
                }
                TerminalToken::LogicalOr => {
                    PreprocessorExpression::Or(Box::new(left), Box::new(expr))
                }
                _ => unreachable!(),
            })
        })
        .boxed()
}

pub fn expression_parser<'src>(
) -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<'src, PreprocessorExpression<'src>>> {
    let identifier = identifier()
        .map(|item| item.map_item(PreprocessorExpression::Identifier))
        .boxed();
    let open_bracket = accept(TerminalToken::OpenBracket, terminal());
    let close_bracket = accept(TerminalToken::CloseBracket, terminal());
    recursive(|expr| {
        let expr = expr.boxed();
        choice((
            combinator_then(expr.clone()),
            identifier.clone(),
            then_expect_named(
                accept(TerminalToken::Not, terminal()),
                "Expression",
                expr.clone(),
            )
            .map(|(not, expr)| {
                let expr = default_parsed(expr, PreprocessorExpression::Placeholder, not.range);
                Parsed::new(not.range, PreprocessorExpression::Not(Box::new(expr)))
            })
            .boxed(),
            open_bracket
                .then(then_expect(
                    expr,
                    vec![TerminalToken::CloseBracket],
                    close_bracket,
                ))
                .map(|(mut start, (inner, end))| {
                    let end = default_parsed(end, TerminalToken::CloseBracket, inner.range);
                    start.range.end = end.range.start;
                    start.map_item(|_| PreprocessorExpression::Bracketed(Box::new(inner)))
                })
                .boxed(),
        ))
    })
    .boxed()
}
