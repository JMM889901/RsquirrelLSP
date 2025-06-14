use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{fileinfo::ModScriptEntry, mods::Mod, ModLoadError};

impl Mod {
    pub fn from_json(root: PathBuf, file: PathBuf) -> Result<Self, ModLoadError> {
        let mod_pure: ModPure = serde_json::from_reader(std::fs::File::open(&file)?)?;
        Ok(Self {
            script_path: root.join("mod").join("scripts").join("vscripts"),
            root,
            mod_path: file,
            load_priority: mod_pure.load_priority,
            name: mod_pure.name,
            loads: mod_pure
                .scripts
                .into_iter()
                .map(|s| ModScriptEntry {
                    rel_path: PathBuf::from(s.path), //Local to root path
                    run_on: s.run_on,
                })
                .collect(),
            depstrings: mod_pure.dependencies,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModPure {
    #[serde(rename = "LoadPriority", default)]
    load_priority: usize,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Scripts", default)]
    pub scripts: Vec<Script>,
    #[serde(rename = "Dependencies", default)]
    pub dependencies: HashMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    #[serde(rename = "Path")]
    pub path: String,
    #[serde(rename = "RunOn")]
    pub run_on: String,
}
