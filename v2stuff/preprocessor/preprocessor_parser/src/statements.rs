use parse_common::errors::ErrorRank;
use parse_common::parsed_element::{ParseVisitable, Parsed, Visitable};
use sqparse::{token::TokenType, TokenItem};
use std::cmp::{max, min};
use std::ops::Range;
use paste::paste;




use crate::errors::PreprocessorError;
#[derive(Debug, PartialEq)]
pub enum PreprocessorExpression{
    Identifier(String),
    Placeholder,//Syntax error, missing identifier
    Bracketed(Box<Parsed<PreprocessorExpression>>),
    And(Box<Parsed<PreprocessorExpression>>, Box<Parsed<PreprocessorExpression>>),
    Or(Box<Parsed<PreprocessorExpression>>, Box<Parsed<PreprocessorExpression>>),   
    Not(Box<Parsed<PreprocessorExpression>>),
}
impl ParseVisitable for PreprocessorExpression {
    fn visit_parse_children(
        &self,
        function: &mut impl FnMut(&Parsed<PreprocessorExpression>),
        pre_visit: &mut impl FnMut(&Parsed<PreprocessorExpression>) -> bool,
    ) {
        match self {
            PreprocessorExpression::Identifier(_) | PreprocessorExpression::Placeholder => {}
            PreprocessorExpression::Bracketed(expr) => {
                expr.visit_parse(function, pre_visit);
            }
            PreprocessorExpression::And(left, right) => {
                left.visit_parse(function, pre_visit);
                right.visit_parse(function, pre_visit);
            }
            PreprocessorExpression::Or(left, right) => {
                left.visit_parse(function, pre_visit);
                right.visit_parse(function, pre_visit);
            }
            PreprocessorExpression::Not(expr) => {
                expr.visit_parse(function, pre_visit);
            }
        }
    }

}
/*
pub type PreprocElement<'src, T> =
    MatchElement<T, PreprocessorError<'src>>; 
pub type PreprocElement2<'src> = TokenElement<'src, PreprocessorAST<'src>, PreprocessorError<'src>>;
pub type PreprocExpression<'src> =
    TokenElement<'src, PreprocessorExpression<'src>, PreprocessorError<'src>>;
#[derive(Debug, Clone, PartialEq)]
pub enum PreprocessorAST<'ast> {
    If {
        condition: Vec<TokenItem<'ast>>,
        body: Vec<PreprocElement2<'ast>>,
    },
    ElseIf {
        condition: Vec<TokenItem<'ast>>,
        body: Vec<PreprocElement2<'ast>>,
    },
    Else {
        body: Vec<PreprocElement2<'ast>>,
    },
    EndIf,
    Block(Vec<TokenItem<'ast>>),
}
#[derive(Debug, Clone, PartialEq)]
pub enum PreprocessorExpression<'ast> {
    Identifier(TokenElement<'ast, String, PreprocessorError<'ast>>),
    Bracketed(Box<PreprocExpression<'ast>>),
    And(
        Box<PreprocExpression<'ast>>,
        Box<PreprocExpression<'ast>>,
    ),
    Or(
        Box<PreprocExpression<'ast>>,
        Box<PreprocExpression<'ast>>,
    ),
    Not(Box<PreprocExpression<'ast>>),
}
impl<'ast> PreprocessorExpression<'ast> {

}
/* 
impl std::fmt::Display for PreprocessorExpression<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreprocessorExpression::Identifier(id) => write!(f, "{}", self.identifier_string()),
            PreprocessorExpression::Bracketed(expr) => write!(f, "({})", expr),
            PreprocessorExpression::And(left, right) => write!(f, "{} && {}", left, right),
            PreprocessorExpression::Or(left, right) => write!(f, "{} || {}", left, right),
            PreprocessorExpression::Not(expr) => write!(f, "!{}", expr),
        }
    }
}*/


pub enum AltPreprocExpr<'ast> {
    And(
        PreprocElement<'ast, PreprocAnd<'ast>>
    )
}

pub struct PreprocAnd<'ast> {
    pub left: Box<PreprocElement<'ast, AltPreprocExpr<'ast>>>,
    pub right: Box<PreprocElement<'ast, AltPreprocExpr<'ast>>>,
}
    */