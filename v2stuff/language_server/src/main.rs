use std::{collections::HashMap, sync::{Arc, RwLock}};

use mod_loader::{fileinfo::LoadedFile, LoadConfig, ModLoader};
use preprocessor::{PreprocessedScript, Preprocessor};
use runner::Analyser;
use tower_lsp::{jsonrpc::Result, lsp_types::{InitializeParams, InitializeResult, InitializedParams, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind}, Client, LanguageServer, LspService, Server};

use crate::runners::parser::ParserStep;

//Why am i even pubbing this? this isnt a library lmao
pub mod files;
pub mod viewpoints;
mod file_edits;
//TODO: This should be its own crate probably
pub mod runners;



/*
    Extension Requirements:
    - some way to call didChangeWatchedFiles on mod json files (Pending, i think server can register this itself?)
    - Either root-uri or workspace folders are used to load mods
    - Alternatively, SquirrelLSP.Workspaces can be used 
    - SquirrelLSP.VanillaFiles is almost equivalent to .Workspaces, with the guarantee that they are run first
    If i am forgetting something, tell me because i probably am
*/


#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
#[derive(Debug, Default, PartialEq, Hash)]
pub struct LSPConfig {
    load_config: LoadConfig,
}
impl LSPConfig {
    pub fn build_analyser(&self) -> Analyser {
        //Eventually, this will apply various config stuff here
        let mut analyser = Analyser::default();
        analyser.insert_step(ParserStep{}, runner::step::PreserveType::Unchanged);
        analyser
    }
}
impl Eq for LSPConfig{}

#[derive(Debug, Default)]
pub struct ConfigData {
    pub files: Vec<LoadedFile>,
    pub last_edit: Option<LastEditData>,
    pub last_save: LastSaveData,
}

#[derive(Debug, Default)]
pub struct LastEditData {
    pub preprocessed: Vec<PreprocessedScript>
}
//TODO: This is some boilerplate for when i feel like config system and moving some stuff to on edit, i should probably do that.
#[derive(Debug, Default)]
pub struct LastSaveData {
    pub analyser: RwLock<Analyser>,
    pub preprocessed: Arc<Vec<PreprocessedScript>>,
}

#[derive(Debug, Default)]
pub struct PreprocessorData{
    pub preprocessor: Preprocessor
}
/* 
impl Default for PreprocessorData {
    fn default() -> Self {
        Self {
            preprocessor: Preprocessor::default(),
        }
    }
}*/

#[derive(Debug)]
struct Backend {
    client: Client,
    loader: RwLock<ModLoader>,
    preprocessor: PreprocessorData,
    configs: RwLock<HashMap<LSPConfig, ConfigData>>
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self { 
            client, 
            loader: RwLock::new(ModLoader::default()), 
            preprocessor: PreprocessorData::default(),
            configs: RwLock::new(HashMap::new())
        }
    }
}


#[tower_lsp::async_trait]
impl LanguageServer for Backend{
    async fn initialized(&self, _: InitializedParams) {
        self.register_workspaces().await;
        self.watch_mods().await;
        self.load_all_files().await;
    }
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {

        Ok(InitializeResult{
            capabilities: ServerCapabilities{
                
                //hover_provider: Some(HoverProviderCapability::Simple(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                //declaration_provider: Some(DeclarationCapability::Simple(true)),
                //references_provider: Some(OneOf::Left(true)),
                //definition_provider: Some(OneOf::Left(true)),
                //inlay_hint_provider: Some(OneOf::Left(false)),
                ..Default::default()
            },
            ..Default::default()
        })
    }
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }


    /// We only watch json/rson files so this only triggers on those (and they kind of matter)
    async fn did_change_watched_files(&self, params: tower_lsp::lsp_types::DidChangeWatchedFilesParams) {
        let changed = params.changes;
        for changed in &changed {
            self.client.log_message(tower_lsp::lsp_types::MessageType::INFO, format!("File changed: {}", changed.uri)).await;
        }
        if changed.is_empty() {
            return;
        }
        //TODO: I Should try update in place instead of throwing away EVERYTHING
        //let mut loader = self.loader.write().expect("Failed to acquire write lock on mod loader :(");
        let newloader = ModLoader::new();
        *self.loader.write().unwrap() = newloader;
        self.register_workspaces().await;
        self.load_all_files().await;
    }
}
impl Backend {
    pub fn generate_progress_token(name: &str) -> tower_lsp::lsp_types::NumberOrString {
        let now = std::time::SystemTime::now();
        let timestamp = now.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        tower_lsp::lsp_types::NumberOrString::String(format!("{}-{}", name, timestamp))
    }

    pub async fn report_progress(&self, token: tower_lsp::lsp_types::NumberOrString, message: String) {
        let update = tower_lsp::lsp_types::ProgressParams {
            token,
            value: tower_lsp::lsp_types::ProgressParamsValue::WorkDone(
                tower_lsp::lsp_types::WorkDoneProgress::Report(tower_lsp::lsp_types::WorkDoneProgressReport {
                    message: Some(message),
                    ..Default::default()
                }),
            ),
        };
        self.client.send_notification::<tower_lsp::lsp_types::notification::Progress>(update).await;
    }
}