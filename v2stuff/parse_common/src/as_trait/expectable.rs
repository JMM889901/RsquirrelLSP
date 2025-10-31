use std::{marker::PhantomData, ops::Deref};

use chumsky::{container::OrderedSeq, extension::v1::ExtParser, extra::{self, ParserExtra}, input::Input, Parser};

pub trait ExpectOneable<'a, Token> {
    fn isoneof<Seq: OrderedSeq<'a, Token>>(&self, seq: Seq, inp: &Token) -> bool;
}


pub struct ExpectOne<'a, In, Out, P, Seq, CompToken, Extra = extra::Default> {
    pub parser: P,
    pub expected: Seq,
    pub _phantom: PhantomData<&'a (In, Out, Extra, CompToken)>,
}
impl<'a, In, Out, P, Seq, CompToken, Extra> ExtParser<'a, In, Out, Extra> for ExpectOne<'a, In, Out, P, Seq, CompToken, Extra>
where
    P: Parser<'a, In, Out, Extra>,
    In: Input<'a>,
    Extra: ParserExtra<'a, In>,
    Seq: OrderedSeq<'a, CompToken>,
    In::Token: ExpectOneable<'a, CompToken>,
    
{
    fn parse(&self, inp: &mut chumsky::input::InputRef<'a, '_, In, Extra>) -> Result<Out, Extra::Error> {
        todo!()
    }
}
