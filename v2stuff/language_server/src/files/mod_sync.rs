use std::path::PathBuf;

use serde_json::Value;
use tower_lsp::lsp_types::{notification::{Progress}, ConfigurationItem, DidChangeWatchedFilesRegistrationOptions, FileSystemWatcher, GlobPattern, ProgressParams, ProgressParamsValue, Registration, RelativePattern, Url, WorkDoneProgress, WorkDoneProgressBegin, WorkDoneProgressEnd};

use crate::Backend;

impl Backend {
    /// Pull mods from configuration and active workspace
    async fn load_workspaces(&self) -> Vec<PathBuf> {
        let mut workspaces = Vec::new();

        let config = self.client.configuration(vec![
            ConfigurationItem {
                scope_uri: None,
                section: Some("SquirrelLSP.VanillaFiles".to_string()),
            },
            ConfigurationItem {
                scope_uri: None,
                section: Some("SquirrelLSP.Workspaces".to_string()),
            }
            ]
        ).await;
        if let Ok(config) = config {
            for value in config {
                if let Value::Array(arr)  = value {
                    for item in arr {
                        if let Value::String(path) = item {
                            let path = PathBuf::from(path); //todo: canonicalize maybe?
                            workspaces.push(path);
                        }
                    }
                }
            }
        }
        let root = self.client.workspace_folders().await;
        if let Ok(Some(folders)) = root {
            for folder in folders {
                if let Ok(path) = folder.uri.to_file_path() {
                    workspaces.push(path);
                }
            }
        }
        workspaces
    }


    pub async fn register_workspaces(&self) {
        //TODO: Parrelize? Either here or during the actual file searching, becuase thats recursive and expensive
        let workspaces = self.load_workspaces().await;
        let mut errs = Vec::new();
        let mut loaded = 0;
        let token = Self::generate_progress_token("DiscoveringMods");
        let update = ProgressParams {
            token: token.clone(),
            value: ProgressParamsValue::WorkDone(
                WorkDoneProgress::Begin( WorkDoneProgressBegin{
                    title: "Discovering Mods".to_string(),
                    cancellable: None,
                    message: Some("Discovering Mods".to_string()),
                    percentage: None,
                })
            ),
        };

        self.client.send_notification::<Progress>(
            update
        ).await;
            
        for workspace in workspaces {
            if workspace.exists() {
                let mut write = self.loader.write().expect("Failed to acquire write lock on mod loader :(");
                //I should probably lock this earlier but this way i can send progress messages (For an opeartion should take under a second lol)
                match write.register_workspace(&workspace) {
                    Err(e) => errs.push(e),
                    Ok(count) => loaded += count,
                }
            } else {
                eprintln!("Workspace {:?} does not exist", workspace);
            }
        }

        self.client.log_message(
            tower_lsp::lsp_types::MessageType::INFO,
            format!("Loaded {} workspaces", loaded)
        ).await;
        for err in errs {
            self.client.log_message(
                tower_lsp::lsp_types::MessageType::ERROR,
                format!("Error loading workspace: {:?}", err)
            ).await;
        }

        self.client.send_notification::<Progress>(
            ProgressParams {
                token,
                value: ProgressParamsValue::WorkDone(
                    WorkDoneProgress::End( WorkDoneProgressEnd{
                        message: Some(format!("Loaded {} mods", loaded)),
                    })
                ),
            }
        ).await;


    }

    pub async fn watch_mods(&self) {
        let mut errs = Vec::new();
        let mut watchers = Vec::new();
        {
        //let read = self.loader.read().expect("Failed to acquire read lock on mod loader :(");
        //for _mod in read.get_mods(){
        for workspace in self.load_workspaces().await {
            let path = workspace.canonicalize();
            if let Err(e) = path {
                errs.push(e);
                continue;
            }
            let path = Url::from_file_path(path.unwrap()).unwrap();//scary? but canonicalize should already error if this isnt valid
            let watcher = FileSystemWatcher {
                glob_pattern: GlobPattern::Relative(
                    RelativePattern{
                        base_uri: tower_lsp::lsp_types::OneOf::Right(path),
                        pattern: "**/{mod.json,scripts.rson}".to_string(),
                    }
                ),
                kind: None,
            };
            watchers.push(watcher);

        }
        }
        let params = DidChangeWatchedFilesRegistrationOptions {
            watchers,
        };
        let registration = Registration {
            id: "didChangeWatchedFiles".to_string(),
            method: "workspace/didChangeWatchedFiles".to_string(),
            register_options: Some(serde_json::to_value(params).unwrap()),
        };
        if let Err(e) = self.client.register_capability(vec![registration]).await {
            eprintln!("Failed to register didChangeWatchedFiles: {}", e);
        }
    }
}