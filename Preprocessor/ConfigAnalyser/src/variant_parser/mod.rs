use crate::sq_file_variant::SqFileVariant;

pub mod variant;
pub mod traversal;
pub mod parse;

peg::parser!{
    pub grammar variant_inout() for variant::SqVariant{
        pub rule out_the_in() -> String =
            a:(a:[_]{a})* {a.into_iter().collect()}
    }
}

peg::parser!{
    pub grammar variant_inout_standard() for SqFileVariant{
        pub rule out_the_in() -> String =
            a:(a:[_]{a})* {a.into_iter().collect()}
    }
}