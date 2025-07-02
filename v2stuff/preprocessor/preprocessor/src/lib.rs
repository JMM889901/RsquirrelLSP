use std::{fmt::Display, path::PathBuf, sync::{Arc, RwLock}};

use chumsky::{input::Input, span::SimpleSpan};
use compiler_state::{term::{CompareResult, KnownTerm}, CompilerState};
use mod_loader::fileinfo::{FileType, LoadedFile};
use parse_common::parsed_element::Parsed;
use preprocessor_parser::{parse_expression, PreprocessedAST, PreprocessorAST};
use sqparse::{Flavor, TokenItem, TokenIter};

use crate::predictor::evaluate_state;

//Estimate states for a given AST
pub mod predictor;
//Create variants from a given AST
pub mod analyser;

#[derive(Debug, Default)]
pub struct Preprocessor{
    //Guess who forgot about multithreading when writing all the code and now has to use a RwLock instead of refactoring because im lazy
    //Its actually very easy to guess who did this
    pub term_map: RwLock<Vec<Arc<KnownTerm>>>,
}

impl Preprocessor {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn preprocess_file(
        &self,
        file: LoadedFile,
    ) -> Result<PreprocessedScript, String> {
        if file.file_type == FileType::Script {
            // I truly, genuinely, deeply hate this shit, when i feel like it i am tearing it all out
            // This is unbelievably scuffed and entirely because i *hate* lifetimes
            let tokens: TokenStore = TokenStore::from_file(&file);

            //Yes this is just fucking with the borrow checker
            //I do not care any more (assuming it works, which it probably wont :)  )
            //TODO: Un-lifetime everything if possible, the ast SHOULD own the input, but chumsky really *really* hates that
            //I would rather not have to clone everything so
            let ast: Vec<Parsed<'_, PreprocessorAST<'_>>> = preprocessor_parser::preprocess(&tokens.tokens)
                .map_err(|e| format!("Failed to preprocess file with name\"{}\": {}", file.file_info.name(), e))?;
            let ast: Vec<Parsed<'_, PreprocessorAST<'_>>> = unsafe {
                std::mem::transmute(ast)
            };
            
            let states = self.identify_valid_states(file.file_info.run_on(), &ast)
                .map_err(|e| format!("Failed for find states for {}: {}", file.file_info.path().to_string_lossy(), e))?;

            let variants = self.split_ast(&ast, states, &file);

            let preproc = PreprocessedScript {
                file,
                tokens,
                ast,
                variants
            };
            Ok(preproc)
        } else {
            let preproc = PreprocessedScript {
                file,
                tokens: TokenStore {
                    tokens: Vec::new(),
                    _text: Arc::new(String::new()),
                },
                ast: Vec::new(),
                variants: Vec::new(),
            };//Temporary, I'm still unsure if i should really handle this here or later, i'm leaning towards later
            //Really, its much easier to treat externals as regular files initially, but its honestly really annoying later so i should change that
            Ok(preproc)
        }
    }
    pub fn build_specific_variant(
        &self,
        file: &PreprocessedScript,
        state: &str,
    ) -> Result<Vec<Block>, String> {
        //TODO: This is an innefficient way to do this, but since its a user facing thing its not going to actually trigger that often
        let expr_tokens = sqparse::tokenize(state, sqparse::Flavor::SquirrelRespawn).into_tokens();
        let expression = parse_expression(&expr_tokens)?;
        let mut blocks = Vec::new();
        for variant in file.variants.iter() {
            if evaluate_state(&variant.id.state, &expression.item) == CompareResult::Accept {
                for block in variant.blocks.iter() {
                    if !blocks.contains(block) {
                        blocks.push(block.clone());
                    }
                }
            }
        }
        if blocks.is_empty() {
            return Err(format!("No variant found for state: {}", state));
        }
        Ok(blocks)
    }

}

///If i understand rust correctly ( I do not )
/// This shouldn't cause a leak since the static is held within another object (the struct) and should drop along with the rest of it
/// TODO: When i get round to benchmarking, profile this
#[derive(Debug)]
pub struct TokenStore<'a> {
    tokens: Vec<TokenItem<'a>>,
    _text: Arc<String>,//Ensures text text wont be dropped while tokens borrows it still
}
impl<'a> TokenStore<'a> {
    pub fn from_file(file: &LoadedFile) -> Self {
        let text = file.file_info.text();
        let tokens: Vec<TokenItem<'_>> = sqparse::tokenize(text.as_ref(), sqparse::Flavor::SquirrelRespawn).into_tokens();
        let tokens = unsafe {
            std::mem::transmute::<Vec<TokenItem<'_>>, Vec<TokenItem<'a>>>(tokens)
        };
        TokenStore {
            tokens,
            _text: Arc::clone(&text),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct VariantID {
    pub state: CompilerState,
    pub file: LoadedFile,//This should be replaced later, its an arc and i dont actually *need* most of that information later
    //I should just use something with copy, and i only really need a path (load position could also work)
    //^ Technically i use the text as well, but only really so i can print for debugging
}
impl Display for VariantID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.file.file_info.name(), self.state)
    }
}

//Test functions
impl VariantID {
    pub fn test_variant(file: PathBuf, state: CompilerState) -> Self {
        VariantID {
            state,
            file: LoadedFile::create_dummy(0, file),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block{
    pub items: SimpleSpan,
    pub range: std::ops::Range<usize>, //This is the range of the block in the original text
}
#[derive(Debug)]
pub struct PreprocessedScript {
    pub file: LoadedFile,
    //I'm not convinced this is safe to pub, its scary (Ok seriously copilot, why did you try correct that to scawwy, lets have a talk)
    tokens: TokenStore<'static>,//Here because lifetimes suck and i hate interacting with them, so fuck it
    pub ast: PreprocessedAST<'static>,
    //pub tokens: Rc<Vec<TokenItem<'a>>>,
    //pub ast: PreprocessedAST<'a>,
    pub variants: Vec<PreprocessedVariant>,
}
impl PreprocessedScript {
    pub fn text(&self) -> Arc<String> {
        self.tokens._text.clone()
    }
    pub fn path(&self) -> &PathBuf {
        self.file.file_info.path()
    }
    pub fn tokens(&self) -> &[TokenItem] {
        &self.tokens.tokens
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ASTTokenIter<'a> {
    pub tokens: &'a [TokenItem<'a>],
    pub blocks: &'a [Block],
    pub index: (usize, usize), // (block_index, item_index)
}
impl<'a> ASTTokenIter<'a> {
    pub fn from(varaint: &'a PreprocessedVariant, file: &'a PreprocessedScript) -> Self {
        ASTTokenIter { tokens: file.tokens.tokens.as_slice(), 
            blocks: &varaint.blocks, 
            index: (
                0, 
                varaint.blocks.first()
                    .map_or(0, |x| x.items.start)
            ) 
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreprocessedVariant {
    pub id: VariantID,
    pub blocks: Vec<Block>,
}
impl PreprocessedVariant {
    pub fn to_text(&self) -> String {
        //TODO: This is problematic, i have somewhat disconected this from the current file state
        //So its now *technically* possible for this to generate from an older file version, and then print using the new versions text
        let text = self.id.file.file_info.text();
        self.blocks.iter()
            .map(|block: &Block| {
                block.range.clone()
            })
            .map(|range| {
                text.get(range).unwrap_or("").to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn to_text_safe(&self, text: Arc<String>) -> String {
        self.blocks.iter()
            .map(|block: &Block| {
                block.range.clone()
            })
            .map(|range| {
                text.get(range).unwrap_or("").to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl<'a> Input<'a> for &'a ASTTokenIter<'a> {
    type Span = SimpleSpan;

    type Token = TokenItem<'a>;

    type MaybeToken = TokenItem<'a>;

    type Cursor = (usize, usize);//Block, Position

    type Cache = Self;

    fn begin(self) -> (Self::Cursor, Self::Cache) {
        let cursor = (0, 0);
        let cache = self;
        (cursor, cache)
    }

    fn cursor_location(cursor: &Self::Cursor) -> usize {
        cursor.1
    }

    unsafe fn next_maybe(
        cache: &mut Self::Cache,
        cursor: &mut Self::Cursor,
    ) -> Option<Self::MaybeToken> {
        if cursor.0 > cache.blocks.len() {
            return None;
        }
        let block = &cache.blocks[cursor.0];
        if cursor.1 >= block.items.end {
            cursor.0 += 1;
            if cursor.0 >= cache.blocks.len() {
                return None;
            }
            cursor.1 = cache.blocks.get(cursor.0)
                .map_or(0, |b| b.items.start);
        }
        let item = &cache.tokens[cursor.1];
        cursor.1 += 1;
        Some(item.clone())
    }

    unsafe fn span(_: &mut Self::Cache, range: std::ops::Range<&Self::Cursor>) -> Self::Span {
        SimpleSpan::from(range.start.1..range.end.1)//TODO: This is very wrong, but not super easy to get right
        //I might have to make Self::Span a span of span which like, ew
    }
}
/// Let this be proof that i WANTED to use sqparse
/// But alas sqparse delegates locating the end of a scope to the lexer, 
/// Which does not account for preprocessing stuff
/// So i can't actually trust sqparse for these cases
/// In short
/// 
/// Boo (And i cannot stress this enough) womp
impl<'a> TokenIter<'a> for ASTTokenIter<'a> {
    fn flavor(self) -> Flavor {
        Flavor::SquirrelRespawn
    }

    fn previous(self) -> Option<&'a TokenItem<'a>> {
        let idx = self.previous_index()?;
        self.tokens.get(idx)
        
    }

    fn next(self) -> Option<&'a TokenItem<'a>> {
        self.tokens.get(self.index.1)
    }

    fn is_ended(self) -> bool {
        self.index.1 >= self.tokens.len() || self.index.0 >= self.blocks.len()
    }

    fn start_index(self) -> usize {
        self.index.1
    }

    fn is_newline(self) -> bool {
        self.previous()
            .map(|item| item.token.new_line.is_some())
            .unwrap_or(false)
    }

    fn split_first(self) -> Option<(Self, &'a TokenItem<'a>)> {
        if let Some(item) = self.tokens.get(self.index.1) {
            let next_index = self.next_index(self.index)?;
            let next_iter = ASTTokenIter {
                tokens: self.tokens,
                blocks: self.blocks,
                index: next_index,
            };
            Some((next_iter, item))
        } else {
            None
        }
    }


    fn split_at(self, index: usize) -> (Self, Self) {
        let block = &self.blocks.binary_search_by(|other|{
            let cmp =  other.items.start.cmp(&index);
            match cmp{
                std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {
                    if other.items.end < index {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Equal
                    }
                },
                _ => std::cmp::Ordering::Greater,
            }
        } ).unwrap_or_else(|x| {eprint!("Attempting to split at a position out of bounds"); x});

        

        (
            ASTTokenIter {
                index: self.index,
                tokens: &self.tokens[..index],//Sketchy
                blocks: self.blocks,//Sketchy
            },
            ASTTokenIter {
                index: (*block, index),
                tokens: self.tokens,
                blocks: self.blocks,
            },
        )
        
    }

    fn previous_index(&self) -> Option<usize> {
        let index = self.index;
        let block = &self.blocks.get(index.0)?;
        if index.1 > block.items.start {
            Some(index.1 - 1)
        } else if index.1 == block.items.start {
            if index.0 > 0 {
                let prev_block = &self.blocks[index.0 - 1];
                Some(prev_block.items.end)
            } else {
                None
            } 
        } else if index.1 > 0 {
            eprintln!("Index {:?} has become disconnected from block {:?}", index, self.blocks[index.0].items);
            Some(index.1-1)
        } else {
            None
        }
    }
}
impl ASTTokenIter<'_> {
    pub fn next_index(&self, index: (usize, usize)) -> Option<(usize, usize)> {
        let block = &self.blocks.get(index.0)?;
        if index.1 < block.items.end {
            Some((index.0, index.1 + 1))
        } else if index.0 + 1 < self.blocks.len() {
            Some((index.0 + 1, self.blocks[index.0 + 1].items.start))
        } else {
            None
        }
    }

}