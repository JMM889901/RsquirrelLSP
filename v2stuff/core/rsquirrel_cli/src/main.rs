use core::error;
use std::{env, path::PathBuf};
use rayon::prelude::*;
use mod_loader::{fileinfo::{FileInfo, LoadedFile}, LoadConfig, ModID, ModLoader};
use preprocessor_parser::preprocessor_parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    todo!()
}

#[cfg(test)]
mod tests{
    use core::panic;

    use compiler_state::term;
    use mod_loader::fileinfo::FileType;

    use super::*;
#[test]
fn test_northstar() {
    let mut loader = ModLoader::new();
    //let path = PathBuf::from("../../../TestSets/ValidStructure/external");
    let path = PathBuf::from("../../../../northstar");
    let count = loader.register_workspace(path);
    if count.is_err() {//This will likely happen, since this is baseed on my file structure, if you are testing this yourself
        //Cry about it i guess?
        eprintln!("Error registering workspace: {:?}", count.err().unwrap());
        return;
    }
    println!("Registered {} mods", count.unwrap());
    let mods = loader.get_mods();
    for _mod in mods {
        println!("Mod: {}", _mod.name);
        println!("Root: {:?}", _mod.root);
        println!("Path: {:?}", _mod.mod_path);
        println!("Script Path: {:?}", _mod.script_path);
        println!("Load Priority: {}", _mod.load_priority);
        println!("Dependencies: {:?}", _mod.depstrings);
    }
    let config = LoadConfig{
        disabled_names: vec!["Northstar.Client".to_string()],
        ..Default::default()
    };
    let unfiltered_files = loader.load().0;
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
                for v in &preproc.variants {
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

    let count = loader.register_workspace(path);
    if count.is_err() {//This will likely happen, since this is baseed on my file structure, if you are testing this yourself
        //Cry about it i guess?
        eprintln!("Error registering workspace: {:?}", count.err().unwrap());
        return;
    }
    println!("Registered {} mods", count.unwrap());
    let mods = loader.get_mods();
    for _mod in mods {
        println!("Mod: {}", _mod.name);
        println!("Root: {:?}", _mod.root);
        println!("Path: {:?}", _mod.mod_path);
        println!("Script Path: {:?}", _mod.script_path);
        println!("Load Priority: {}", _mod.load_priority);
        println!("Dependencies: {:?}", _mod.depstrings);
    }
    let end = std::time::Instant::now();
    println!("Time taken to register mods: {:?}", end.duration_since(start));
    let unfiltered_files = loader.load().0;
    let iter = unfiltered_files.par_iter();
    let preprocessor = preprocessor::Preprocessor::new();
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
                for v in &preproc.variants {
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
    println!("Found {} terms", preprocessor.term_map.read().unwrap().len());
    for term in preprocessor.term_map.read().unwrap().iter() {
        println!("Term: {}", term);
    }
}
#[test]
fn test_vanilla_overrided() {
    let vanilla_path = PathBuf::from("../../../../Titanfall2-master");
    let northstar_path = PathBuf::from("../../../../northstar");
    let mut loader = ModLoader::new();
    let load_vanilla_start = std::time::Instant::now();
    let count = loader.register_workspace(vanilla_path);
    let load_vanilla_end = std::time::Instant::now();
    if count.is_err() {//This will likely happen, since this is baseed on my file structure, if you are testing this yourself
        //Cry about it i guess?
        eprintln!("Error registering workspace: {:?}", count.err().unwrap());
        return;
    }
    let mut count = count.unwrap();
    let load_northstar_start = std::time::Instant::now();
    count += loader.register_workspace(northstar_path).unwrap();
    let load_northstar_end = std::time::Instant::now();
    println!("Registered {} mods", count);
    let mods = loader.get_mods();
    for _mod in mods {
        println!("Mod: {}", _mod.name);
        println!("Root: {:?}", _mod.root);
        println!("Path: {:?}", _mod.mod_path);
        println!("Script Path: {:?}", _mod.script_path);
        println!("Load Priority: {}", _mod.load_priority);
        println!("Dependencies: {:?}", _mod.depstrings);
    }
    let load_mods_start = std::time::Instant::now();
    let unfiltered_files = loader.load().0;
    let load_mods_end = std::time::Instant::now();

    let _items = unfiltered_files.iter().find(|item| item.file_info.name() == "_items.nut");
    if _items.is_none() {
        panic!("Failed to find items.nut in unfiltered files");
    }
    let items = _items.unwrap();
    println!("_items.nut: {:?}", items.file_info.path());
    assert!(items.file_info.path().to_string_lossy().contains("Northstar.CustomServers"));
    println!("Took {:?} to load vanilla scripts", load_vanilla_end.duration_since(load_vanilla_start));
    println!("Took {:?} to load northstar mods", load_northstar_end.duration_since(load_northstar_start));
    println!("Took {:?} to get files", load_mods_end.duration_since(load_mods_start));
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
    let mut preprocessor = preprocessor::Preprocessor::new();
    let valid = preprocessor.identify_valid_states(&runon.to_string(), &parsed);
    let map = &preprocessor.term_map;
    for state in valid.as_ref().unwrap() {
        println!("State: {}", state);

    }
    println!("Term Map: {:#?}", map);
    let mock_info = FileInfo::new("name".into(), PathBuf::from("hi"), "run".into(), ModID(0));
    mock_info.set_text(example_text.to_string());
    let mock_file = LoadedFile {
        load_position: 0,
        file_type: mod_loader::fileinfo::FileType::Script,
        file_info: mock_info,
    };
    let split = preprocessor.split_ast(&parsed, valid.unwrap(), &mock_file);
    for variant in split {
        println!("Variant ( {} ):\n {}\n\n", variant.id.state, variant.to_text());
    }
    println!("Doing real preprocessing: ");
    let preprocessed = preprocessor.preprocess_file(mock_file).unwrap();
    let variants = preprocessed.variants;
    for variant in variants {
        println!("Variant ID: {}", variant.id.state);
        println!("Variant Text: \n{}\n\n", variant.to_text());
    }
}
}