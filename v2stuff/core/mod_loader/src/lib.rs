use indexmap::IndexMap;
use peg::error::ParseError;
use std::sync::Arc;
use std::{path::Path};
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


#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct LoadConfig {
    pub disabled_names: Vec<String>,  //Mods by name
    pub disabled_paths: Vec<PathBuf>, //Mods by path (root or to json)
}

pub struct LoadOrder {
    pub config: LoadConfig,
    pub loaded: Vec<ModScriptEntry>,
}

#[derive(Debug, Clone)]
pub struct ModLoader {
    mods: IndexMap<usize, Vec<Arc<Mod>>>, // Load priority, mods in order of loaded
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
    pub fn get_mods(&self) -> Vec<&Arc<Mod>> {
        self.mods.values().flat_map(|v| v.iter()).collect()
    }
    pub fn get_mods_config<'a>(
        &'a self,
        config: &'a LoadConfig,
    ) -> impl DoubleEndedIterator<Item = &'a Arc<Mod>> {
        self.mods.values().flat_map(|v| v.iter()).filter(move |m| {
            !config.disabled_names.contains(&m.name) && !config.disabled_paths.contains(&m.mod_path)
        })
    }
    fn replace_mod(&mut self, old: Arc<Mod>, new: Arc<Mod>) -> Result<(), ModLoadError> {
        let priority = old.load_priority;
        if let Some(mods) = self.mods.get_mut(&priority) {
            if let Some(index) = mods.iter().position(|m| Arc::ptr_eq(m, &old)) {
                if old.load_priority == new.load_priority {
                    mods.remove(index);
                } else {
                    mods[index] = new;
                    return Ok(());
                }
            }
        } else {
            return Err(ModLoadError::CouldNotFindMod(old.mod_path.clone()))
        }
        for (p, mods) in &mut self.mods {
            if p == &new.load_priority {
                mods.push(new);
                return Ok(());
            }
        }
        Err(ModLoadError::CouldNotFindMod(new.mod_path.clone()))
        
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
    pub fn reload_mod(&mut self, mod_path: &Path) -> Result<bool, ModLoadError> {
        if let Some(mod_item) = self.get_mod_by_abs_path(mod_path) {
                let new  = {
                    if mod_item.name == "scripts.rson"{
                        Mod::from_rson(&mod_item.root, mod_item.mod_path.clone())?
                    } else {
                        Mod::from_json(&mod_item.root, mod_item.mod_path.clone())?
                    }
                };
                self.replace_mod(mod_item.clone(), Arc::new(new))?;
            }
        todo!()
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
            .get_mods_config(&config.config).map(|m| {
                //Get the externals file
                externals.extend(m.get_externs());
                m.clone()
            }).rev().collect::<Vec<_>>();
        //First mod that has the thing is also the highest priority this way
        for (idx, to_load) in config.loaded.iter().enumerate() {
            let mut found = false;
            for m in to_search.iter() {
                if m.script_path.join(&to_load.rel_path).exists() {
                    //This is a file we can load
                    let file = FileInfo::new(
                        //I'm not sure why i even really track this
                        to_load.get_name(),
                        m.script_path.join(&to_load.rel_path),
                        to_load.run_on.clone(),
                        m.clone(),
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
    pub fn register_workspace(&mut self, root_path: &Path) -> Result<usize, ModLoadError> {
        //TODO: Replace this with the rust_search package if its worthwhile (probably is because my code is shit)
        self.find_mods(/*root_path,*/ root_path, 0)
    }

    /// Recursively search for mod.json and mod.rson files, up to the depth limit (currently hardcoded to 3)
    /// Returns the number of loaded mods
    /// TODO: Warn the user if this takes longer than a few ms, this is very inefficient and doesnt really need to recurr
    fn find_mods(
        &mut self,
        //root_path: &Path, //Might track this? but not much point, this is the root of the search not the mod itself so kinda useless
        current_path: &Path,
        depth: usize,
    ) -> Result<usize, ModLoadError> {
        if depth >= 3 {
            return Ok(0); // Prevent too deep recursion
        }
        if current_path.join("mod.json").exists() {
                let mod_item = Mod::from_json(current_path, current_path.join("mod.json"))?;
                if let Some(existing_mod) = self.get_mod(&mod_item.name) {
                    return Err(ModLoadError::DuplicateMods(
                        existing_mod.mod_path.clone(),
                        mod_item.mod_path,
                    ));
                }
                self.mods
                    .entry(mod_item.load_priority)
                    .or_default()
                    .push(Arc::new(mod_item));
                return Ok(1);
        } else if current_path.join("scripts.rson").exists() {
            let mod_item = Mod::from_rson(current_path, current_path.join("scripts.rson"))?;
            self.mods
                .entry(mod_item.load_priority)
                .or_default()
                .push(Arc::new(mod_item));
            return Ok(1);
        }
        let dir = current_path.read_dir()?;
        let mut loaded_count = 0;
        for entry in dir {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively search in subdirectories
                loaded_count += self.find_mods(/*root_path,*/ &path, depth + 1)?;
            } 
        }
        Ok(loaded_count)
    }
    pub fn get_mod(&self, name: &str) -> Option<&Arc<Mod>> {
        self.mods
            .values()
            .flat_map(|v| v.iter())
            .find(|m| m.name == name)
    }
    pub fn get_mod_by_abs_path(&self, path: &Path) -> Option<&Arc<Mod>> {
        self.mods
            .values()
            .flat_map(|v| v.iter())
            .find(|m| m.mod_path.canonicalize().is_ok_and(|x| x == path))
    }
}

#[derive(Debug)]
pub enum ModLoadError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    Peg(ParseError<peg::str::LineCol>),
    DuplicateMods(PathBuf, PathBuf),
    CouldNotFindMod(PathBuf), // Mod not found in the workspace
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
        let count = loader.register_workspace(&path).unwrap();
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
        let count = loader.register_workspace(&path).unwrap();
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
