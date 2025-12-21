use std::{collections::btree_map::Range, fmt::Display, path::PathBuf, sync::{Arc, RwLock}};

use chumsky::{input::Input, span::SimpleSpan};
use compiler_state::{term::{CompareResult, KnownTerm}, CompilerState};
use mod_loader::fileinfo::{FileType, LoadedFile};
use parse_common::parsed_element::Parsed;
use preprocessor_parser::{parse_expression, PreprocessedAST, PreprocessorAST};
use sqparse::{token::TerminalToken, Flavor, TokenItem, TokenIter};

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
            //^Slightly future me here to point out that this is entirely my own fault, errors are kinda bound to the lifetime of the input since they contain borrows to it
            //I kinda just, don't care any more? 
            //This is *extremely* unsafe and bad, tough shit.
            let ast: Vec<Parsed<'_, PreprocessorAST<'_>>> = preprocessor_parser::preprocess(&tokens.tokens)
                .map_err(|e| format!("Failed to preprocess file with name\"{}\": {}", file.file_info.name(), e))?;
            let ast: Vec<Parsed<'_, PreprocessorAST<'_>>> = unsafe {
                std::mem::transmute(ast)
            };
            
            let states = self.identify_valid_states(file.file_info.run_on(), &ast)
                .map_err(|e| format!("Failed for find states for {}: {}", file.file_info.path().to_string_lossy(), e))?;

            let variants = self.split_ast(&ast, states, &file, tokens.tokens.len());

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
    pub variantidx: usize,
    //I should just use something with copy, and i only really need a path (load position could also work)
    //^ Technically i use the text as well, but only really so i can print for debugging
    //pub fileidx: usize,//So, you know how they say preemptive optimization is the root of all evil?
    //That, literally fucking that. I am literally the cause of all my problems
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
            variantidx: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block{
    ///IMPORTANT: Items.end is the trailing index, so it will likely be a preprocessor token
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
    pub block: &'a Block,
    pub index: (usize, usize), // (block_index, item_index)
}
impl<'a> ASTTokenIter<'a> {
    pub fn from(varaint: &'a PreprocessedVariant, file: &'a PreprocessedScript) -> Option<Self> {
        Some(ASTTokenIter { tokens: file.tokens.tokens.as_slice(), 
            blocks: &varaint.blocks, 
            index: (
                0, 
                varaint.blocks.first()
                    .map_or(0, |x| x.items.start)
            ),
            block: varaint.blocks.last()?,
        })
    }
}


#[derive(Debug, Clone, Copy)]
pub struct ASTTokenIndexIter<'a> {
    pub tokens: &'a [TokenItem<'static>],
    pub mapped_positions: &'a [usize],
    pub index: usize,
}
impl<'a> ASTTokenIndexIter<'a> {
    pub fn from_variant(varaint: &'a PreprocessedVariant, file: &'a PreprocessedScript) -> Option<Self> {
        Some(ASTTokenIndexIter { tokens: file.tokens.tokens.as_slice(), 
            mapped_positions: &varaint.index_map,
            index: 0,
        })
    }
    pub fn from(block_map: &'a [usize], file: &'a PreprocessedScript) -> Option<Self> {
        Some(ASTTokenIndexIter { tokens: file.tokens.tokens.as_slice(), 
            mapped_positions: block_map,
            index: 0,
        })
    }
}


#[derive(Debug, Clone, PartialEq)]
pub struct PreprocessedVariant {
    pub id: VariantID,
    //TODO: Pick, either blocks of tokens, or a map of token indexes
    pub blocks: Vec<Block>,
    pub index_map: Vec<usize>,
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
/// This is a stupidly bad way to do this
/// Since like, this just drops shit constantly
/// it probably should just self mutate
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

    fn split_first(mut self) -> Option<(Self, &'a TokenItem<'a>)> {
        if let Some(item) = self.next() {

            let next_index = self.next_index(self.index);
            match next_index {
                Some(index) => {
                    self.block = &self.blocks[index.0];
                    self.index = index;
                },
                None => {
                    self.block = &self.blocks[self.blocks.len() - 1];
                    self.index = (self.blocks.len(), self.tokens.len() + 1);
                }
            }
            Some((self, item))
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
                tokens: self.tokens,//Sketchy
                blocks: self.blocks,//Sketchy
                block: self.block,
            },
            ASTTokenIter {
                index: (*block, index),
                tokens: self.tokens,
                blocks: self.blocks,
                block: &self.blocks[*block],

            },
        )
        
    }


}
impl ASTTokenIter<'_> {
    pub fn next_index(&self, index: (usize, usize)) -> Option<(usize, usize)> {
        if index.1 < self.block.items.end {
            return Some((index.0, index.1 + 1))
        }
        Some((index.0 + 1, self.blocks.get(index.0 + 1)?.items.start))

    }

    fn previous_index(&self) -> Option<usize> {
        let block = self.blocks.get(self.index.0)?;
        if self.index.1 > block.items.start {
            Some(self.index.1 - 1)
        } else if self.index.1 == block.items.start {
            if self.index.0 > 0 {
                let prev_block = &self.blocks[self.index.0 - 1];
                Some(prev_block.items.end)
            } else {
                None
            }
        } else if self.index.1 > 0 {
            eprintln!("Index {:?} has become disconnected from block {:?}", self.index, self.blocks[self.index.0].items);
            Some(self.index.1 - 1)
        } else {
            None
        }
    }

}

impl<'a> TokenIter<'a> for ASTTokenIndexIter<'a> {
    fn flavor(self) -> Flavor {
        Flavor::SquirrelRespawn
    }

    fn previous(self) -> Option<&'a TokenItem<'a>> {
        if self.index == 0 {
            return None;
        }
        self.tokens.get(self.mapped_positions[self.index - 1])
    }

    fn next(self) -> Option<&'a TokenItem<'a>> {
        self.tokens.get(self.mapped_positions[self.index])
    }

    fn is_ended(self) -> bool {
        self.index + 1 >= self.mapped_positions.len()
    }

    fn start_index(self) -> usize {
        self.mapped_positions[self.index]
    }

    fn is_newline(self) -> bool {
        self.previous()
            .map(|item| item.token.new_line.is_some())
            .unwrap_or(false)
    }

    fn split_first(mut self) -> Option<(Self, &'a TokenItem<'a>)> {
        if let Some(item) = self.next() {
            self.index += 1;
            Some((self, item))
        } else {
            None
        }
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let second = ASTTokenIndexIter {
            tokens: self.tokens,
            mapped_positions: self.mapped_positions,
            index
        };
        (self, second)
    }
}