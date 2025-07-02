use std::{collections::HashMap, fmt::Display, path::PathBuf, sync::{Arc, RwLock}};

use url::Url;

use crate::{fileinfo::{ExternalType, FileInfo, FileType, LoadedFile, ModScriptEntry}};



#[derive(Debug, Clone)]
pub enum FileSource {
    DirectExternals(Arc<Vec<PathBuf>>),//TODO: Should each one be a seperate entry?
    Mod(Arc<Mod>),
    Vanilla(Arc<VanillaFiles>),//Vanilla is split across multiple paths
    Dummy//Testing, saves me effort later
}
impl From<Mod> for FileSource {
    fn from(mod_: Mod) -> Self {
        FileSource::Mod(Arc::new(mod_))
    }
}
impl From<VanillaFiles> for FileSource {
    fn from(vanilla: VanillaFiles) -> Self {
        FileSource::Vanilla(Arc::new(vanilla))
    }
}
impl From<Arc<Mod>> for FileSource {
    fn from(mod_: Arc<Mod>) -> Self {
        FileSource::Mod(mod_)
    }
}
impl From<Arc<VanillaFiles>> for FileSource {
    fn from(vanilla: Arc<VanillaFiles>) -> Self {
        FileSource::Vanilla(vanilla)
    }
}
impl Display for FileSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileSource::Mod(mod_) => write!(f, "Mod {{ name: {}, at: {} }}", mod_.name, mod_.mod_path.display()),
            FileSource::Vanilla(vanilla) => write!(f, "VanillaFiles {{ root: {} }}", vanilla.root.display()),
            FileSource::DirectExternals(externs) => write!(f, "Externals {{ count: {} }}", externs.len()),
            FileSource::Dummy => write!(f, "Dummy"),
        }
    }
}

impl FileSource {
    pub fn long_text(&self) -> String {
        match self {
            FileSource::Mod(mod_) => mod_.long_text(),
            FileSource::Vanilla(vanilla) => vanilla.long_text(),
            FileSource::DirectExternals(externs) => {
                let mut buff = format!("DirectExternals: count {}\n", externs.len());
                for path in externs.iter() {
                    buff.push_str(&format!("  - {}\n", path.display()));
                }
                buff
            }
            FileSource::Dummy => "Dummy FileSource".to_string(),
        }
    }
    pub fn loads(&self) -> Box<dyn Iterator<Item = &ModScriptEntry> + '_> {
        match self {
            FileSource::Mod(mod_) => Box::new(mod_.loads.iter()),
            FileSource::Vanilla(vanilla) => Box::new(vanilla.loads.iter()),
            FileSource::DirectExternals(_) => Box::new(std::iter::empty()), //No loads for direct externals
            FileSource::Dummy => Box::new(std::iter::empty()),
        }
    }
    pub fn try_file(&self, to_load: &ModScriptEntry) -> Option<FileInfo> {
        match self {
            FileSource::DirectExternals(_) => None, //Already handled in get_externs
            FileSource::Mod(_mod) => {
                if let Some(path) = _mod.try_file(to_load) {
                    return Some(FileInfo::new(
                            _mod.name.clone(), path, to_load.run_on.clone(), self.clone()
                ));}
                None
            }
            FileSource::Vanilla(vanilla) => {
                if let Some(path) = vanilla.try_file(to_load) {
                    return Some(FileInfo::new(
                        "Vanilla".to_string(), path, to_load.run_on.clone(), self.clone()
                    ));
                }
                None
            }
            FileSource::Dummy => {
                //Dummy files are just for testing, so we can return a dummy path
                Some(FileInfo::new(
                    "Dummy".to_string(), PathBuf::from("Testing only stuff"), to_load.run_on.clone(), self.clone()
                ))
            }
        }
    }
    pub fn get_externs(&self) -> Vec<LoadedFile> {
        match self {
            FileSource::Mod(mod_) => mod_.get_externs(),
            FileSource::Vanilla(vanilla) => Vec::new(),
            FileSource::DirectExternals(externs) => {
                externs.iter().map(|path| LoadedFile {
                    file_info: FileInfo::new(
                        "DirectExternals".to_string(), path.clone(), String::new(), self.clone()
                    ),
                    load_position: 0,
                    file_type: FileType::Externals(ExternalType::Externals),
                }).collect()
            }
            FileSource::Dummy => Vec::new(), //No externals for dummy files
        }
    }
}
#[derive(Debug)]
pub struct VanillaFiles {
    ///In this case, pretty much whatever folder we were pointed at initially
    pub root: PathBuf,
    ///Path to the main RSON file
    pub main_path: PathBuf,
    pub script_paths: RwLock<Vec<PathBuf>>,
    pub loads: Vec<ModScriptEntry>,
}
impl VanillaFiles {
    pub fn long_text(&self) -> String {
        let mut buff = format!("VanillaFiles: root {}\n", self.root.display());
        buff.push_str(&format!("Main Path: {}\n", self.main_path.display()));
        buff.push_str("Script Paths:\n");
        for path in &*self.script_paths.read().unwrap() {
            buff.push_str(&format!("  - {}\n", path.display()));
        }
        buff.push_str("Loads:\n");
        for load in &self.loads {
            buff.push_str(&format!("  - {}\n", load.get_name()));
        }
        buff
    }
    pub fn try_file(&self, to_load: &ModScriptEntry) -> Option<PathBuf> {
        for script_path in self.script_paths.read().unwrap().iter() {
            if script_path.join(&to_load.rel_path).exists() {
                return Some(script_path.join(&to_load.rel_path));
            }
        }
        None
    }
}
#[derive(Debug, Clone)]
pub struct Mod {
    /// Mod root, mod_path will always be this/mod.json
    pub root: PathBuf,
    /// Path to mod.json or scripts.rson
    pub mod_path: PathBuf,
    /// Mod root
    pub script_path: PathBuf,
    pub load_priority: usize,
    pub name: String,
    pub loads: Vec<ModScriptEntry>,
    pub depstrings: HashMap<String, String>,
}


impl Display for Mod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mod {{ name: {}, at: {} }}", self.name, self.mod_path.display())
    }
}
impl Mod {
    pub fn long_text(&self) -> String {
        let mut buff = format!("Mod: name {}\n", self.name);
        buff.push_str(&format!("Root: {}\n", self.root.display()));
        buff.push_str(&format!("Mod Path: {}\n", self.mod_path.display()));
        buff.push_str(&format!("Script Path: {}\n", self.script_path.display()));
        buff.push_str(&format!("Load Priority: {}\n", self.load_priority));
        buff.push_str("Loads:\n");
        for load in &self.loads {
            buff.push_str(&format!("  - {}\n", load.get_name()));
        }
        buff.push_str("Dependencies:\n");
        for (key, value) in &self.depstrings {
            buff.push_str(&format!("  - {}: {}\n", key, value));
        }
        buff
    }
    pub fn get_externals_file(self: &Arc<Self>) -> Option<LoadedFile> {

        let file = self.root.join("externals.json");
        if file.exists() {
            return Some(LoadedFile {
                file_info: FileInfo::new(
                    self.name.clone() + "Externals", file, String::new(), FileSource::Mod(self.clone())
                ),
                load_position: 0,//These always come first
                file_type: FileType::Externals(ExternalType::Externals),

            })
        }
        None
    }
    pub fn get_native_funcs_file(self: &Arc<Self>) -> Option<LoadedFile> {
        let file = self.root.join("nativefuncs.json");
        if file.exists() {
            return Some(LoadedFile {
                file_info: FileInfo::new(
                    self.name.clone() + "NativeFuncs", file, String::new(), FileSource::Mod(self.clone())
                ),
                load_position: 0,//These always come first
                file_type: FileType::Externals(ExternalType::NativeFuncs),
            })
        }
        None
    }
    pub fn get_externs(self: &Arc<Self>) -> Vec<LoadedFile> {
        let mut files = Vec::new();
        if let Some(file) = self.get_externals_file() {
            files.push(file);
        }
        if let Some(file) = self.get_native_funcs_file() {
            files.push(file);
        }
        files
    }
    pub fn try_file(&self, to_load: &ModScriptEntry) -> Option<PathBuf> {
        if self.script_path.join(&to_load.rel_path).exists() {
            return Some(self.script_path.join(&to_load.rel_path));
        }
        None
    }
}