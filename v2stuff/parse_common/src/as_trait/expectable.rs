use std::{marker::PhantomData, ops::Deref};

use chumsky::{container::OrderedSeq, extension::v1::ExtParser, extra::{self, ParserExtra}, input::Input, Parser};

pub trait ExpectOneable<'a, In> {
    fn isoneof<Seq: OrderedSeq<'a, In>>(&self, seq: Seq, inp: &In) -> bool;
}


pub struct ExpectOne<'a, In, Out, P, Seq, Extra = extra::Default> {
    pub parser: P,
    pub expected: Seq,
    pub _phantom: PhantomData<&'a (In, Out, Extra)>,
}
impl<'a, In, Out, P, Seq, Extra> ExtParser<'a, In, Out, Extra> for ExpectOne<'a, In, Out, P, Seq, Extra>
where
    P: Parser<'a, In, Out, Extra>,
    In: Input<'a>,
    Extra: ParserExtra<'a, In>,
    Seq: OrderedSeq<'a, In>,
{
    fn parse(&self, inp: &mut chumsky::input::InputRef<'a, '_, In, Extra>) -> Result<Out, Extra::Error> {
    
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::prelude::*;

    fn testparser() -> impl Parser<'static, &'static str, String> {
        just("test")
            .map(|s| s.to_string())
    }
    fn expect<'a, In, Out, P, Seq, Extra>(
        parser: P,
        expected: Seq,
    ) -> ExpectOne<'a, In, Out, P, Seq, Extra>
    where
        P: Parser<'a, In, Out, Extra>,
        In: Input<'a>,
        Extra: ParserExtra<'a, In>,
        Seq: OrderedSeq<'a, In>,
    {
        ExpectOne {
            parser,
            expected,
            _phantom: PhantomData,
        }
    }

    #[test]
    fn test_expect() {
        let parser = testparser();
        let expected = vec!["test"];
        let expect = expect(parser, expected.clone());
        expect.into_parser().then(just("test2"))
            .map(|(a, b)| format!("{} {}", a, b));

    }
}