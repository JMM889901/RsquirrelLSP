use std::{env};
///TODO SIKE THIS IS JUST E2Es AGAIN, I CAN'T KEEP GETTING AWAY WITH THIS
fn main() {
    let _args: Vec<String> = env::args().collect();
    todo!()
}

#[cfg(test)]
mod tests{
    use preprocessor::{ASTTokenIndexIter, ASTTokenIter};
    use rayon::prelude::*;
    use sqparse::{parse, parse_iter};
    use std::{collections::HashMap, path::PathBuf};

    use mod_loader::{fileinfo::{FileInfo, FileType, LoadedFile}, mods::Mod, LoadConfig, ModLoader};


#[test]
fn test_scuffed_paths() {
    //Because northstar has a single file which it loads with a leading / for no reason >:(
    let path = PathBuf::from("../../../../northstar/Northstar.CustomServers/mod/scripts/vscripts/");
    let good_join = "ai/_ai_turret_sentry.gnut";
    let join = "/ai/_ai_turret_sentry.gnut";
    let joined = path.join(PathBuf::from(join));
    assert!(!joined.exists(), "Joined path should not currently exist: {:?}", joined);
    let joined = path.join(good_join);
    assert!(joined.exists(), "Joined path should exist: {:?}", joined);


    let fixed = join.strip_prefix("/").unwrap_or(join);
    let fixed_joined = path.join(fixed);
    assert!(fixed_joined.exists(), "Fixed joined path should exist: {:?}", fixed_joined);
    println!("Fixed joined path: {:?}", fixed_joined);
    let good_fixed = good_join.strip_prefix("/").unwrap_or(good_join);
    assert_eq!(good_fixed, good_join);
}

#[test]
fn test_northstar() {
    let mut loader = ModLoader::new();
    //let path = PathBuf::from("../../../TestSets/ValidStructure/external");
    let path = PathBuf::from("../../../../northstar");
    let count = loader.register_workspace(&path);
    if count.is_err() {//This will likely happen, since this is baseed on my file structure, if you are testing this yourself
        //Cry about it i guess?
        eprintln!("Error registering workspace: {:?}", count.err().unwrap());
        return;
    }
    println!("Registered {} mods", count.unwrap());
    let mods = loader.get_mods();
    for _mod in mods {
        println!("{}", _mod.long_text());
    }
    let config = LoadConfig{
        disabled_names: vec!["Northstar.Client".to_string()],
        ..Default::default()
    };
    let (unfiltered_files, load_errs) = loader.load();
    let filtered_order = loader.initialize_with_config(config);
    let filtered_files = loader.load_from_order(filtered_order.0).0;
    println!("{} unfiltered files", unfiltered_files.len());
    println!("{} filtered files", filtered_files.len());
    let preprocessor = preprocessor::Preprocessor::new();
    let iter = unfiltered_files.par_iter();
    let variants = iter.map(
        |file: &LoadedFile| {
            preprocessor.preprocess_file(file.clone())
        }
    ).collect::<Vec<_>>();
    for variant in variants {
        match variant {
            Ok(preproc) => {
                println!("Preprocessed file: {}", preproc.file.file_info.path().to_string_lossy());
                println!("Run on: {}", preproc.file.file_info.run_on());
                println!("File size: {}", preproc.file.file_info.text().len());
                println!("Variants: {}", preproc.variants.len());
                if preproc.variants.is_empty() && preproc.file.file_type == FileType::Script {
                    panic!("valid states: {:?}", preprocessor.identify_valid_states(preproc.file.file_info.run_on(), preproc.ast.as_ref()).unwrap());
                }
                for v in preproc.variants.clone().iter() {
                    println!("Variant ID: {}", v.id.state);
                    println!("Variant blocks: {}", v.blocks.len());
                    //println!("Variant Text: {}\n\n", v.to_text());

                }
                
            },
            Err(e) => {
                eprintln!("Error preprocessing file: {}", e);
                panic!("Failed to preprocess file: {}", e);
            }
        }
        
    }
    for err in load_errs {
        eprintln!("Error loading file: {:?}", err);
    }
    println!("Found {} terms", preprocessor.term_map.read().unwrap().len());
    for term in preprocessor.term_map.read().unwrap().iter() {
            println!("Term: {}", term);
    }

    

}
#[test]
fn test_vanilla() {
    let mut loader = ModLoader::new();
    //let path = PathBuf::from("../../../TestSets/ValidStructure/external");
    let path = PathBuf::from("../../../../Titanfall2-master");
    let start = std::time::Instant::now();

    let count = loader.register_workspace(&path);
    if count.is_err() {//This will likely happen, since this is baseed on my file structure, if you are testing this yourself
        //Cry about it i guess?
        eprintln!("Error registering workspace: {:?}", count.err().unwrap());
        return;
    }
    println!("Registered {} mods", count.unwrap());
    let mods = loader.get_mods();
    for _mod in mods {
        println!("{}", _mod.long_text());
    }
    let end = std::time::Instant::now();
    let (unfiltered_files, load_errs) = loader.load();
    let iter = unfiltered_files.par_iter();
    let preprocessor = preprocessor::Preprocessor::new();
    let variants = iter.map(
        |file: &LoadedFile| {
            preprocessor.preprocess_file(file.clone())
        }
    ).collect::<Vec<_>>();
    let mut count = 0;
    let analysis_start = std::time::Instant::now();
    for variant in &variants {
        match variant {
            Ok(preproc) => {
                println!("Preprocessed file: {}", preproc.file.file_info.path().to_string_lossy());
                println!("Run on: {}", preproc.file.file_info.run_on());
                println!("File size: {}", preproc.file.file_info.text().len());
                println!("Variants: {}", preproc.variants.len());
                if preproc.variants.is_empty() && preproc.file.file_type == FileType::Script {
                    panic!("valid states: {:?}", preprocessor.identify_valid_states(preproc.file.file_info.run_on(), preproc.ast.as_ref()).unwrap());
                }
                for v in preproc.variants.clone().iter() {
                    count += 1;
                    println!("Variant ID: {}", v.id.state);
                    println!("Variant blocks: {}", v.blocks.len());
                    for block in v.blocks.iter() {
                        if block.items.start < preproc.tokens().len() {
                            let tok = &preproc.tokens()[block.items.start];
                            assert!(!matches!(tok.token.ty, sqparse::token::TokenType::Terminal(sqparse::token::TerminalToken::PreprocessorIf)), "Block starts with preprocessor if: {:?}", tok);
                        }
                    }
                    //println!("Variant Text: {}\n\n", v.to_text());
                    let mut combined = v.blocks.iter().flat_map(|b| b.items.into_range()).collect::<Vec<usize>>();
                    combined.push(preproc.tokens().len());
                    let mut dedup = combined.clone();
                    dedup.dedup();
                    assert!(dedup.len() == combined.len());
                    
                    let tokeniter = ASTTokenIndexIter::from(&combined, preproc);
                    //let tokeniter = ASTTokenIter::from(v, preproc);
                    if tokeniter.is_none() {
                        eprintln!("variant of {} with 0 blocks: {}", preproc.file.file_info.path().display(), v.id.state);
                        continue;
                    }
                    let parse = parse_iter(tokeniter.unwrap());
                    match parse {
                    Ok(program) => eprintln!("Parsed program with {} statements", program.statements.len()),
                    Err(err) => {
                        let mut idx = err.token_index;
                        if idx >= preproc.tokens().len() {
                            idx = preproc.tokens().len() - 1;
                            eprintln!("Error token index {} out of bounds, using last token", err.token_index);
                        }
                        let realpos = preproc.tokens()[idx].token.range.clone();
                        let real = &preproc.text()[realpos.clone()];
                        eprintln!("Error parsing script {}: {:?} at {:?} text: {:?}", preproc.path().display(), err, realpos, real);
                    }
                }
                }
            },
            Err(e) => {
                eprintln!("Error preprocessing file: {}", e);
                panic!("Failed to preprocess file: {}", e);
            }
        }
    }
    let analysis_end = std::time::Instant::now();
    println!("Time taken to analyze mods: {:?}", analysis_end.duration_since(analysis_start));
    println!("Time taken to register mods: {:?}", end.duration_since(start));
    println!("Total variants: {}", count);
    let regular_parse_start = std::time::Instant::now();
    let mut count = 0;
    for v in variants.into_iter().flatten() {
        count += 1;
        let tok = v.tokens();
        let prs = parse(tok, sqparse::Flavor::SquirrelRespawn);
    }
    let regular_parse_end = std::time::Instant::now();
    println!("Without preprocesing, took {:?} to parse {} files", regular_parse_end.duration_since(regular_parse_start), count);
    for err in load_errs {
        eprintln!("Error loading file: {:?}", err);
    }
    println!("Found {} terms", preprocessor.term_map.read().unwrap().len());
    for term in preprocessor.term_map.read().unwrap().iter() {
        println!("Term: {}", term);
    }
}
#[test]
fn test_overriden_vanilla() {
    let vanilla_path = PathBuf::from("../../../../Titanfall2-master");
    let northstar_path = PathBuf::from("../../../../northstar");
    let mut loader = ModLoader::new();
    let load_vanilla_start = std::time::Instant::now();
    let count = loader.register_workspace(&vanilla_path);
    let load_vanilla_end = std::time::Instant::now();
    if count.is_err() {//This will likely happen, since this is baseed on my file structure, if you are testing this yourself
        //Cry about it i guess?
        eprintln!("Error registering workspace: {:?}", count.err().unwrap());
        return;
    }
    let mut count = count.unwrap();
    let load_northstar_start = std::time::Instant::now();
    count += loader.register_workspace(&northstar_path).unwrap();
    let load_northstar_end = std::time::Instant::now();
    println!("Registered {} mods", count);
    let mods = loader.get_mods();
    for _mod in mods {
        println!("{}", _mod.long_text());
    }
    let load_mods_start = std::time::Instant::now();
    let unfiltered_files = loader.load().0;
    let load_mods_end = std::time::Instant::now();

    let _items = unfiltered_files.iter().find(|item| item.file_info.name() == "_items.nut");
    if _items.is_none() {
        for item in unfiltered_files.iter() {
            println!("File: {}", item.file_info.name());
        }
        panic!("Failed to find items.nut in unfiltered files");
    }
    let items = _items.unwrap();
    println!("_items.nut: {:?}", items.file_info.path());
    assert!(items.file_info.path().to_string_lossy().contains("Northstar.CustomServers"));
    println!("Took {:?} to load vanilla scripts", load_vanilla_end.duration_since(load_vanilla_start));
    println!("Took {:?} to load northstar mods", load_northstar_end.duration_since(load_northstar_start));
    println!("Took {:?} to get files", load_mods_end.duration_since(load_mods_start));


    let preprocessor = preprocessor::Preprocessor::new();
    let preproc_start = std::time::Instant::now();
    let preproc = unfiltered_files.par_iter().map(
        |file: &LoadedFile| {
            preprocessor.preprocess_file(file.clone())
        }
    ).collect::<Vec<_>>();
    for preproc in preproc {
        match &preproc {
            Ok(preproc) => {
                for v in preproc.variants.clone().iter() {
                    let mut combined = v.blocks.iter().flat_map(|b| b.items.into_range()).collect::<Vec<usize>>();
                    combined.push(preproc.tokens().len());
                    let mut dedup = combined.clone();
                    dedup.dedup();
                    assert!(dedup.len() == combined.len());
                    
                    let tokeniter = ASTTokenIndexIter::from(&combined, preproc);

                    if tokeniter.is_none() {
                        eprintln!("variant of {} with 0 blocks: {}", preproc.file.file_info.path().display(), v.id.state);
                        continue;
                    }
                    let parse = parse_iter(tokeniter.unwrap());
                    match parse {
                        Ok(program) => (),//eprintln!("Parsed program with {} statements", program.statements.len()),
                        Err(err) => {
                            let mut idx = err.token_index;
                            if idx >= preproc.tokens().len() {
                                idx = preproc.tokens().len() - 1;
                                eprintln!("Error token index {} out of bounds, using last token", err.token_index);
                            }
                            let realpos = preproc.tokens()[idx].token.range.clone();
                            let real = &preproc.text()[realpos.clone()];
                            eprintln!("Error parsing script {}: {:?} at {:?} text: {:?}", preproc.file.file_info.path().display(), err, realpos, real);
                        }
                    }   
                }
            },
            Err(e) => {
                eprintln!("Error preprocessing file: {}", e);
                panic!("Failed to preprocess file: {}", e);
            }
        }
        
    }
    let preproc_end = std::time::Instant::now();
}

#[test]
fn test_preprocessing() {
        let example_text = "
        #if SERVER && (MP || DEV)
            float first_if = 1.0;
            #if DEV
            int y = 3
            int z = 4
            #else
            int x = 1;
            #endif
        #endif
        bool some_value = true;
        #if SERVER
            float server_only = 2.0;
        #endif
    ";
    let tokens: Vec<sqparse::TokenItem<'static>> = sqparse::tokenize(example_text, sqparse::Flavor::SquirrelRespawn).into_tokens();
    let parsed = preprocessor_parser::preprocess(&tokens).unwrap();
    println!("Parsed Preprocessor: {:#?}", parsed);
    let mut any = false;

    for item in &parsed {
        for error in item.find_errors() {
            any = true;
            println!("Error: {}", error.text());
            println!("Range: {:?}", error.range());
        }
    }
    if any {
        panic!("Errors found in preprocessor parsing");
    }

    let runon = "SERVER || CLIENT";
    let preprocessor = preprocessor::Preprocessor::new();
    let valid = preprocessor.identify_valid_states(runon, &parsed);
    let map = &preprocessor.term_map;
    for state in valid.as_ref().unwrap() {
        println!("State: {}", state);

    }
    println!("Term Map: {:#?}", map);
    let mock_mod = Mod{
        name: "TestMod".to_string(),
        root: PathBuf::from("test"),
        mod_path: PathBuf::from("test/mod"),
        script_path: PathBuf::from("test/mod/scripts"),
        load_priority: 0,
        depstrings: HashMap::new(),
        loads: vec![],
    };
    let mock_info = FileInfo::new("name".into(), PathBuf::from("hi"), "run".into(), mock_mod.into());
    mock_info.set_text(example_text.to_string());
    let mock_file = LoadedFile {
        load_position: 0,
        file_type: mod_loader::fileinfo::FileType::Script,
        file_info: mock_info,
    };
    let split = preprocessor.split_ast(&parsed, valid.unwrap(), &mock_file, tokens.len());
    for variant in split {
        println!("Variant ( {} ):\n {}\n\n", variant.id.state, variant.to_text());
    }
    println!("Doing real preprocessing: ");
    let preprocessed = preprocessor.preprocess_file(mock_file).unwrap();
    let variants = preprocessed.variants;
    for variant in variants.iter() {
        println!("Variant ID: {}", variant.id.state);
        println!("Variant Text: \n{}\n\n", variant.to_text());
    }
}


#[test]
fn sqparse_test() {
    let text = "
    void function test() {
        int x = 1;
    }

    struct thing {
    int a
    }
    ";
    let tokens = sqparse::tokenize(text, sqparse::Flavor::SquirrelRespawn).into_tokens();
    let ast = sqparse::parse(&tokens, sqparse::Flavor::SquirrelRespawn);
    println!("Tokens: {:#?}", ast);
    
}


}