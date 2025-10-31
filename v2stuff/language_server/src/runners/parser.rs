use std::sync::Arc;

use preprocessor::ASTTokenIndexIter;
use runner::{analysis_error::AnalysisError, results::AnalysisResult, step::AnalysisStep};
use sqparse::{ast::Program, parse_iter};


pub struct ParserStep{}

impl AnalysisStep<ParseResult> for ParserStep {
    fn analyse(&self, context: &runner::AnalysisContext) -> Result<ParseResult, AnalysisError> {
        let script = &context.analyser.files[context.variant_idx];
        let variant = &script.variants[context.variant_idx];
        let tokeniter = ASTTokenIndexIter::from_variant(variant, script);
        match tokeniter {
            Some(tokeniter) => {
                let parse = parse_iter(tokeniter);
                match parse {
                    Ok(program) => {
                        Ok(ParseResult {
                            program: Arc::new(program),
                        })
                    },
                    Err(err) => {
                        panic!("TODO: handle parse error properly: {:?}", err);
                        //TODO: return proper error
                    }
                }
            }
            None => {
                panic!("TODO: handle token iterator creation error properly");
            }
        }
           
    }
}

impl AnalysisResult for ParseResult {}
pub struct ParseResult {
    pub program: Arc<Program<'static>>,
}