use preprocessor::Preprocessor;
use tower_lsp::{jsonrpc::Result, lsp_types::{DeclarationCapability, InitializeParams, InitializeResult, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind}, Client, LanguageServer, LspService, Server};

pub mod sync;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
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
    preprocessor: PreprocessorData,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self { client, preprocessor: PreprocessorData::default() }
    }
}


#[tower_lsp::async_trait]
impl LanguageServer for Backend{
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
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
}