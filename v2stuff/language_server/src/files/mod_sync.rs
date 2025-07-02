use std::path::PathBuf;

use mod_loader::mods::FileSource;
use rayon::vec;
use serde::Deserialize;
use serde_json::Value;
use tower_lsp::lsp_types::{notification::Progress, ConfigurationItem, DidChangeWatchedFilesRegistrationOptions, FileSystemWatcher, GlobPattern, MessageType, ProgressParams, ProgressParamsValue, Registration, RelativePattern, Url, WorkDoneProgress, WorkDoneProgressBegin, WorkDoneProgressEnd};

use crate::Backend;


///TODO: This is overcomplicated, just because i required all "mods" to have an associated json, i should just refactor that away and remove all these pointless enums 
#[derive(Deserialize, Debug, Clone)]
enum WorkspaceSchema {
    Local(String),
    Git {
        url: String,
        branch: Option<String>,
    },
    Thunderstore {
        url: String,
        package: String,
        version: Option<String>,
    },
}

enum SqWorkspace {
    Mod(PathBuf),
    Vanilla(PathBuf, Vec<PathBuf>),
}
impl SqWorkspace {
    pub fn exists(&self) -> bool {
        match self {
            SqWorkspace::Mod(path) => path.exists(),
            SqWorkspace::Vanilla(path, _) => path.exists(),
        }
    }
    pub fn main_path(&self) -> PathBuf {
        match self {
            SqWorkspace::Mod(path) => path.clone(),
            SqWorkspace::Vanilla(path, _) => path.clone(),
        }
    }
    pub fn paths_to_watch(&self) -> Vec<PathBuf> {
        match self {
            SqWorkspace::Mod(path) => vec![path.clone()],
            SqWorkspace::Vanilla(path, extra) => {
                let mut paths = extra.clone();
                paths.push(path.clone());
                paths
            }
        }
    }
}

impl Backend {
    async fn to_workspace(
        &self,
        workspace: &WorkspaceSchema,
    ) -> Result<SqWorkspace, String> {
        match workspace {
            WorkspaceSchema::Local(path) => {
                let path = PathBuf::from(path);
                if path.exists() {
                    Ok(SqWorkspace::Mod(path))
                } else {
                    Err(format!("Local workspace path does not exist: {}", path.display()))
                }
            }
            _ => todo!()
        }
    }
    /// Pull mods from configuration and active workspace
    async fn load_workspaces(&self) -> Vec<SqWorkspace> {
        let mut workspaces = Vec::new();

        let config = self.client.configuration(vec![
            ConfigurationItem {
                scope_uri: None,
                section: Some("SquirrelLSP.VanillaDir".to_string()),
        }, ConfigurationItem {
                scope_uri: None,
                section: Some("SquirrelLSP.VanillaExtraDirs".to_string()),
        }]).await;
        if let Ok(config) = config {
            let value = Value::Array(config);
            let vanilla = serde_json::from_value(value);
            if let Err(e) = vanilla {
                //eprintln!("Failed to parse vanilla workspace from config: {:?}", e);//I really need to decide whether to eprint? I dont think it works 
                self.client.log_message(MessageType::ERROR, format!("Failed to parse vanilla workspace from config: {}", e)).await;
            } else {
                let (vanilla, arr): (String, Vec<String>) = vanilla.unwrap();
                let vanilla_path = PathBuf::from(vanilla);
                let extra_paths = arr.into_iter().map(PathBuf::from).collect();
                let vanilla = SqWorkspace::Vanilla(vanilla_path, extra_paths);
                workspaces.push(vanilla);
            }
        }

        let config = self.client.configuration(vec![
            ConfigurationItem {
                scope_uri: None,
                section: Some("SquirrelLSP.Workspaces".to_string()),
        }]).await;

        if let Ok(config) = config {
            for value in config {
                match serde_json::from_value::<WorkspaceSchema>(value.clone()) {
                    Ok(workspace) => {
                        match self.to_workspace(&workspace).await {
                            Ok(workspace) => workspaces.push(workspace),
                            Err(e) => eprintln!("Failed to parse workspace from config {:?} : {}", value, e),
                        }
                    },
                    Err(e) => eprintln!("Failed to parse workspace from config {:?} : {}", value, e),
                }
            }
        }
        let root = self.client.workspace_folders().await;
        if let Ok(Some(folders)) = root {
            for folder in folders {
                if let Ok(path) = folder.uri.to_file_path() {
                    workspaces.push(SqWorkspace::Mod(path));
                }
            }
        }
        workspaces
    }

    /// Load workspaces into the modloader
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
            let mut write = self.loader.write().expect("Failed to acquire write lock on mod loader :(");

            if workspace.exists() {
                //I should probably lock this earlier but this way i can send progress messages (For an opeartion should take under a second lol)
                match write.register_workspace(&workspace.main_path()) {
                    Err(e) => errs.push(e),
                    Ok(count) => loaded += count,
                }
            } else {
                eprintln!("Workspace {:?} does not exist", workspace.main_path());
            }
            if let SqWorkspace::Vanilla(path, extra) = workspace {
                write.register_additional_rson_roots(&path, &extra);
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

    /// Watch mods for changes, currently all mods are reloaded on change
    pub async fn watch_mods(&self) {
        let mut errs = Vec::new();
        let mut watchers = Vec::new();
        {
        //let read = self.loader.read().expect("Failed to acquire read lock on mod loader :(");
        //for _mod in read.get_mods(){
        for workspace in self.load_workspaces().await {
            for path in workspace.paths_to_watch() {
                let path = path.canonicalize();
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