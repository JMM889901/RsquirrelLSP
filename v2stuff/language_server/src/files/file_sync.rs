use std::sync::Arc;

use mod_loader::LoadConfig;
use preprocessor::{analyser, ASTTokenIndexIter, ASTTokenIter, PreprocessedScript};
use rayon::iter::{Either, IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use runner::Analyser;
use sqparse::{parse, parse_iter};
use tower_lsp::lsp_types::{notification::Progress, MessageType};

use crate::{Backend, ConfigData, LSPConfig, LastSaveData};

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

        let res = Arc::new(res);

        let config = LSPConfig::default();//TODO: if i choose to do configs this way, change this
        let mut analyser = config.build_analyser(); 
        let analysis_start = std::time::Instant::now();
        analyser.push_files(res.clone());
        analyser.run_steps();
        let analysis_duration = analysis_start.elapsed();
        self.client.log_message(
            MessageType::INFO,
            format!("Analyzed {} files in {:?}", res.len(), analysis_duration)
        ).await;

        let last_save = LastSaveData {
            preprocessed: res.clone(),
            analyser: std::sync::RwLock::new(analyser),
        };
        let config_data = ConfigData {
            files: loaded,
            last_save,
            ..Default::default()
        };



        // TEMP
        /* 
        let parse_start = std::time::Instant::now();
        let iter = res.par_iter();
        let map = iter.map(|script| {
            let mut parsed = Vec::new();
            for variant in &script.variants {
                //let mut dedup = combined.clone();
                //dedup.dedup();
                //assert!(dedup.len() == combined.len());
                //let tokeniter = ASTTokenIndexIter::from(&combined, script);
                let tokeniter = ASTTokenIndexIter::from_variant(variant, script);
                //let tokeniter = ASTTokenIter::from(variant, script);
                match tokeniter {
                    Some(tokeniter) => {
                        let parse = parse_iter(tokeniter);
                        match parse {
                            Ok(program) => parsed.push(program),
                            Err(err) => {
                                eprintln!("Error parsing script {}: {:?}", script.path().display(), err);
                            }
                        }
                    }
                    None => {
                        eprintln!("Error creating token iterator for script {}", script.path().display());
                    }
                }
            }
            parsed
        }).collect::<Vec<_>>();
        let parse_duration = parse_start.elapsed();
        //println!("Parsed {} scripts", map.len());
        self.client.log_message(
            MessageType::INFO,
            format!("Parsed {} scripts in {:?}", map.len(), parse_duration)
        ).await;
        // TEMP 
        */

        let mut configs = self.configs.write().unwrap();

        configs.insert(config, config_data);

    }
}


