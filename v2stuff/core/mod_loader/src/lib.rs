use indexmap::IndexMap;
use peg::error::ParseError;
use core::error;
use std::fmt::Debug;
use std::sync::Arc;
use std::{path::Path};
use std::path::{self, PathBuf};

use crate::mods::{FileSource, VanillaFiles};
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
impl LoadConfig {
    pub fn reject(&self, files: &FileSource) -> bool {
        match files {
            FileSource::DirectExternals(_) => false, // This is things directly loaded from config, so why would we reject them
            FileSource::Mod(mod_item) => {
                self.disabled_names.contains(&mod_item.name) ||
                self.disabled_paths.contains(&mod_item.mod_path)
            }
            FileSource::Vanilla(files) => {
                self.disabled_names.contains(&"scripts.rson".into()) ||
                self.disabled_paths.contains(&files.root)
            }
        }
    }
}

pub struct LoadOrder {
    pub config: LoadConfig,
    pub loaded: Vec<(FileSource, ModScriptEntry)>,
}

#[derive(Debug, Clone)]
pub struct ModLoader {
    mods: IndexMap<usize, Vec<FileSource>>, // Load priority, mods in order of loaded
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
    pub fn get_mods(&self) -> Vec<&FileSource> {
        self.mods.values().flat_map(|v| v.iter()).collect()
    }
    pub fn get_mods_config<'a>(
        &'a self,
        config: &'a LoadConfig,
    ) -> impl DoubleEndedIterator<Item = &'a FileSource> {
        self.mods.values().flat_map(|v| v.iter()).filter(move |m| {
            !config.reject(m)
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
            for script in found_mod.loads() {
                if let Some((_, existing)) = load_order.get(&script.rel_path) {
                    let existing: &ModScriptEntry = existing;
                    errors.push(ModLoadError::DuplicateFiles(
                        script.rel_path.clone(),
                        existing.rel_path.clone(),
                    ));
                    continue;
                }
                load_order.insert(script.rel_path.clone(), (found_mod.clone(), script.clone()));
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
    /// OR just use an actual file searching crate, which is literally build for this and probably 100* faster
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
        for (idx, (source_mod, to_load)) in config.loaded.iter().enumerate() {
            let mut found = false;
            for m in to_search.iter() {
                if let Some(file) = m.try_file(to_load) {
                    files.push(LoadedFile {
                        file_info: file,
                        file_type: fileinfo::FileType::Script,
                        load_position: idx + 1,
                    });
                    found = true;
                    break;
                }
                if found {
                    break;
                }
            }
            if !found {
                errors.push(ModLoadError::NotLoaded(source_mod.clone(), to_load.clone()));
            }
        }
        let files = externals
            .into_iter()
            .chain(files)
            .collect::<Vec<_>>();
        (files, errors)
    }

    pub fn register_additional_rson_roots(
        &self,
        root_path: &Path, 
        additional_paths: &[PathBuf],
    ) -> (bool, Vec<ModLoadError>) {
        let mut errors = Vec::new();
        let root = self.mods.values().flat_map(|v| v.iter())
            .find_map(|m| match m { FileSource::Vanilla(v) => Some(v), _ => None });
        if root.is_none() {
            errors.push(ModLoadError::CouldNotFindMod(root_path.to_path_buf()));
            return (false, errors);
        }
        let root = root.unwrap();
        for additional_path in additional_paths {
            let mut additional_path: &Path = additional_path;
            if !additional_path.exists() {
                errors.push(ModLoadError::CouldNotFindMod(additional_path.to_path_buf()));
                continue;
            }
            if additional_path.is_file() {
                additional_path = additional_path.parent().unwrap_or(additional_path);   
            }
            root.script_paths.write().unwrap().push(additional_path.to_path_buf());
        }

        (true, errors)

    }

    /// Recursively search for mod.json and mod.rson files, up to the depth limit (currently hardcoded to 6, which i made up for no reason)
    /// Returns the number of loaded mods
    pub fn register_workspace(&mut self, root_path: &Path) -> Result<usize, ModLoadError> {
        //TODO: Replace this with the rust_search package if its worthwhile (probably is because my code is shit)
        self.find_mods(root_path,  root_path, 0)
    }

    /// Recursively search for mod.json and mod.rson files, up to the depth limit (currently hardcoded to 3)
    /// Returns the number of loaded mods
    /// TODO: Warn the user if this takes longer than a few ms, this is very inefficient and doesnt really need to recurr
    fn find_mods(
        &mut self,
        root_path: &Path, //Might track this? but not much point, this is the root of the search not the mod itself so kinda useless
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
                    .push(FileSource::Mod(Arc::new(mod_item)));
                return Ok(1);
        } else if current_path.join("scripts.rson").exists() {
            let mod_item = VanillaFiles::from_rson(root_path, current_path)?;
            self.mods
                .entry(0)
                .or_default()
                .push(FileSource::Vanilla(Arc::new(mod_item)));
            return Ok(1);
        }
        let dir = current_path.read_dir()?;
        let mut loaded_count = 0;
        for entry in dir {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively search in subdirectories
                loaded_count += self.find_mods(root_path, &path, depth + 1)?;
            } 
        }
        Ok(loaded_count)
    }
    pub fn get_mod(&self, name: &str) -> Option<&Arc<Mod>> {
        self.mods
            .values()
            .flat_map(|v| v.iter())
            .filter_map(|m| match m {
                FileSource::Mod(mod_) => Some(mod_),
                _ => None,
            })
            .find(|m| m.name == name)
    }


}

pub enum ModLoadError {
    Io(std::io::Error),
    Serde(serde_json::Error),
    Peg(ParseError<peg::str::LineCol>),
    DuplicateMods(PathBuf, PathBuf),
    CouldNotFindMod(PathBuf), // Mod not found in the workspace
    DuplicateFiles(PathBuf, PathBuf), // Duplicate file paths in different mods
    
    NotLoaded(FileSource, ModScriptEntry), //TODO: Why doesn't this track the mod that registered it, stupid
}
impl Debug for ModLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModLoadError::Io(e) => write!(f, "IO Error: {}", e),
            ModLoadError::Serde(e) => write!(f, "Serde Error: {}", e),
            ModLoadError::Peg(e) => write!(f, "PEG Parse Error: {}", e),
            ModLoadError::DuplicateMods(a, b) => write!(f, "Duplicate mods found: {} and {}", a.display(), b.display()),
            ModLoadError::CouldNotFindMod(p) => write!(f, "Could not find mod at path: {}", p.display()),
            ModLoadError::DuplicateFiles(p, m) => write!(f, "Duplicate file found at path: {} in mod: {}", p.display(), m.display()),
            ModLoadError::NotLoaded(mod_item, script_entry) => write!(f, "Script entry '{}' required by mod '{}' was not found", script_entry.rel_path.display(), mod_item),
        }
    }
}
impl ModLoadError {
    pub fn is_concerning(&self) -> bool {
        match self {
            ModLoadError::DuplicateFiles(left, right) => left != right, 
            ModLoadError::NotLoaded(_mod, _) => !matches!(_mod, FileSource::Vanilla(_)), //Scripts.rson has a bunch of files that arent loaded, 
            //presumably they were removed in retail
            _=> true
        }
    }
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

}
