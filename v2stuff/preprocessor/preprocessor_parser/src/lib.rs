use chumsky::{prelude::{any, choice, recursive}, Boxed, Parser};
use sqparse::{token::{TerminalToken, TokenType}, TokenItem};

pub enum PreprocessorAST<'ast>{
    If {
        condition: Vec<TokenItem<'ast>>,
        body: Vec<PreprocessorAST<'ast>>,
    },
    ElseIf {
        condition: Vec<TokenItem<'ast>>,
        body: Vec<PreprocessorAST<'ast>>,
    },
    Else {
        body: Vec<PreprocessorAST<'ast>>,
    },
    EndIf,
    Block(Vec<TokenItem<'ast>>),
}
#[derive(Debug, Clone, PartialEq)]
pub enum PreprocessorExpression<'ast>{
    Identifier(TokenItem<'ast>),
    Bracketed(Box<PreprocessorExpression<'ast>>),
    And(Box<PreprocessorExpression<'ast>>, Box<PreprocessorExpression<'ast>>),
    Or(Box<PreprocessorExpression<'ast>>, Box<PreprocessorExpression<'ast>>),
    Not(Box<PreprocessorExpression<'ast>>),
}
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
}
impl PreprocessorExpression<'_>{
    pub fn identifier_string<'ast>(&self) -> String {
        match self {
            PreprocessorExpression::Identifier(id) => {
                if let TokenType::Identifier(name) = id.token.ty {
                    name.to_string()
                } else {
                    panic!("Expected identifier token, found: {:?}", id.token.ty);
                }
            },
            _ => panic!("This is not an identifier expression"),
        }
    }
}

fn identifier<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], TokenItem<'src>> {
    any().filter(|item: &TokenItem<'src>| {
        matches!(item.token.ty, TokenType::Identifier(_))
    })
}
fn terminal<'src>(token: TerminalToken) ->  Boxed<'src, 'src, &'src [TokenItem<'src>], TokenItem<'src>> {
    any().filter(move |item: &TokenItem| {
        matches!(item.token.ty, TokenType::Terminal(t) if t == token)
    }).boxed()
}
fn logical_op<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], TokenItem<'src>> {
    any().filter(|item: &TokenItem<'src>| {
        matches!(item.token.ty, TokenType::Terminal(TerminalToken::LogicalAnd) | 
                  TokenType::Terminal(TerminalToken::LogicalOr) | TokenType::Terminal(TerminalToken::Not))
    })
}

fn preprocessor_expression<'src>() -> impl Parser<'src, &'src [TokenItem<'src>], PreprocessorExpression<'src>> {
    let identifier_parser = identifier().map(|item| PreprocessorExpression::Identifier(item)).boxed();
    let logical_op_parser = logical_op().boxed();
    //let bracketed = preprocessor_expression().delimited_by(
    //    terminal(TerminalToken::OpenBracket),
   //     terminal(TerminalToken::CloseBracket),
    //).map(|exprs| PreprocessorExpression::Bracketed(Box::new(exprs)));
    recursive(|expr|
        choice((
            identifier_parser.clone().then(logical_op_parser).then(expr.clone()).map(|((id, op), right)| {
                match op.token.ty {
                    TokenType::Terminal(TerminalToken::LogicalAnd) => PreprocessorExpression::And(Box::new(id), Box::new(right)),
                    TokenType::Terminal(TerminalToken::LogicalOr) => PreprocessorExpression::Or(Box::new(id), Box::new(right)),
                    TokenType::Terminal(TerminalToken::Not) => PreprocessorExpression::Not(Box::new(right)),
                    _ => unreachable!(),
                }
            }),
            identifier_parser,
            expr.delimited_by(
                terminal(TerminalToken::OpenBracket),
                terminal(TerminalToken::CloseBracket),
            ).map(|exprs| PreprocessorExpression::Bracketed(Box::new(exprs))),
        ))
    )
}

pub fn preprocess_tokens<'ast>(tokens: Vec<TokenItem>) -> Vec<PreprocessorAST<'ast>> {
    
}