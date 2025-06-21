

use chumsky::{input::InputRef, prelude::*, Boxed, Parser};
use sqparse::{token::{TerminalToken, TokenType}, TokenItem};

use crate::{errors::{expected::{ExpectedArbitary, ExpectedNoMatch}, unknown::UnexpectedToken}, parsed_element::{Expectable, Parsed}};


pub mod parse_state;
pub mod errors;
pub mod parsed_element;

pub fn fallback<'src, T: Clone + 'src>(
    backup_token: T,
) -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<'src, T>> {
    any()
        .map(move |item: TokenItem<'src>| {
            let span = SimpleSpan::from(item.token.range.clone());
            let err = UnexpectedToken {
                token: item,
                range: span,
            };
            let mut token = Parsed::new(span, backup_token.clone());
            token.add_error(err);
            token
        })
        .boxed()
}

pub fn identifier<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, String>> {
    any().try_map(
        |item: TokenItem<'src>, _| {//I should look into using the builtin span, its based on the token index NOT the text range like token.range is 
            if let TokenType::Identifier(id) = item.token.ty {
                return Ok(Parsed {
                    range: item.token.range.clone().into(),
                    item: id.to_string(),
                    polluted: 0,
                    errors: vec![],
                })
            }
            Err(EmptyErr::default())
        },
    )
}

pub fn terminal<'src>() -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<'src, TerminalToken>> {
    any().try_map(
        |item: TokenItem<'src>, _| {//I should look into using the builtin span, its based on the token index NOT the text range like token.range is 
            if let TokenType::Terminal(token) = item.token.ty {
                return Ok(Parsed {
                    range: item.token.range.clone().into(),
                    item: token,
                    polluted: 0,
                    errors: vec![],
                })
            }
            Err(EmptyErr::default())
        },
    ).boxed()
}
pub fn accept<'src, T: Expectable>(
    token: T,
    parser: impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, T>> + 'src,
) -> impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, T>> {
    parser.filter(
        move |item: &Parsed<'src, T>| {
            item.item.matches(&token)
        },
    )
}

/// Allows the match to pass, if this IS a terminal, but not the expected
pub fn expect_terminal<'src>(
    token: TerminalToken,
) -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<'src, TerminalToken>> {
    terminal().map(
        move |mut item: Parsed<'src, TerminalToken>| {
            item.expect(vec![token]);
            item
        },
    ).boxed()
}

pub fn expect_match<'src, T: Expectable + Clone + 'src> (
    parser: impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, T>> + 'src,
    expected: Vec<T>,
) -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<'src, T>> 
where
{
    parser.map(
        move |mut item: Parsed<'src, T>| {
            item.expect(expected.clone());
            item
        },
    ).boxed()
}
pub fn token<'src>() -> Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<'src, TokenItem<'src>>> {
    any().map(
        |item: TokenItem<'src>| {
            Parsed {
                range: SimpleSpan::from(item.token.range.clone()),
                item,
                polluted: 0,
                errors: vec![],
            }
        }
    ).boxed()
}



pub fn then_expect<'src, T, V>(
    base_parser: impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, T>> + 'src,
    expected: Vec<V>,
    second_parser: impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, V>> + 'src,
) -> impl Parser<'src, &'src [TokenItem<'src>], (Parsed<'src, T>, Option<Parsed<'src, V>>)>
where
    V: Expectable + Clone + 'src,
{
    base_parser.
    then(expect_match(second_parser, expected.clone()).or_not())
    .map(move |(mut first, second)| {
        if let Some(second) = second {
            (first, Some(second))
        } else {
            let err = ExpectedNoMatch::new(expected.clone(), first.range.start);
            first.add_error(err);
            (first, None)
        }
    })
}


pub fn then_expect_named<'src, T, V>(
    base_parser: impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, T>> + 'src,
    expected: &'src str,
    second_parser: impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, V>> + 'src,
) -> impl Parser<'src, &'src [TokenItem<'src>], (Parsed<'src, T>, Option<Parsed<'src, V>>)> {
    base_parser.then(second_parser.or_not())
    .map(move |(mut first, second)| {
        if let Some(second) = second {
            (first, Some(second))
        } else {
            let err = ExpectedArbitary::new(expected.to_string(), first.range.start);
            first.add_error(err);
            (first, None)
        }
    })
}
pub fn ending_or_default<'src, T, V: Clone>(
    base_parser: impl Parser<'src, &'src [TokenItem<'src>], Vec<Parsed<'src, T>>> + 'src,
    second_parser: impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, V>> + 'src,
    expected: &'src str,
    default: V,
) -> impl Parser<'src, &'src [TokenItem<'src>], (Vec<Parsed<'src, T>>, Parsed<'src, V>)> {
    base_parser.then(second_parser.or_not())
        .map(move |(mut first, second)| {
            if let Some(second) = second {
                (first, second)
            } else {
                let default_parsed = Parsed::new(first.last().map(|x| x.range).unwrap(), default.clone());
                let err = ExpectedArbitary::new(expected.to_string(), first.last().map_or_else(|| 0, |x| x.range.start));
                if let Some(last) = first.last_mut() {
                    last.add_error(err);
                }
                (first, default_parsed)
            }
        })
}

pub fn ending_or_fn<'src, T, V>(
    base_parser: impl Parser<'src, &'src [TokenItem<'src>], Vec<Parsed<'src, T>>> + 'src,
    second_parser: impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, V>> + 'src,
    expected: &'src str,
    default: impl Fn() -> V
) -> impl Parser<'src, &'src [TokenItem<'src>], (Vec<Parsed<'src, T>>, Parsed<'src, V>)> {
    base_parser.then(second_parser.or_not())
        .try_map(move |(mut first, second), _| {
            if let Some(second) = second {
                Ok((first, second))
            } else {
                let mut default_parsed = Parsed::new(first.last().map(|x| x.range).unwrap(), default());
                let err = ExpectedArbitary::new(expected.to_string(), first.last().map_or_else(|| 0, |x| x.range.start));
                if first.last_mut().is_some() {
                    //last.add_error(err.clone());
                } else {
                    return Err(EmptyErr::default());//We matched rule 1 0 times and didnt match rule 2, so presumably nothing matched
                }
                default_parsed.add_error(err);
                Ok((first, default_parsed))
            }
        })

}



pub fn then_or_default<'src, T, V: Clone>(
    base_parser: impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, T>> + 'src,
    second_parser: impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, V>> + 'src,
    expected: &'src str,
    default: V,
) -> impl Parser<'src, &'src [TokenItem<'src>], (Parsed<'src, T>, Parsed<'src, V>)> {
    base_parser.then(second_parser.or_not())
        .map(move |(mut first, second)| {
            if let Some(second) = second {
                (first, second)
            } else {
                let default_parsed = Parsed::new(first.range, default.clone());
                let err = ExpectedArbitary::new(expected.to_string(), first.range.start);
                first.add_error(err);
                (first, default_parsed)
            }
        })
}

type AndMaybe<'src, T, V> = (Parsed<'src, T>, Option<Parsed<'src, V>>);

//Try both, if both exists match, if one exists match and register the error, fail if neither
//This is really quite pricy, honestly not sure if its worth it over traditional "skip until the error goes away" recovery
pub fn try_both<'src, 
T: 'src,
V: 'src,//Expectable explicitly refers to comparing against a set of expected, we dont need it for this which tests did it match
>(first_parser: impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, T>> + 'src,
    first_expected: &'src str,
    second_parser: Boxed<'src, 'src, &'src [TokenItem<'src>], Parsed<'src, V>>,
    second_expected: &'src str,
) -> impl Parser<'src, &'src [TokenItem<'src>], Result<AndMaybe<'src, T, V>, Parsed<'src, V>>> {
    let first_exists = first_parser.then(second_parser.clone().or_not()).map(
        move |(mut first, second)| {
            if let Some(second) = second {
                Ok((first, Some(second)))
            } else {
                let err = ExpectedArbitary::new(second_expected.to_string(), first.range.start);
                first.add_error(err);
                Ok((first, None))
            }
        }
    );
    let second_exists = second_parser.map(
        move |mut second| {
            let err = ExpectedArbitary::new(first_expected.to_string(), second.range.start);
            second.add_error(err);
            Err(second)
        }
    );
    first_exists.or(second_exists)
}





/// You may be asking yourself "Isnt this literally the opposite of what a peg is meant to be".
/// Shut up nerd
pub fn choose_best<'src, T: Expectable + Clone>(
    parsers: Vec<impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, T>> + 'src>,
) -> impl Parser<'src, &'src [TokenItem<'src>], Parsed<'src, T>> {
    custom(
        move |inputref: &mut InputRef<_, _>| {
            let save = inputref.save();
            let mut best_position = inputref.save();
            let mut best: Option<Parsed<'src, T>> = None;
            for parser in &parsers {
                if let Ok(result) = inputref.parse(parser) {
                    if let Some(ref best_result) = best {
                        if result.polluted < best_result.polluted {
                            best_position = inputref.save();
                            best = Some(result);
                        }
                    } else {
                        best_position = inputref.save();
                        best = Some(result);
                    }
                }
                inputref.rewind(save.clone());
            }
            if let Some(best_result) = best {
                inputref.rewind(best_position);
                Ok(best_result)
            } else {
                //I dont think i need to rewind here
                Err(EmptyErr::default())
            }
        }
    )
}