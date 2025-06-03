use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::format;
use std::fs::read_to_string;
use std::hash::Hash;
use std::path::{self, Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use analysis_common::spanning_search::SpanningSearch;
use analysis_common::variable::{Variable, VariableExternal};
use rayon::iter::IntoParallelIterator;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::request::{GotoDeclarationParams, GotoDeclarationResponse, References};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use rayon::prelude::*;

use ASTAnalyser::load_order::{identify_file_tree, identify_globals, File, FilePreAnalysis, ParseType};
use ASTAnalyser::single_file::{analyse, collect_errs, AnalysisState};
use ASTAnalyser::{analyse_state, find_funcs, LogicError, Scope};
use analysis_common::CompiledState;
use analysis_common::modjson::{load_mod, load_mods, Script};
use common::FileInfo;


#[tokio::main]
async fn main() {
    
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client, initialized: RwLock::new(false), load_order: RwLock::new(Vec::new()), last_run: RwLock::new(Vec::new()), hint_cache: RwLock::new(HashMap::new()) });
    Server::new(stdin, stdout, socket).serve(service).await;
}


#[derive(Debug)]
struct Backend {
    client: Client,
    initialized: RwLock<bool>,
    load_order: RwLock<Vec<FileInfo>>,
    last_run: RwLock<Vec<RunData>>,
    hint_cache: RwLock<HashMap<String, FileNeedsInlayHintPos>>,//This, is stupid
}

#[derive(Debug, Clone)]
pub struct RunData{
    file: Arc<File>,
    outputs: Vec<(Arc<FilePreAnalysis>, Arc<Scope>)>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct FileNeedsInlayHintPos{
    pub pos: Position,
}

impl Backend{

    async fn initialize_load_order(&self, workspace_folders: Option<Vec<WorkspaceFolder>>) -> bool {

        let mut moddata = Vec::new();
        //For now assume we are in the root of a mod, so json is ./mod.json
        match workspace_folders{
            Some(workspace_folders) => {
                for path in workspace_folders{
                    let path = path.uri.to_file_path().unwrap();
                    let mods = load_mods(path.clone());
                    if mods.is_err(){
                        self.client.log_message(MessageType::ERROR, format!("Failed to load mod.json: {:?}", mods.err())).await;
                        return false;
                    }
                    moddata.extend(mods.unwrap());
                    
                }
            },//i dont know if that would even cause it but i cant think of a reason why it would fail here
            None => {
                self.client.log_message(MessageType::ERROR, "No workspace folders found").await;
                let path = PathBuf::new();
                let moddata = load_mods(path.clone()).expect("Failed to load mod.json, are you in a network folder?");
            }
        };
        let scripts = moddata.into_iter().flat_map(|x| x.scripts);

        //Note: ^ This should be reworked to use the loaded files from vscode instead of reading, because apparently
        //VsCode dont like it when you read the file yourself
        //Maybe make a custom file handler or something
        //For now, reset the load order every time
        let mut new_scripts = Vec::new();
        let mut changed = false;
        {
            let old = self.load_order.read().unwrap();
            for (index, new) in scripts.enumerate(){
                let old = old.get(index);
                if old.is_none(){
                    changed = true;
                    new_scripts.push(new.clone());
                } else {
                    let old = old.unwrap();
                    if old.path() != new.path(){
                        changed = true;
                    }
                    new_scripts.push(new);
                }
            }
        }

        if !changed{
            return false;
        }
        self.load_order.write().unwrap().clear();
        self.load_order.write().unwrap().extend(new_scripts);
        return true;
    }
    async fn on_change(&self, url: Url, text: &String) -> bool {
            let folders = self.client.workspace_folders().await.unwrap();
            let update= self.initialize_load_order(folders.clone()).await;
            return self.parse(url, text, update).await;
    }
    async fn parse(&self, url: Url, text: &String, update: bool) -> bool {


        //TODO: This should only reall need doing once at the start and whenever the mod.json changes
        //    *self.initialized.write().unwrap() = true;
        //}
        let path = url.to_file_path().unwrap();

        //Invalidate anything with a lower order:
        //todo lol

        let files = self.load_order.read().unwrap().clone();

        //self.client.log_message(MessageType::INFO, format!("edited file: {:?}", path)).await;

        let mut hit_changed = false;
        let mut unchanged = Vec::new();
        let mut changed = Vec::new();
        
        
        for script in files.iter(){
            //println!("Loading file: {}", script.path);
            //self.client.log_message(MessageType::INFO, format!("Loading file: {:?}", script.path())).await;
            if script.path() == &path{
                //self.client.log_message(MessageType::INFO, format!("direct reading file {:?}", script.path())).await;
                script.purge();
                script.set_text(text.clone());
                hit_changed = true;
                changed.push(script.clone());
                continue;
            } else if hit_changed{
                //script.purge();
                //changed.push(script.clone());
            } else if update{
                script.purge();
                changed.push(script.clone());
                continue;
            } 
            let last = self.get_run_uri(&Url::from_file_path(script.path()).unwrap());
            if let Some(last) = last{
                let mut variants = last.file.variants.get_direct();
                let variants = variants.iter().map(|x| {
                    x.globalinfo.clone()
                }).collect::<Vec<_>>();
                unchanged.push((script.clone(), variants))
            } else {
                changed.push(script.clone());
            }
        }
        //Catchall
        if !hit_changed{
            for script in files.iter(){
                if script.path() == &path{
                    script.purge();
                    for (file, _) in &unchanged{
                        changed.push(file.clone());
                    }
                    unchanged.clear();
                }
            }
        }
        let updated = changed.len();
        //Should CFG this
        let time_preprocess_start = std::time::Instant::now();
        let time_parse_start = std::time::Instant::now();
        let preprocessed = identify_globals(changed);
        let time_parse_end = std::time::Instant::now();
        unchanged.extend(preprocessed);
        
        let mut tree:Vec<Arc<File>> = Vec::new();
        {//Funky thread shenanigans


            tree = identify_file_tree(unchanged);
        }
        let time_preprocess_end = std::time::Instant::now();
        //self.client.log_message(MessageType::INFO, format!("starting analysis on {:?}", tree.iter().map(|x| x.load.path()))).await;

        let time_analysis_start = std::time::Instant::now();
        let iter = tree.into_par_iter();
        let this_run: Vec<(Url, RunData)> = iter.filter_map(|file| {
            let is_this_file = file.load.path() == &path;
            //self.client.log_message(MessageType::INFO, format!("offsets are {:?}", file.load.offsets())).await;

            if file.parse_type == ParseType::PreAnalysis{
                return None;
            }
            let mut diagnostics = Vec::new();
            let mut this_file = Vec::new();

            for variant in file.variants.get_direct(){
                //self.client.log_message(MessageType::INFO, format!("Analysing file: {:?} variant: {:?}", file.load.name(), variant.primitive.context)).await;
                let scope = analyse_state(variant.clone());
                for err in collect_errs(scope.clone()){
                    //let message = format!("{:?} at \n {} \n", err, file.load.text()[err.range.0 .. err.range.1].to_string());
                    let message = format!("{}", err.value.render(variant));
                    let range = Range{
                        start: Self::offset_to_linecol(&file.load, err.range.0),
                        end: Self::offset_to_linecol(&file.load, err.range.1)
                    };
                    let mut diag = Diagnostic::new_simple(range, message);
                    if matches!(err.value.as_ref(), LogicError::SyntaxWarning(_)) {
                        diag.severity = Some(DiagnosticSeverity::WARNING);
                    }
                    diagnostics.push(diag);
                }
                this_file.push((variant.clone(), scope.clone()));
            }
            let rundata = RunData{
                file: file.clone(),
                outputs: this_file,
                diagnostics: diagnostics
            };
            let mut newurl;
            if is_this_file{
                newurl = url.clone();//This is to ensure query stuff stays
            } else {
                newurl = Url::from_file_path(file.load.path()).unwrap();
            }
            //self.client.publish_diagnostics(newurl, diagnostics, None).await;
            return Some((newurl, rundata));
        }).collect();
        let time_analysis_end = std::time::Instant::now();
        self.last_run.write().unwrap().clear();
        for (url, rundata) in this_run{
            self.client.publish_diagnostics(url, rundata.diagnostics.clone(), None).await;
            self.last_run.write().unwrap().push(rundata);
        }
        let message = format!("Finished analysing {} files, took {:?}ms\n\t Parsing: {:?}\n\t Preproc: {:?}\n\t Analysis: {:?}",
            updated,
            time_analysis_end.duration_since(time_preprocess_start).as_millis(),
            time_parse_end.duration_since(time_parse_start).as_millis(),
            time_preprocess_end.duration_since(time_preprocess_start).as_millis(),
            time_analysis_end.duration_since(time_analysis_start).as_millis()
        );
        self.client.log_message(MessageType::INFO, message ).await;
        return hit_changed;
    }

    pub fn get_run_uri(&self, path: &Url) -> Option<RunData>{
        let lastrun = self.last_run.read().unwrap();
        if lastrun.is_empty(){
            return None;
        }
        let run = lastrun.iter().find(|x| x.file.load.path() == &path.to_file_path().unwrap());
        return run.map(|x| x.clone());
    }

    pub fn offset_to_linecol(file: &FileInfo, offset: usize) -> Position{
        let (line, col) = file.offset_to_linecol(offset);
        Position{
            line: line as u32,
            character: col as u32
        }
    }

    pub fn linecol_to_offset(file: &FileInfo, line: usize, col: usize) -> usize{
        let offset = file.linecol_to_offset(line, col);
        return offset;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.client
        .log_message(MessageType::INFO, "server initializing!")
        .await;

        Ok(InitializeResult{
            capabilities: ServerCapabilities{
                //hover_provider: Some(HoverProviderCapability::Simple(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                declaration_provider: Some(DeclarationCapability::Simple(true)),
                references_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                inlay_hint_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        //Log the uris query for testing}
        let in_scope = self.on_change(params.text_document.uri, &params.text_document.text).await;
        if !in_scope{
            self.client.show_message(MessageType::INFO, "This file is not currently in the load order, it will not be analysed").await;
        }
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        //TODO: I can do a lot more cool stuff with the condition being tracked when opening the file, but i do not have the time :(
        //self.client.log_message(MessageType::INFO, format!("Inlay hint request for: {:?}", params.text_document.uri)).await;
        if let Some(text) = params.text_document.uri.query(){
            //String::from_utf8(text.as_bytes().to_vec()).unwrap();
            self.client.log_message(MessageType::INFO, format!("Opened file with query: {}", text )).await;
            let mut line = params.range.start.line;
            let mut character = params.range.start.character;

            if text.contains("condition%3D"){
                let args = text.split("condition%3D");
                //let pos = args.clone().nth(0).unwrap().replace("lc%3D", "").split("%3A").map(|x| x.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
                //let (mut line, character) = (pos[0], pos[1]);
                //if line > 0 {
                //    line -= 1;
                //}


                let condition = args.last().unwrap_or(&"").replace("%7B", "").replace("%7D", "").replace("%26", "&").replace("%21", "!");

                {
                    let cache = self.hint_cache.read().unwrap();
                    if let Some(pos) = cache.get(&condition){
                        line = pos.pos.line;
                        character = pos.pos.character;
                    } else {
                        //Cant log anything here because borrow checker says no
                    }
                    drop(cache);
                }
                if line > 0 {
                    line -= 1;
                }
                //Todo: Get an actual decoder for this, this is a bit jank
                let hint_text = format!("Condition: {}", condition);

                self.client.log_message(MessageType::INFO, format!("Inlay hint: {}", hint_text)).await;

                let hint = InlayHint{
                    position: Position{line, character},
                    label: InlayHintLabel::String(hint_text),
                    kind: Some(InlayHintKind::PARAMETER),
                    padding_left: Some(false),
                    padding_right: Some(false),
                    text_edits: None,
                    tooltip: Some(InlayHintTooltip::String("The preprocessor conditions used to generate the referenced file".to_string())),
                    data: None
                };
                return Ok(Some(vec![hint]));
            }
        }
        return Ok(None);
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>>{
        let pos = params.text_document_position.position;
        let uri = params.text_document_position.text_document.uri;
        let run = self.get_run_uri(&uri);
        if run.is_none(){
            panic!("No run found for uri: {:?}", uri); //Panicking is bad but its for debugging,
            //If you call find references after being told that this ISNT a valid file, cope i guess
            return Ok(None);
        }
        let run = run.unwrap();
        let file = run.file.clone();
        let offset = file.load.offsets()[(pos.line) as usize];
        let offset = offset + pos.character as usize;
        let outputs = run.outputs.clone();
        let mut refs = Vec::new();
        for (state, scope) in outputs{
            refs.extend(scope.find_uses(offset).into_iter().map(|x| (state.clone(), x)));
        };
        if refs.is_empty(){
            //panic!("offset: {} references: {:?}", offset, run.outputs.iter().map(|x| x.1.debuginfo()).collect::<Vec<_>>());
            return Ok(None);
        }
        self.hint_cache.write().unwrap().clear();//TODO: Janky

        let links = refs.into_iter().map(|(state, x)|{
            let target_file = x.source_run.file.clone();
            let mut url = Url::from_file_path(target_file.path()).unwrap();
            let callcontext = x.source_run.context.clone();
            let linecol = Self::offset_to_linecol(&target_file, x.source.range.0);
            let target = x.target.clone();
            if let Variable::Global(var) = target.as_ref(){
                url.set_query(Some(&format!("condition={{{}}}", callcontext.string_out_simple())));
            }//Technically i should be able to only show this in specific cases but thats a lot of logic i dont want to write right now
            let mut write = self.hint_cache.write().unwrap();//Should this store the context? it does when getting decl
            write.insert(callcontext.string_out_simple(), FileNeedsInlayHintPos{
                pos: linecol
            });
            drop(write);
            let link = Location{
                uri: url,
                range: Range { start: linecol, end: Self::offset_to_linecol(&target_file, x.source.range.1) },
            };
            link
        });
        let links = links.collect::<Vec<Location>>();
        return Ok(Some(links));

    }


    /*async fn hover(&self, _params: HoverParams) -> Result<Option<Hover>> {
        let loc = _params.text_document_position_params;
        let position = loc.position;
        
        let uri = loc.text_document.uri;
        let run = self.get_run_uri(&uri);
        if run.is_none(){
            return Ok(None);
        }
        let run = run.unwrap();
        let file = run.file.clone();
        let offset = file.load.offsets()[(position.line) as usize];
        let offset = offset + position.character as usize;

        let outputs = run.outputs.clone();
        let mut decls = Vec::new();
        let mut refs = Vec::new();
        for (state, scope) in outputs{
            decls.extend(scope.find_declaration(offset));
            refs.extend(scope.find_uses(offset));
        }
        if refs.is_empty() && decls.is_empty(){
            //panic!("offset: {} references: {:?}", offset, run.outputs.iter().map(|x| x.1.debuginfo()).collect::<Vec<_>>());
            return Ok(None);
        }
        //todo: Merge duplicates and make this pretty
        //Externals can provide descriptions and such so we should provide those
        let decls = decls.into_iter().map(|x|{
            let mut text = "".to_string();
            let target = x.target.clone();
            text = format!("referenced by: {:?}:\n\n references: {:?},\n\n from:{:?}\n\n", x.source, target.ast().text_none_rec(), x.source_run.file.path());
            MarkedString::String(format!("\n{}", text))//Make this pretty
        });
        let refs = refs.into_iter().map(|x|{
            let mut text = "".to_string();
            let target = x.target.clone();
            text = format!("referenced by: {:?}:\n\n references: {:?},\n\n from:{:?}\n\n", x.source, target.ast().text_none_rec(), x.source_run.file.path());
            MarkedString::String(format!("\n{}", text))//Make this pretty
        });
        let divider = MarkedString::String("REFS:".to_string());
        let decls = decls.chain(std::iter::once(divider));
        let both = decls.chain(refs).collect::<Vec<MarkedString>>();
        let contents = HoverContents::Array(both);
        return Ok(Some(Hover{
            contents,
            range: Some(Range{
                start: Position{line: position.line, character: position.character},
                end: Position{line: position.line, character: position.character}
            })
        }));
    }*/

    async fn goto_declaration(&self, params: GotoDeclarationParams) -> Result<Option<GotoDeclarationResponse>> {
        self.goto_definition(params).await
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let pos = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let run = self.get_run_uri(&uri);
        if run.is_none(){
            return Ok(None);
        }
        let run = run.unwrap();
        let file = run.file.clone();
        let offset = file.load.offsets()[(pos.line) as usize];
        let offset = offset + pos.character as usize;
        let outputs = run.outputs.clone();
        let mut decls = Vec::new();
        for (state, scope) in outputs{
            decls.extend(scope.find_declaration(offset).into_iter().map(|x| (state.clone(), x)));
        };
        if decls.is_empty(){
            return Ok(None);
        }
        //TODO: Merge same file/ast references
        self.hint_cache.write().unwrap().clear();//TODO: Janky

        //TODO: Make this pretty
        let multi = decls.len() > 1;
        let decls = decls.into_iter().map(|(state, x )|{
            let target = x.target.clone();
            let target_file = target.file().unwrap_or(file.load.clone());
            let mut url = Url::from_file_path(target_file.path()).unwrap();
            let linecol = Self::offset_to_linecol(&target_file, target.get_range_precise().0);
            if multi{
                let mut context_str;
                if let Variable::Global(var) = target.as_ref(){
                    context_str = target.try_get_context().string_out_simple();
                } else{
                    context_str = state.globalinfo.primitive.context.string_out_simple();
                }
                url.set_query(Some(&format!("condition={{{}}}", context_str)));
                let mut write = self.hint_cache.write().unwrap();
                write.insert(context_str, FileNeedsInlayHintPos{
                    pos: linecol
                });
                drop(write);
            }

            let link = LocationLink{
                origin_selection_range: None,
                target_uri: url,
                target_range: Range { start: Self::offset_to_linecol(&target_file, target.get_range_precise().0), end: Self::offset_to_linecol(&target_file, target.get_range_precise().1) },
                target_selection_range: Range { start: Self::offset_to_linecol(&target_file, target.get_range_precise().0), end: Self::offset_to_linecol(&target_file, target.get_range_precise().1) },
                //What is the difference?
            };
            link
        });
        let decls = decls.collect::<Vec<LocationLink>>();
        let response = GotoDeclarationResponse::Link(decls);
        return Ok(Some(response));
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.on_change(params.text_document.uri, &params.content_changes[0].text).await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())//Todo: Maybe consider outputting a json or something for vanilla files, since those dont really change often
        //< dont, this is also called on reload and chances are if im reloading this i dont WANT to keep anything previous because its broken
    }
}


#[cfg(test)]
#[tokio::test]
async fn test_northstar() {
    use std::env::current_dir;

    let base = current_dir().unwrap();

    let path = "../../NorthstarMods-1.30.0/";
    let base = base.join(path);
    let url = Url::from_file_path(base).unwrap();
    let workspace_folders = vec![WorkspaceFolder{
        uri: url.clone(),
        name: "Northstar".to_string()
    }];
    println!("Base: {:?}", url);
    let (service, _) = LspService::new(|client| Backend { client, initialized: RwLock::new(false), load_order: RwLock::new(Vec::new()), last_run: RwLock::new(Vec::new()), hint_cache: RwLock::new(HashMap::new()) });
    service.inner().initialize_load_order(Some(workspace_folders)).await;
    
    service.inner().parse(url, &"".to_string(), true).await;
    assert!(service.inner().last_run.read().unwrap().len() > 0, "No runs found for northstar");
}


#[cfg(test)]
#[tokio::test]
async fn get_declaration() {
    use std::env::current_dir;

    let base = current_dir().unwrap();
    let path: &str = "../TestSets/ValidStructure/8Global/";
    let base = base.join(path);
    let url = Url::from_file_path(base).unwrap();
    let workspace_folders = vec![WorkspaceFolder{
        uri: url.clone(),
        name: "Northstar".to_string()
    }];
    println!("Base: {:?}", url);
    let (service, _) = LspService::new(|client| Backend { client, initialized: RwLock::new(false), load_order: RwLock::new(Vec::new()), last_run: RwLock::new(Vec::new()), hint_cache: RwLock::new(HashMap::new()) });
    service.inner().initialize_load_order(Some(workspace_folders)).await;
    println!("{:?}", service.inner().load_order.read().unwrap());
    service.inner().parse(url.clone(), &"".to_string(), true).await;
    let file = "8Global/mod/scripts/vscripts/Calls.gnut";
    let file = url.join(file).unwrap();
    println!("File: {:?}", file);
    //println!("RunData: {:?}", service.inner().last_run.read().unwrap());

    let params = GotoDeclarationParams{
        text_document_position_params: TextDocumentPositionParams{
            text_document: TextDocumentIdentifier{
                uri: file.clone()
            },
            position: Position{
                line: 2,
                character: 9
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };
    let result = service.inner().goto_declaration(params).await.unwrap();
    assert!(result.is_some(), "No result found for goto declaration");
    println!("Result: {:?}", result);
    let result = result.unwrap();
    if let GotoDefinitionResponse::Link(links) = result{
        for link in &links{
            println!("Link: {:?}", link);
        }
        assert!(links.len() == 9, "Insufficient links found for goto declaration");
    } else {
        panic!("No links found for goto declaration")
    }

}


#[cfg(test)]
#[tokio::test]
async fn get_uses() {
    use std::env::current_dir;

    let base = current_dir().unwrap();
    let path: &str = "../TestSets/ValidStructure/8Global/";
    let base = base.join(path);
    let url = Url::from_file_path(base).unwrap();
    let workspace_folders = vec![WorkspaceFolder{
        uri: url.clone(),
        name: "Northstar".to_string()
    }];
    println!("Base: {:?}", url);
    let (service, _) = LspService::new(|client| Backend { client, initialized: RwLock::new(false), load_order: RwLock::new(Vec::new()), last_run: RwLock::new(Vec::new()), hint_cache: RwLock::new(HashMap::new()) });
    service.inner().initialize_load_order(Some(workspace_folders)).await;
    println!("{:?}", service.inner().load_order.read().unwrap());
    service.inner().parse(url.clone(), &"".to_string(), true).await;
    let file = "8Global/mod/scripts/vscripts/Defines8.gnut";
    let file = url.join(file).unwrap();
    println!("File: {:?}", file);
    //println!("RunData: {:?}", service.inner().last_run.read().unwrap());

    let params = ReferenceParams{
        text_document_position: TextDocumentPositionParams{
            text_document: TextDocumentIdentifier{
                uri: file.clone()
            },
            position: Position{
                line: 0,
                character: 9
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: ReferenceContext{
            include_declaration: false
        },
    };
    let result = service.inner().references(params).await.unwrap();
    assert!(result.is_some(), "No result found for references");
    println!("Result: {:?}", result);
    if let Some(links)= result{
        for link in &links{
            println!("Link: {:?}", link);
        }
        assert!(links.len() == 1, "Insufficient links found for references");
    } else {
        panic!("No links found for references")
    }

}