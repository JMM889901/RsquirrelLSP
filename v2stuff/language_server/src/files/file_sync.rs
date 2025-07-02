use std::sync::Arc;

use mod_loader::LoadConfig;
use preprocessor::{analyser, ASTTokenIter, PreprocessedScript};
use rayon::iter::{Either, IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use runner::Analyser;
use tower_lsp::lsp_types::{notification::Progress, MessageType};

use crate::{Backend, ConfigData, LastSaveData};

impl Backend {
    pub async fn load_all_files(&self) {
        //TODO: Load configs and such, if i ever choose to do that
        let token = Self::generate_progress_token("LoadingFiles");
        let update = tower_lsp::lsp_types::ProgressParams {
            token: token.clone(),
            value: tower_lsp::lsp_types::ProgressParamsValue::WorkDone(
                tower_lsp::lsp_types::WorkDoneProgress::Begin(tower_lsp::lsp_types::WorkDoneProgressBegin {
                    title: "Loading Files".to_string(),
                    cancellable: None,
                    message: Some("Loading files from mods".to_string()),
                    percentage: None,
                }),
            ),
        };
        self.client.send_notification::<Progress>(update).await;

        let (loaded, errors) = self.loader.write().unwrap().load();
        
        self.report_progress(token.clone(), format!("Loaded {} files", loaded.len())).await;

        if !errors.is_empty() {
            for error in errors {
                self.client.log_message(MessageType::ERROR, format!("Error loading file: {:?}", error)).await;
            }
        }
        let iter = loaded.par_iter();
        let (res, errs): (Vec<PreprocessedScript>, Vec<String>) = iter.partition_map(|file| {
            self.preprocessor.preprocessor.preprocess_file(file.clone()).map_or_else(Either::Right, Either::Left)
        });
        self.report_progress(token.clone(), format!("Preprocessed {} files", res.len())).await;
        let res = Arc::new(res);
        for err in errs {
            self.client.log_message(MessageType::ERROR, format!("Error preprocessing file: {}", err)).await;
        }


        self.client.send_notification::<Progress>(tower_lsp::lsp_types::ProgressParams {
            token,
            value: tower_lsp::lsp_types::ProgressParamsValue::WorkDone(
                tower_lsp::lsp_types::WorkDoneProgress::End(tower_lsp::lsp_types::WorkDoneProgressEnd {
                    message: Some(format!("Loaded/preprocessed {} files", loaded.len())),
                }),
            ),
        }).await;

        let clone = res.clone();
        let mut analyser = Analyser::default();
        analyser.push_files(clone);

        let config = LoadConfig::default();//TODO: if i choose to do configs this way, change this
        let last_save = LastSaveData {
            preprocessed: res,
            analyser: std::sync::RwLock::new(analyser),
        };
        let config_data = ConfigData {
            files: loaded,
            last_save,
            ..Default::default()
        };


        let mut configs = self.configs.write().unwrap();

        configs.insert(config, config_data);

    }
}