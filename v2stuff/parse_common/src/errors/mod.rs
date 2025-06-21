//Some inbuilt error types

pub mod substitute;
pub mod expected;
pub mod unknown;
use std::{fmt::{Debug}};

use chumsky::span::SimpleSpan;


pub trait ErrorRank {
    /// Using usize here just so you can arbitrarily rank these 
    /// No actual errors, should really just be used for formatting suggestions and such
    fn fine() -> usize {
        0
    }
    /// A substituted match (or similar), no match found but a predicted alternative was used (Still an error most likely)
    fn substitute() -> usize{
        1
    }
    /// Unable to match, but you can recover
    fn expected() -> usize {
        2
    }
    /// Only when using the fallback strategy, bad
    fn unknown_token() -> usize {
        3
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct DefaultErrorRank;
impl ErrorRank for DefaultErrorRank {}
pub enum ErrorLevel {
    None,
    Info,
    Warning,
    Error,//The rank and severity of an error are seperate
}//The parser can recover from a full error (such as using & instead of &&), but the rsquirrel compiler cannot.
//TODO: do i need this to be send/sync?
pub trait TryClone {
    fn try_clone<'src>(&self) -> Option<Box<dyn ParseError + 'src>> where Self: 'src {
        None//Fucking traits
    }
}

pub trait ParseError : TryClone + Debug + Send + Sync {
    fn rank(&self) -> usize;
    fn level(&self) -> ErrorLevel;
    fn text(&self) -> String;//Redundant
    fn range(&self) -> SimpleSpan;
    fn type_id(&self) -> std::any::TypeId where Self: 'static {
        std::any::TypeId::of::<Self>()
    }
}
/* 
impl Clone for Box<dyn ParseError> {
    fn clone(&self) -> Self {
        if let Some(cloned) = self.as_ref().try_clone() {
            cloned
        } else {
            panic!("Cannot clone a ParseError that does not implement TryClone");
        }
    }
}*/
impl Clone for Box<dyn ParseError + '_> {
    fn clone(&self) -> Self {
        if let Some(cloned) = self.as_ref().try_clone() {
            cloned
        } else {
            panic!("Cannot clone a ParseError that does not implement TryClone");
        }
    }
}
impl<T: Clone> TryClone for T where T: ParseError {
    fn try_clone<'src>(&self) -> Option<Box<dyn ParseError + 'src>> where Self: 'src {
        Some(Box::new(self.clone()))
    }
}



//pub struct ElementError(pub Box<dyn ParseError>);
impl PartialEq for dyn ParseError {//This is specifically for finding the worst error, this is not a real equality check
    fn eq(&self, other: &Self) -> bool {
        self.rank() == other.rank()
    }
}
impl PartialOrd for dyn ParseError {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.rank().cmp(&other.rank()))
    }
}


#[cfg(test)]
mod test_thing{

    use crate::parsed_element::Parsed;

    use super::*;

    pub struct CustomErrorRank;
    impl ErrorRank for CustomErrorRank {
        fn fine() -> usize {
            10
        }
        fn substitute() -> usize {
            20
        }
        fn expected() -> usize {
            30
        }
    }
    impl CustomErrorRank {
        pub fn test_error() -> usize {
            100
        }
    }
    #[derive(Clone, PartialEq, Eq)]
    pub struct TestError;

    impl Debug for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Test error")
        }
    }
    
    impl ParseError for TestError {
        fn rank(&self) -> usize {
            CustomErrorRank::test_error()
        }
        fn level(&self) -> ErrorLevel {
            ErrorLevel::Error
        }
        fn text(&self) -> String {
            "Test error".to_string()
        }
        fn range(&self) -> SimpleSpan {
            SimpleSpan { start: 0, end: 10, context: () }
        }
    }
    #[derive(Clone, PartialEq, Eq)]
    pub struct TestSubstituteError;

    impl Debug for TestSubstituteError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Test substitute error")
        }
    }
    impl ParseError for TestSubstituteError {
        fn rank(&self) -> usize {
            DefaultErrorRank::substitute()
        }
        fn level(&self) -> ErrorLevel {
            ErrorLevel::Warning
        }
        fn text(&self) -> String {
            "Test substitute error".to_string()
        }
        fn range(&self) -> SimpleSpan {
            SimpleSpan { start: 0, end: 10, context: () }
        }
    }
    #[test]
    fn test_error_rank() {
        let mut matchelement = Parsed{
            range: SimpleSpan { start: 0, end: 10, context: () },
            item: (),
            polluted: 0,
            errors: vec![],
        };
        let error = TestError;
        let substitute_error = TestSubstituteError;

        matchelement.add_error(error);
        matchelement.add_error(substitute_error);
        let errs = matchelement.errors.iter().map(|e| e.rank()).collect::<Vec<_>>();
        assert_eq!(errs, vec![100, 20]); 
        
        
    }

}