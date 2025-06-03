use ConfigAnalyser::SqFileVariant;

use crate::ast::{Element, AST};

mod declaration_tests;
mod assignment_tests;
mod struct_tests;
mod filescope_tests;
mod for_tests;
mod if_tests;
mod integration_tests;
mod switch_tests;
mod expression_tests;
pub fn generate_data_structure(text: String) -> SqFileVariant{
    return SqFileVariant::stateless(text)
}

//pub struct DebugASTBuilder(Element<AST>);
//impl DebugASTBuilder{
//
//}
//impl PartialEq<Element<AST>> for DebugASTBuilder{
//    //We dont care about ranges really
//    fn eq(&self, other: &Element<AST>) -> bool {
//        
//    }
//}
//impl PartialEq<AST> for DebugASTBuilder{
//    fn eq(&self, other: &AST) -> bool {
//
//    }
//}


macro_rules! test_ast_eq {
    ($($name:ident: $value:expr,)*) => {
    $(
        #[test]
        fn $name() {
            let (input, expected) = $value;
            let input = generate_data_structure(input);
            let expected = generate_data_structure(expected);
            let parse = &SquirrelParse::empty();
            let result = squirrel_ast::file_scope_dbg(&input, &RefCell::new(0), parse);
            assert_eq!(expected, fib(result.unwrap()));
        }
    )*
    }
}

macro_rules! test_ast_matches {
    ($($name:ident: $value:expr,)*) => {
    $(
        #[test]
        fn $name() {
            let (input, expected) = $value;
            let input = generate_data_structure(input);
            let expected = generate_data_structure(expected);
            let parse = &SquirrelParse::empty();
            let result = squirrel_ast::file_scope_dbg(&input, &RefCell::new(0), parse);
            let result = result.unwrap();
            assert!(matches!(result, expected), "Expected {:?} to match {:?}", result, expected);
        }
    )*
    }
}