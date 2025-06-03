#[derive(Debug, Clone, PartialEq)]
pub enum Error{
    ExpectedNewlineError,
    UnknownTokenError(String),
    UnwantedTokenWarning(String),//; newlines, redundant
    ExpectedTokenWarning(String),//Missing , on enums. Does not actually break anytihng but anti-pattern
}
impl Error{
    pub fn get_level(&self) -> usize{
        match self{
            Error::ExpectedNewlineError => 2,//This can sometimes not be a problem , but i dont have a way to identify that right now
            Error::UnknownTokenError(_) => 2,
            Error::UnwantedTokenWarning(_) => 1,
            Error::ExpectedTokenWarning(_) => 1,
        }
    }
}