use std::sync::{Arc, RwLock};

use chumsky::{input::Input, span::SimpleSpan};
use compiler_state::{term::KnownTerm, CompilerState};
use mod_loader::fileinfo::{FileType, LoadedFile};
use parse_common::parsed_element::Parsed;
use preprocessor_parser::{PreprocessedAST, PreprocessorAST};
use sqparse::TokenItem;

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
        //
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

#[derive(Debug, Clone, PartialEq)]
pub struct VariantID {
    pub state: CompilerState,
    pub file: LoadedFile,//This should be replaced later, its an arc and i dont actually *need* most of that information later
    //I should just use something with copy, and i only really need a path
    //^ Technically i use the text as well, but only really so i can print for debugging
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block<'a>{
    pub items: Arc<Vec<TokenItem<'a>>>,
    pub range: std::ops::Range<usize>, //This is the range of the block in the original text
}
#[derive(Debug)]
pub struct PreprocessedScript {
    pub file: LoadedFile,
    //I'm not convinced this is safe to pub, its scary (Ok seriously copilot, why did you try correct that to scawwy, lets have a talk)
    pub tokens: TokenStore<'static>,//Here because lifetimes suck and i hate interacting with them, so fuck it
    pub ast: PreprocessedAST<'static>,
    //pub tokens: Rc<Vec<TokenItem<'a>>>,
    //pub ast: PreprocessedAST<'a>,
    pub variants: Vec<PreprocessedVariant<'static>>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct PreprocessedVariant<'a> {
    pub id: VariantID,
    pub blocks: Vec<Block<'a>>,
}
impl PreprocessedVariant<'_> {
    pub fn to_text(&self) -> String {

        //TODO: This is problematic, i have somewhat disconected this from the current file state
        //So its now *technically* possible for this to generate from an older file version, and then print using the new versions text
        let text = self.id.file.file_info.text();
        self.blocks.iter()
            .map(|block: &Block<'_>| {
                block.range.clone()
            })
            .map(|range| {
                text.get(range).unwrap_or("").to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl<'a> Input<'a> for &'a PreprocessedVariant<'a> {
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
        if cursor.1 >= block.items.len() {
            cursor.0 += 1;
            cursor.1 = 0;
            if cursor.0 >= cache.blocks.len() {
                return None;
            }
        }
        let item = &block.items[cursor.1];
        cursor.1 += 1;
        Some(item.clone())
    }

    unsafe fn span(_: &mut Self::Cache, range: std::ops::Range<&Self::Cursor>) -> Self::Span {
        SimpleSpan::from(range.start.1..range.end.1)//TODO: This is wrong i think
    }
}