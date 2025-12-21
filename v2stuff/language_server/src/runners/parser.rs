use std::sync::Arc;

use preprocessor::ASTTokenIndexIter;
use runner::{analysis_error::AnalysisError, results::AnalysisResult, step::AnalysisStep};
use sqparse::{ast::Program, parse_iter};


pub struct ParserStep{}

impl AnalysisStep<ParseResult> for ParserStep {
    fn analyse(&self, context: &runner::AnalysisContext) -> Result<ParseResult, AnalysisError> {
        let script = &context.analyser.files[context.variant.file_idx];//thats.. awful
        let variant = &script.variants[context.variant.variant_idx];
        let tokeniter = ASTTokenIndexIter::from_variant(variant, script);
        match tokeniter {
            Some(tokeniter) => {
                let parse = parse_iter(tokeniter);
                match parse {
                    Ok(program) => {
                        //I'm going to be sick
                        let program: Program<'static> = unsafe {
                            std::mem::transmute(program)
                        };//All this shit has FUCKING lifetimes
                        //In some esoteric series of chaining ownership, this is *probably* guaranteed to last longer than the source AST
                        //But like, the lifetime chain is absolutely horrific to manage
                        //Its like, text -> tokens<'a> -> ast1<'b> -> tokeniter<'c> -> program<'d> 
                        //Since tokens are guaranteed to outlive program, its sorta kinda ok
                        //I should just use fucking arcs good god

                        //The urge to tear this all down AGAIN and rewrite this shit from scratch is ever growing
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