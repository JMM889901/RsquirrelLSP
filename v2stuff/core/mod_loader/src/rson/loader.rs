use std::{collections::HashMap, path::PathBuf};

use crate::{mods::Mod, rson::parser::rson};

impl Mod {
    pub fn from_rson(root: PathBuf, file: PathBuf) -> Result<Self, crate::ModLoadError> {
        let text = std::fs::read_to_string(&file)?;
        let scripts = rson::rson(&text)?;
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("scripts.rson")
            .to_string();
        Ok(Self {
            script_path: root.clone(), //Again, assuming this is just coming from vanilla, where the rson is already in the scripts folder
            root,
            mod_path: file,
            load_priority: 0,
            name,
            loads: scripts,
            depstrings: HashMap::new(),
        })
    }
}
