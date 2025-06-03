#[derive(Debug, Clone, PartialEq)]
pub enum Error{
    ExpectedNewlineError,
    UnknownTokenError(String)
}