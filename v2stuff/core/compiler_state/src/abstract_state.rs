//TODO: I think this is the better model, but I'm not sure if i can use QM with this
//^I can't, QM is truth tables, previously i hacked it by allowing cancel operations on dont cares, but this isnt a truth table
/*
    Essentially, states currently are a list of AND terms, without support for or, 
    In essence, this means a given state has one of 3 states, Required, Refused, or dont care
    As a result, an or expression must instead of split into every possible combination in which it is accepted
    the goal here would be for state not to actually represent the state, just the vague requirements of the state
    Essentially, by comparing an expression to an expression, rather than a state to a state 

    This approach does lose me a few optimizations, the sorted vec state lets be just binary everything, but realistically how often does make a real difference?
    iirc vanillas most preprocessor-heavy file only has like 10 terms
*/

/*
    If i can get this to work, it opens the door to multistate/one parse instead of the preprocessor approach i use now, and if I can do that effectively, 
    basically any cost from not having QM is meaningless

    Unfortunately, I am VERY stupid
 */

#[derive(Clone)]
pub enum ExprState<'a> {
    Not(Box<ExprState<'a>>),
    And(Box<(ExprState<'a>, ExprState<'a>)>),
    Or(Box<(ExprState<'a>, ExprState<'a>)>),
    Term(&'a String), // Borrowing here is intentional, see ArcState comparisons and put 1 and 1 together
}
impl<'a> ExprState<'a> {
    
}

pub enum AbstractState<'a> {
    Must(&'a String), // Must be true
    MustNot(&'a String), // Must be false
    
}