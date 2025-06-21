use std::{path::{Path}, sync::RwLock};

use crate::{mods::{VanillaFiles}, rson::parser::rson};

impl VanillaFiles {
    pub fn from_rson(root: &Path, main: &Path) -> Result<Self, crate::ModLoadError> {
        let file = main.join("scripts.rson");
        let text = std::fs::read_to_string(&file)?;
        let scripts = rson::rson(&text)?;

        Ok(Self {
            script_paths: RwLock::new(vec![main.to_path_buf()]), 
            root: root.to_path_buf(),
            loads: scripts,
            main_path: main.to_path_buf(),
        })
    }
}
