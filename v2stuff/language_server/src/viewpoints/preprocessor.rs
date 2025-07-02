use std::path::{Path, PathBuf};

use tower_lsp::lsp_types::Url;

use crate::Backend;


impl Backend {
    pub fn visualize_with_state(
        &self,
        state: &str,
        file: &Path,
    ) -> Result<(), String> {
        //i REALLY need to decide whether i want to use urls, pathbufs, or canonicalized pathbufs
        let results = &self.configs.read().unwrap();
        let file = results.values()
            .find_map(|config| config.last_save.preprocessed.iter().find(|f| f.file.abs_path_eq(file)));
        if let Some(file) = file {
            let locks = self.preprocessor.preprocessor.build_specific_variant(file, state)?;

        }
        Ok(())
    }
}