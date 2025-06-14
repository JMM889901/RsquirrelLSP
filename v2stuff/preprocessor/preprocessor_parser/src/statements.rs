use parse_common::parsed_element::{ParseVisitable, Parsed};

#[derive(Debug, Clone, PartialEq)]
pub enum PreprocessorExpression<'src> {
    Identifier(String),
    Placeholder, //Syntax error, missing identifier
    Bracketed(Box<Parsed<'src, PreprocessorExpression<'src>>>),
    And(
        Box<Parsed<'src, PreprocessorExpression<'src>>>,
        Box<Parsed<'src, PreprocessorExpression<'src>>>,
    ),
    Or(
        Box<Parsed<'src, PreprocessorExpression<'src>>>,
        Box<Parsed<'src, PreprocessorExpression<'src>>>,
    ),
    Not(Box<Parsed<'src, PreprocessorExpression<'src>>>),
}
impl<'src> ParseVisitable<'src> for PreprocessorExpression<'src> {
    fn visit_parse_children(
        &'src self,
        function: &mut impl FnMut(&'src Parsed<'src, Self>),
        pre_visit: &mut impl FnMut(&'src Parsed<'src, Self>) -> bool,
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
