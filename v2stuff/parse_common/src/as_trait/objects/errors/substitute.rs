use std::{fmt::Display, ops::Sub};

use chumsky::span::SimpleSpan;
use sqparse::{token::{TerminalToken, Token}, TokenItem};

use crate::as_trait::{objects::errors::{parse_error::{ErrorPos, FixStep, TextSuggestion, ToArbToken, ToSuggestion}, static_repr::{StaticRepr, StaticReprReq}}, ParseError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Substituted<Wanted: 'static, Real: 'static = Wanted> {
    pub original: Real,
    pub substitute: Wanted,
    pub wanted: Option<Vec<Wanted>>,
    pub position: SimpleSpan,//Token index
}
impl<Wanted: StaticRepr<Token>, Real: StaticRepr<Token>, Token> ToSuggestion<Token> for Substituted<Wanted, Real>
where
    Wanted: ToArbToken<[Token]>,
{
    
    fn suggest(&self, tokens: &[Token], raw: &String) -> Vec<TextSuggestion> {
        let tok = &tokens[self.position.into_range()];
        let replace = self.substitute.to_arbitrary_token(tok, self.position.start);
        let description = Some(format!("Replace {} with {}", self.original, self.substitute));
        let step = FixStep::ReplaceToken(replace, self.position);
        let suggestion = TextSuggestion{
            suggestion: vec![step],
            description,
            start: self.position.start
        };
        vec![suggestion]
        
    }
}

impl<Wanted: StaticReprReq, Real: StaticReprReq> ParseError for Substituted<Wanted, Real>
{
    fn position(&self) -> ErrorPos {
        ErrorPos::Range(self.position)
    }

}

impl<Wanted: 'static + Display, Real: 'static + Display> Display for Substituted<Wanted, Real> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(wanted) = &self.wanted {
            let wanted_str: Vec<String> = wanted.iter().map(|w| w.to_string()).collect();
            let wanted_str = wanted_str.join(", ");
            write!(f, "Substituted '{}' with '{}', expected one of {:?}", self.original, self.substitute, wanted_str)
        } else {
            write!(f, "Substituted '{}' with '{}'", self.original, self.substitute)
        }
    }
}
impl<T: 'static > Substituted<T, T> 
{
    pub fn simple<Token>(original: Token, substitute: T, wanted: Option<Vec<T>>, position: SimpleSpan) -> Self
    where
        Token: Into<T>,
    {
        Substituted {
            original: original.into(),
            substitute,
            wanted,
            position,
        }
    }
}

pub struct WrappedTerminal(pub TerminalToken);
impl Display for WrappedTerminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_str())
    }
}
impl<'a> TryFrom<Token<'a>> for WrappedTerminal {
    type Error = Token<'a>;

    fn try_from(value: Token<'a>) -> Result<Self, Self::Error> {
        match value.ty {
            sqparse::token::TokenType::Terminal(a) => Ok(WrappedTerminal(a)),
            _ => Err(value),
        }
    }
}







#[cfg(test)]
mod tests {

    use chumsky::Parser;
    use sqparse::{tokenize, TokenItem};
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DummyEnum {
        Thing,
        OtherThing,
        ThirdThing,
    }
    impl PartialEq<TokenItem<'_>> for DummyEnum {
        fn eq(&self, other: &TokenItem<'_>) -> bool {
            true
        }
    }
    impl ToArbToken<[TokenItem<'_>]> for DummyEnum {
        fn to_arbitrary_token(&self, tokens: &[TokenItem<'_>], idx: usize) -> crate::as_trait::objects::errors::parse_error::ArbitraryToken {
            ArbitraryToken{
                token: self.to_string(),
            }
        }
    }
    impl Display for DummyEnum {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                DummyEnum::Thing => write!(f, "DummyEnum::Thing"),
                DummyEnum::OtherThing => write!(f, "DummyEnum::OtherThing"),
                DummyEnum::ThirdThing => write!(f, "DummyEnum::ThirdThing"),
            }
        }
    }
    impl From<TokenItem<'_>> for DummyEnum {
        fn from(item: TokenItem<'_>) -> Self {
            DummyEnum::Thing
        }
    }


    use crate::as_trait::{objects::errors::{parse_error::ArbitraryToken, static_repr::StaticRepr}, ParseErrorDyn};

    use super::*;
    fn guh(input: TokenItem<'_>) -> Box<dyn ParseErrorDyn<TokenItem<'_>> + 'static> {

        //Box::new(Substituted::simple(input, DummyEnum::Thing, Some(vec![DummyEnum::OtherThing]), SimpleSpan::from(0..10)))
        Box::new(Substituted::simple(input, DummyEnum::OtherThing, Some(vec![DummyEnum::ThirdThing]), SimpleSpan::from(0..10)))
    }

    #[test]
    fn test_substituted_display() {
        let original = "original_token";
        let substitute = "substituted_token";
        let wanted = Some(vec!["expected_token"]);
        let position = SimpleSpan::from(0..10);
        let dummytext = "int";
        let tokens = tokenize(dummytext, sqparse::Flavor::SquirrelRespawn).unwrap();
        let error = guh(tokens[0].clone());

        println!("{}", error);
    }
}

