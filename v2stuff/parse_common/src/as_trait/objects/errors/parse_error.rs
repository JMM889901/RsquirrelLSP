use std::{collections::HashMap, fmt::Display, os::raw};

use chumsky::span::SimpleSpan;
use sqparse::{token::Token, TokenItem};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorPos{
    File,
    Range(SimpleSpan),
}



pub struct TextSuggestion {
    pub description: Option<String>, 
    pub suggestion: Vec<FixStep>,
    pub start: usize, // Start token index
}
impl TextSuggestion {
    // I straight up do not feel like making this generic, so its not
    //Literally nobody else will use this code, what are you going to do about it?
    pub fn build_section(&self, raw: &str, tokens: &[TokenItem<'_>], start_idx: usize, start_char: usize) -> (String, usize, usize) { //(Text block, End token index)
        let mut result = String::new();
        let mut end_char = start_char;
        let mut end_idx = start_idx;
        for step in &self.suggestion {
            let range = step.char_range(tokens);
            // Add the whitespace (and i guess technically text can be here)
            result.push_str(&raw[end_char..range.start]);
            eprintln!("Prefix: '{}'", &raw[end_char..range.start]);
            end_char = range.end;

            result.push_str(&step.string());
            eprintln!("Step: '{}'", step.string());
            assert!(end_idx <= step.end_idx(end_idx), "FixStep end index must be greater than or equal to the previous end index");
            end_idx = step.end_idx(end_idx);
            eprint!("end_idx: {}, end_char: {}\n", end_idx, end_char);
        }
        (result, end_idx+1, end_char)
    }
}

/// Can be converted to a drag-and-drop replacement for an actual token
/// Should probably not have any whitespace
pub trait ToArbToken<Tokens : ?Sized> {
    fn to_arbitrary_token(&self, tokens: &Tokens, idx: usize) -> ArbitraryToken;
}

pub struct ArbitraryToken{
    pub token: String,
    // When i made this i definitely thought i would need more here
    // But i don't think i do, so this is just a string
}
pub enum FixStep {
    //Stick text there
    Insert(String, usize),//Start pos

    //Replace existing token, this *should* preserve spaces between tokens and such
    //Replacing multiple tokens can be done by doing this a couple times
    //Otherwise things get weird when you are replacing two tokens with one
    //I mean, this is also weird but like, i've already done this so cope
    ReplaceToken(ArbitraryToken, SimpleSpan), 

    RemoveTokens(SimpleSpan),//Index range
}
impl FixStep {
    pub fn string(&self) -> String {
        match self {
            FixStep::Insert(str, _) => str.clone(),
            FixStep::ReplaceToken(token, _) => token.token.clone(),
            FixStep::RemoveTokens(_) => String::new(), 
        }
    }

    pub fn char_range(&self, tokens: &[TokenItem<'_>]) -> std::ops::Range<usize> {
        match self {
            FixStep::Insert(_, start) => *start..*start,//Position in the original text remains the same
            FixStep::ReplaceToken(_, span) => tokens[span.end].token.range.clone(),
            //FixStep::Replace(_, span) => span.end,
            FixStep::RemoveTokens(span) => tokens[span.end].token.range.clone(),
            //FixStep::Remove(span) => span.end,
        }
    }
    pub fn end_idx(&self, current_idx: usize) -> usize {
        match self {
            FixStep::Insert(_, _) => current_idx,
            FixStep::ReplaceToken(_, span) => span.end,
            FixStep::RemoveTokens(span) => span.end,
        }
    }
}

pub trait ParseErrorDyn<T> : Display + 'static + ParseError + ToSuggestion<T> {
}
impl<A, B> ParseErrorDyn<A> for B where
    B: ParseError + Display + ToSuggestion<A>,
{
} 
pub trait ParseError: 'static + Display{
   //fn severity(&self) -> u8;
    //Ranking handled elsewhere now
    fn position(&self) -> ErrorPos;
}
pub trait ToSuggestion<T>{
    fn suggest<'a>(&self, tokens: &'a [T], raw: &String) -> Vec<TextSuggestion> {
        vec![]
    }
}

impl PartialEq for dyn ParseError {
    fn eq(&self, other: &Self) -> bool {
        // Yeah this is just as sketchy as it looks
        // I could at least make a dedicated comp string or something
        // Dyn wont let me do anything involving sized types, so regular eq is off the table
        // And i don't feel like doing downcasting again
        self.to_string() == other.to_string() && self.position() == other.position()
    }
}

/// Takes a SORTED (by the start index) list of changes
pub fn apply_changes(source: &str, tokens: &[TokenItem<'_>], changes: Vec<TextSuggestion>) -> String {
    let mut result = String::new();
    let mut last_token = 0;
    let mut last_char = 0;
    for change in changes {
        let (text_block, end_idx, end_char) = change.build_section(source, tokens, last_token, last_char);
        result.push_str(&text_block);
        last_token = end_idx;
        last_char = end_char;

    }
    result.push_str(&source[last_char..]);
    result
    
}


#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::span::SimpleSpan;
    use sqparse::{token, tokenize};

    #[test]
    fn test_build() {
        let text = "#if SERVER &    CLIENT || UI";
        let tokens = tokenize(text, sqparse::Flavor::SquirrelRespawn).into_tokens();
        for (idx, token) in tokens.iter().enumerate() {
            println!("Token {}: {:?}", idx, token);
        }
        let step = FixStep::ReplaceToken(
            ArbitraryToken { token: "&&".to_string() },
            SimpleSpan::from(2..2),//Replace just the & token, should preserve the fucked up spaces
        );
        let suggestion = TextSuggestion {
            description: Some("Replace & with &&".to_string()),
            suggestion: vec![step],
            start: 2,
        };
        let res = apply_changes(text, tokens.as_slice(), vec![suggestion]);
        println!("{}", res);
        assert!(res == "#if SERVER &&    CLIENT || UI", "Expected '#if SERVER &&    CLIENT || UI', got '{}'", res);
    }
}