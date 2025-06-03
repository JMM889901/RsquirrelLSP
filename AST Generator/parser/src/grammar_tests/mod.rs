use ConfigAnalyser::sq_file_variant::SqFileVariant;

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