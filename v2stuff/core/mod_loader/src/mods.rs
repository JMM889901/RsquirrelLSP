use std::{collections::HashMap, fmt::Display, path::PathBuf, sync::Arc};

use url::Url;

use crate::{fileinfo::{ExternalType, FileInfo, FileType, LoadedFile, ModScriptEntry}};

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
    /// The mod/file root
    pub root: PathBuf,
    /// Path to mod.json or scripts.rson
    pub mod_path: PathBuf,
    /// Path to the scripts/vscripts folder
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
    pub fn get_externals_file(self: &Arc<Self>) -> Option<LoadedFile> {
        let file = self.root.join("externals.json");
        if file.exists() {
            Some(LoadedFile {
                file_info: FileInfo::new(
                    self.name.clone() + "Externals", file, String::new(), self.clone()
                ),
                load_position: 0,//These always come first
                file_type: FileType::Externals(ExternalType::Externals),

            })
        } else {
            None
        }
    }
    pub fn get_native_funcs_file(self: &Arc<Self>) -> Option<LoadedFile> {
        let file = self.root.join("nativefuncs.json");
        if file.exists() {
            Some(LoadedFile {
                file_info: FileInfo::new(
                    self.name.clone() + "NativeFuncs", file, String::new(), self.clone()
                ),
                load_position: 0,//These always come first
                file_type: FileType::Externals(ExternalType::NativeFuncs),
            })
        } else {
            None
        }
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
}