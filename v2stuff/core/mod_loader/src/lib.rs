use indexmap::IndexMap;
use peg::error::ParseError;
use std::fmt::Display;
use std::path::PathBuf;

use crate::{
    fileinfo::{FileInfo, LoadedFile, ModScriptEntry},
    mods::Mod,
};

pub mod fileinfo;
pub mod mod_json;
pub mod mods;
pub mod rson;
/*
    Needed lsp configs:
    Point to mod root folders to discover jsons/rsons
    Point to externals (Externals should be discoverable from a mod json, but might need extra ones)
*/

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModID(pub usize);
impl Display for ModID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ModID({})", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoadConfig {
    pub disabled_names: Vec<String>,  //Mods by name
    pub disabled_paths: Vec<PathBuf>, //Mods by path (root or to json)
}

pub struct LoadOrder {
    pub config: LoadConfig,
    pub loaded: Vec<ModScriptEntry>,
}

pub struct ModLoader {
    mods: IndexMap<usize, Vec<Mod>>, // Load priority, mods in order of loaded
}
impl Default for ModLoader {
    fn default() -> Self {
        Self {
            mods: IndexMap::new(),
        }
    }
}
impl ModLoader {
    ///TODO: Watch mod files for changes.
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get_mods(&self) -> Vec<&Mod> {
        self.mods.values().flat_map(|v| v.iter()).collect()
    }
    pub fn get_mods_config<'a>(
        &'a self,
        config: &'a LoadConfig,
    ) -> impl DoubleEndedIterator<Item = &'a Mod> {
        self.mods.values().flat_map(|v| v.iter()).filter(move |m| {
            !config.disabled_names.contains(&m.name) && !config.disabled_paths.contains(&m.mod_path)
        })
    }
    pub fn initialize(&mut self) -> (LoadOrder, Vec<ModLoadError>) {
        let config = LoadConfig::default();
        self.initialize_with_config(config)
    }
    /// In the future, I intend for it to be possible to load multiple configs, hence why this finds everything THEN filters it
    pub fn initialize_with_config(&mut self, config: LoadConfig) -> (LoadOrder, Vec<ModLoadError>) {
        let mut errors = Vec::new();
        let mut load_order = IndexMap::new();
        for found_mod in self.get_mods_config(&config) {
            for script in &found_mod.loads {
                if load_order.contains_key(&script.rel_path) {
                    errors.push(ModLoadError::DuplicateFiles(
                        script.rel_path.clone(),
                        found_mod.mod_path.clone(),
                    ));
                    continue;
                }
                load_order.insert(script.rel_path.clone(), script.clone());
            }
        }
        let loaded = load_order.values().collect::<Vec<_>>();
        let load_order = LoadOrder {
            loaded: loaded.into_iter().cloned().collect(),
            config,
        };
        (load_order, errors)
    }
    pub fn load(&mut self) -> (Vec<LoadedFile>, Vec<ModLoadError>) {
        let order = self.initialize().0;
        self.load_from_order(order)
    }

    /// Loads files from a given load order.
    ///
    /// For my own sanity, I am going to just say that external function jsons are loaded from the mod root, fight me
    /// 
    /// TODO: Is there any reason NOT to parallelize this?
    pub fn load_from_order(&mut self, config: LoadOrder) -> (Vec<LoadedFile>, Vec<ModLoadError>) {
        let mut externals = Vec::new();
        let mut files = Vec::new();
        let mut errors = Vec::new();
        let to_search = self
            .get_mods_config(&config.config).enumerate().map(|(idx, m)| {
                //Get the externals file
                externals.extend(m.get_externs(ModID(idx)));
                (idx, m)
            }).collect::<Vec<_>>();
        //First mod that has the thing is also the highest priority this way
        for (idx, to_load) in config.loaded.iter().enumerate() {
            let mut found = false;
            for (mod_idx, m) in to_search.iter().rev() {
                if m.script_path.join(&to_load.rel_path).exists() {
                    //This is a file we can load
                    let file = FileInfo::new(
                        //I'm not sure why i even really track this
                        to_load.get_name(),
                        m.mod_path.join(&to_load.rel_path),
                        to_load.run_on.clone(),
                        ModID(*mod_idx),
                    );
                    files.push(LoadedFile {
                        file_info: file,
                        file_type: fileinfo::FileType::Script,
                        load_position: idx + 1,
                    });
                    found = true;
                    break;
                }
            }
            if !found {
                errors.push(ModLoadError::NotLoaded(to_load.clone()));
            }
        }
        let files = externals
            .into_iter()
            .chain(files)
            .collect::<Vec<_>>();
        (files, errors)
    }

    /// Recursively search for mod.json and mod.rson files, up to the depth limit (currently hardcoded to 6, which i made up for no reason)
    /// Returns the number of loaded mods
    pub fn register_workspace(&mut self, root_path: PathBuf) -> Result<usize, ModLoadError> {
        self.find_mods(root_path.clone(), root_path, 0)
    }

    /// Recursively search for mod.json and mod.rson files, up to the depth limit (currently hardcoded to 6)
    /// Returns the number of loaded mods
    fn find_mods(
        &mut self,
        root_path: PathBuf,
        current_path: PathBuf,
        depth: usize,
    ) -> Result<usize, ModLoadError> {
        if depth > 6 {
            return Ok(0); // Prevent too deep recursion
        }

        let dir = current_path.read_dir()?;
        let mut loaded_count = 0;
        for entry in dir {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively search in subdirectories
                loaded_count += self.find_mods(root_path.clone(), path, depth + 1)?;
            } else if path.file_name().is_some_and(|name| name == "mod.json") {
                let mod_item = Mod::from_json(current_path.clone(), path)?;
                loaded_count += 1;
                if let Some(existing_mod) = self.get_mod(&mod_item.name) {
                    return Err(ModLoadError::DuplicateMods(
                        existing_mod.mod_path.clone(),
                        mod_item.mod_path,
                    ));
                }
                self.mods
                    .entry(mod_item.load_priority)
                    .or_default()
                    .push(mod_item);
            } else if path.extension().is_some_and(|ext| ext == "rson") {
                let mod_item = Mod::from_rson(current_path.clone(), path)?;
                loaded_count += 1;
                self.mods
                    .entry(mod_item.load_priority)
                    .or_default()
                    .push(mod_item);
            }
        }
        Ok(loaded_count)
    }
    pub fn get_mod(&self, name: &str) -> Option<&Mod> {
        self.mods
            .values()
            .flat_map(|v| v.iter())
            .find(|m| m.name == name)
    }
}

#[derive(Debug)]
pub enum ModLoadError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    Peg(ParseError<peg::str::LineCol>),
    DuplicateMods(PathBuf, PathBuf),
    DuplicateFiles(PathBuf, PathBuf), // Duplicate file paths in different mods
    NotLoaded(ModScriptEntry), //TODO: Why doesn't this track the mod that registered it, stupid
}

impl From<std::io::Error> for ModLoadError {
    fn from(e: std::io::Error) -> Self {
        ModLoadError::Io(e)
    }
}
impl From<serde_json::Error> for ModLoadError {
    fn from(e: serde_json::Error) -> Self {
        ModLoadError::Serde(e)
    }
}
impl From<ParseError<peg::str::LineCol>> for ModLoadError {
    fn from(e: ParseError<peg::str::LineCol>) -> Self {
        ModLoadError::Peg(e)
    }
}

#[cfg(test)]
mod tests {
    ///Look, i know that i SHOULD be writing proper tests, but fuck that i dont want to right now
    use super::*;
    #[test]
    fn test_single_json() {
        let mut loader = ModLoader::new();
        let path = "../../../TestSets/ValidStructure/external/";
        let path = PathBuf::from(path);
        let count = loader.register_workspace(path).unwrap();
        assert_eq!(count, 1);
        let order = loader.initialize().0;
        let (files, errors) = loader.load_from_order(order);
        for error in &errors {
            println!("Error loading file: {:?}", error);
        }
        assert!(errors.is_empty());
        for file in files {
            println!("{}", file);
        }
    }
    #[test]
    fn test_multi_json() {
        let mut loader = ModLoader::new();
        let path = "../../../TestSets/ValidStructure/";
        let path = PathBuf::from(path);
        let count = loader.register_workspace(path).unwrap();
        assert_eq!(count, 5);
        let order = loader.initialize().0;
        let (files, errors) = loader.load_from_order(order);
        for error in &errors {
            println!("Error loading file: {:?}", error);
        }
        assert!(errors.is_empty());
        let externs = files.iter().take(4).collect::<Vec<_>>();
        assert!(externs.iter().all(|f| matches!(f.file_type, fileinfo::FileType::Externals(_))));

        for file in files {
            println!("{}", file);
        }
    }
    /// I don't feel like sticking the entire titanfall script dump here, so i will simply not bother with rson tests
    /// This is stupid and lazy but like, really if i wanted to i could hardcode the entire rson, you should never change it
    /// So the actual logic really doesnt need to be that great, its literally only going to parse a single file that i already know the contents of 
    #[test]
    fn test_rson() {}
}
