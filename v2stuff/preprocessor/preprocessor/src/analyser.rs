use compiler_state::{term::CompareResult, CompilerState};
use mod_loader::fileinfo::LoadedFile;
use parse_common::parsed_element::Parsed;
use preprocessor_parser::{PreprocessedAST, PreprocessorAST};

use crate::{predictor::evaluate_state, Block, PreprocessedVariant, Preprocessor, VariantID};

impl Preprocessor{
    pub fn split_ast<'a>(&self, ast: &PreprocessedAST<'a>, states: Vec<CompilerState>, id: &LoadedFile) -> Vec<PreprocessedVariant<'a>> {
        let mut variants = Vec::new();
        for state in states {
            let variant = self.apply_state(ast, &state, id);
            variants.push(variant);
        }
        variants
    }

    pub fn apply_state<'a>(&self, ast: &PreprocessedAST<'a>, state: &CompilerState, id: &LoadedFile) -> PreprocessedVariant<'a> {
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


    pub fn get_blocks<'a>(ast: &Parsed<'a, PreprocessorAST<'a>>, state: &CompilerState) -> Vec<Block<'a>> {
        let mut blocks = Vec::new();
        match &ast.item {
            PreprocessorAST::Block(block) => {
                blocks.push(Block {
                    items: block.clone(),
                    range: ast.range.into_range(),
                });
            }
            PreprocessorAST::If { body, ends, condition } => {
                if evaluate_state(state, &condition.item) == CompareResult::Accept {
                    for item in body {
                        blocks.extend(get_blocks(item, state));
                    }
                } else {
                    blocks.extend(get_blocks(ends, state));
                }
            }
            PreprocessorAST::ElseIf { body, ends, condition } => {
                if evaluate_state(state, &condition.item) == CompareResult::Accept {
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