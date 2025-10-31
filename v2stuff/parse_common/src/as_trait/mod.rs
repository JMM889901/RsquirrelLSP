use chumsky::{combinator::Then, extra::{self, ParserExtra}, input::Input, prelude::just, primitive::Just, Parser};

mod expectable;
mod err_builder;
mod objects;
pub use objects::errors::ParseError;
pub use objects::errors::ParseErrorDyn;

pub trait ParserExt<'a, In: Input<'a>, Out, Extra: ParserExtra<'a, In> = extra::Default> : Parser<'a, In, Out, Extra> {
    fn test<U, B: Parser<'a, In, U, Extra>>(self, other: B) -> Then<Self, B, Out, U, Extra>
    where
        Self: Sized,
    {
        self.then(other)
    }
}

impl<'a, In: Input<'a>, Out, Extra: ParserExtra<'a, In>, T> ParserExt<'a, In, Out, Extra> for T where 
    T: Parser<'a, In, Out, Extra> + Sized,
{

}


#[cfg(test)]
mod tests {
    use chumsky::prelude::*;

    use crate::as_trait::ParserExt;

    fn parser() -> impl Parser<'static, &'static str, String> {
        just("test").test(just("test2")).map(|(a, b)| format!("{} {}", a, b))
    }
    #[test]
    fn test_ext() {
        let parser = parser();
        todo!()
    }
}