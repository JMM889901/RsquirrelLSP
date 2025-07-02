use compiler_state::{term::CompareResult, CompilerState};
use mod_loader::fileinfo::LoadedFile;
use parse_common::parsed_element::Parsed;
use preprocessor_parser::{PreprocessedAST, PreprocessorAST};

use crate::{predictor::evaluate_state, Block, PreprocessedVariant, Preprocessor, VariantID};

impl Preprocessor{
    pub fn split_ast(&self, ast: &PreprocessedAST<'_>, states: Vec<CompilerState>, id: &LoadedFile) -> Vec<PreprocessedVariant> {
        let mut variants = Vec::new();
        for state in states {
            let variant = self.apply_state(ast, &state, id);
            variants.push(variant);
        }
        variants
    }

    pub fn apply_state(&self, ast: &PreprocessedAST<'_>, state: &CompilerState, id: &LoadedFile) -> PreprocessedVariant {
        let mut blocks = Vec::new();
        for ast in ast{
            blocks.extend(get_blocks(ast, state));
        }
        
        PreprocessedVariant {
            id: VariantID {
                state: state.clone(),
                file: id.clone(),
            },
            blocks,
        }
    }
}


    pub fn get_blocks<'a>(ast: &Parsed<'a, PreprocessorAST<'a>>, state: &CompilerState) -> Vec<Block> {
        let mut blocks = Vec::new();
        match &ast.item {
            PreprocessorAST::Block(block) => {
                blocks.push(Block {
                    items: *block,
                    range: ast.range.into_range(),
                });
            }
            PreprocessorAST::If { body, ends, condition } => {
                if evaluate_state(state, &condition.item) != CompareResult::Reject {// If it works its better to allow dont care
                    //In parsing, we should never encounter dont care, however this lets me do things like user facing options to view the file with different terms
                    for item in body {
                        blocks.extend(get_blocks(item, state));
                    }
                } else {
                    blocks.extend(get_blocks(ends, state));
                }
            }
            PreprocessorAST::ElseIf { body, ends, condition } => {
                if evaluate_state(state, &condition.item) != CompareResult::Reject {
                    for item in body {
                        blocks.extend(get_blocks(item, state));
                    }
                } else {
                    blocks.extend(get_blocks(ends, state));
                }
            }
            PreprocessorAST::Else { body, .. } => {
                for item in body {
                    blocks.extend(get_blocks(item, state));
                }
                //blocks.extend(self.get_blocks(&ends.item, state));
            }
            PreprocessorAST::EndIf => {
            }
            PreprocessorAST::Error => {
            }
        }
        blocks
    }