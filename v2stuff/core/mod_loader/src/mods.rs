use std::{collections::HashMap, path::PathBuf};

use url::Url;

use crate::{fileinfo::{ExternalType, FileInfo, FileType, LoadedFile, ModScriptEntry}, ModID};

/// Not going to make this, I don't feel like trying to setup actual web requests and handling shit
/// *Maybe* I'll just have it download the file but like, do you want something I wrote downloading things?
/// I don't trust me not to accidentally break everything, you definitely shouldn't
/// I might handle online mods by having the LSP download them, and not this.
pub struct RemoteMod {
    //No, not doing it
    pub root: Url,
    pub path: Url,
    pub load_priority: usize,
    pub name: String,
    pub loads: Vec<ModScriptEntry>,
}
#[derive(Debug, Clone)]
pub struct Mod {
    pub root: PathBuf,
    pub mod_path: PathBuf,
    pub script_path: PathBuf,
    pub load_priority: usize,
    pub name: String,
    pub loads: Vec<ModScriptEntry>,
    pub depstrings: HashMap<String, String>,
}
impl Mod {
    pub fn get_externals_file(&self, mod_id: ModID) -> Option<LoadedFile> {
        let file = self.root.join("externals.json");
        if file.exists() {
            Some(LoadedFile {
                file_info: FileInfo::new(
                    self.name.clone() + "Externals", file, String::new(), mod_id
                ),
                load_position: 0,//These always come first
                file_type: FileType::Externals(ExternalType::Externals),

            })
        } else {
            None
        }
    }
    pub fn get_native_funcs_file(&self, mod_id: ModID) -> Option<LoadedFile> {
        let file = self.root.join("nativefuncs.json");
        if file.exists() {
            Some(LoadedFile {
                file_info: FileInfo::new(
                    self.name.clone() + "NativeFuncs", file, String::new(), mod_id
                ),
                load_position: 0,//These always come first
                file_type: FileType::Externals(ExternalType::NativeFuncs),
            })
        } else {
            None
        }
    }
    pub fn get_externs(&self, mod_id: ModID) -> Vec<LoadedFile> {
        let mut files = Vec::new();
        if let Some(file) = self.get_externals_file(mod_id) {
            files.push(file);
        }
        if let Some(file) = self.get_native_funcs_file(mod_id) {
            files.push(file);
        }
        files
    }
}