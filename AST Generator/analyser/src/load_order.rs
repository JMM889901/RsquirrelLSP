use std::{collections::HashMap, fmt::Debug, hash::Hash, path::{self, PathBuf}, primitive, sync::{Arc, RwLock}};

use analysis_common::{spanning_search::TraversableMap, variable::{Variable, VariableSearch}, CompiledState, HasState, RunPrimitiveInfo};
use common::{FileInfo, FileType};
use ASTParser::{ast::{Element, ExternalType, AST}, error::Error, external_resources::{parse_externalfuncs, parse_nativefuncs, ExternalResource, ExternalResourceEntry, ExternalResourceType}, Global};
use ConfigAnalyser::{force_get_states_statement, get_file_varaints};
use PreprocessorParser::parse_condition_expression;
use TokenIdentifier::{get_globals, Globals};
use rayon::{prelude::*, vec};


use crate::{generate_asts};
#[derive(Debug)]
pub struct RunGlobalInfo{
    pub globals: Globals,
    pub primitive: Arc<RunPrimitiveInfo>,//Mostly to let me easily delete usage data
}
impl RunGlobalInfo{
    pub fn debugblank() -> Self{
        Self{
            globals: Globals{
                globals: TraversableMap::new(),
            },
            primitive: Arc::new(RunPrimitiveInfo{
                file: FileInfo::new(String::new(), PathBuf::new(), String::new(), FileType::External),
                context: CompiledState::from(HashMap::new()),
                id: 0,
                ast: Vec::new(),
            }),
        }
    }
}
impl HasState for RunGlobalInfo{
    fn get_state(&self) -> CompiledState {
        self.primitive.get_state()
    }
}
#[derive(Debug)]
pub struct FilePreAnalysis{//These will be accessed across threads
    //This is effectively the file without regards to any other files
    //pub context: CompiledState,
    //pub ast: Vec<Element<AST>>,
    pub dependencies: Vec<Dependency<RunGlobalInfo>>,
    pub untyped: bool,
    pub globalinfo: Arc<RunGlobalInfo>,
}
impl HasState for FilePreAnalysis{
    fn get_state(&self) -> CompiledState {
        self.globalinfo.get_state()
    }
}
impl FilePreAnalysis{
    pub fn debugblank() -> Arc<Self>{
        Arc::new(Self{
            //context: HashMap::new(),
            //globals: Globals{
            //    globals: TraversableMap::new(),
            //},
            //ast: Vec::new(),
            dependencies: Vec::new(),
            untyped: false,
            globalinfo: Arc::new(RunGlobalInfo::debugblank()),
            //primitive: Arc::new(RunPrimitiveInfo{
            //    file: FileInfo::new(String::new(), PathBuf::new(), String::new(), FileType::External),
            //    context: CompiledState::from(HashMap::new()),
            //    id: 0,
            //    ast: Vec::new(),
            //}),
        })
    }
    pub fn get_global_internal(&self, name: String) -> VariableSearch{

        //This entire function can be simplified now that everything is a Variable and i dont have 50 billion intermidiary types

        //This should probably be recursive but thats surprisingly difficult given the dependency architecture
        //pain
        if let Some(global) = self.globalinfo.globals.get(&name){
            return VariableSearch::Single(global.clone());
        }
        let mut working = Vec::new();
        for dep in &self.dependencies{
            match dep{
                Dependency::Single(file) => {
                    if let Some(global) = file.globals.get(&name){
                        return VariableSearch::Single(global.clone());
                    }
                }, //Technically this could result in instances where an always defined variable and a sometimes one conflict, but that isnt allowed anyway so it should only happen with code thats rejected for other reasons
                Dependency::Multi(files) => {
                    //Now this is where things get problematic
                    //The easiest case here is all variants have it defined so
                    for file in files{
                        if let Some(global) = file.globals.get(&name){
                            working.push(global);
                        }
                    }
                    if working.len() >0 && self.get_state().will_i_always_accept_one_of(&working.iter().map(|x| x.try_get_context()).collect::<Vec<_>>()){
                        return VariableSearch::Multi(working);
                    }
                }
                Dependency::Possible(files) => {
                    //This is a strange case, but i think as long as we ensure that another variant is true (like with standard multi) it should be ok
                    //i think

                    for file in files{
                        if let Some(global) = file.globals.get(&name){
                            working.push(global);
                        }
                    }
                    if working.len() >0 && self.get_state().will_i_always_accept_one_of(&working.iter().map(|x| x.try_get_context()).collect::<Vec<_>>()){
                        return VariableSearch::Multi(working);
                    }
                }
            }
        }
        if working.len() == 0{
            return VariableSearch::None;
        }else{
            return VariableSearch::MultiIncomplete(working);
        }
    }

}


#[derive(Debug, Clone)]
pub enum Dependency<T>{
    Multi(Vec<Arc<T>>),//Bad but inevitable
    Single(Arc<T>),
    Possible(Vec<Arc<T>>),//This is for later error handling, we cannot confirm this will run
    //More accurately, we can confirm it will NOT run in some cases
}//I dont fully remember what the CompiledState is here but i think its explicitly the additional conditions on top of the ones provided when this was made... maybe
impl <T> Dependency<T>{
    //pub fn construct(file: Option<Arc<File>>, context: &CompiledState) -> Vec<Dependency<FilePreAnalysis>>{
    //    let mut parent = file;
    //    let mut vec = Vec::new();
    //    while let Some(file) = parent{
    //        if let Some(dep) = file.variants.get(context){
    //            vec.push(dep);
    //        }
    //        parent = file.parent.clone();
    //    }
    //    vec
    //}
    pub fn construct_all(data: &Vec<(FileInfo, ParsedData)>, context: &CompiledState) -> Vec<Dependency<RunGlobalInfo>>{
        let mut vec = Vec::new();
        //Expensive
        for (file, parsed) in data{
            let filter = ContextFilter{
                content: parsed.iter().map(|x| x.clone()).collect(),
            };
            let filter = filter.get(context);
            if let Some(dep) = filter{
                vec.push(dep);
            }
        }
        vec
    }
}
impl <T> Dependency<T> where T: HasState{
    pub fn map(self, f: impl Fn(Arc<T>) -> Arc<T>) -> Self{
        match self{
            Dependency::Single(file) => Dependency::Single(f(file)),
            Dependency::Multi(files) => Dependency::Multi(files.into_iter().map(|x| f(x)).collect()),
            Dependency::Possible(files) => Dependency::Possible(files.into_iter().map(|x| f(x)).collect()),
        }
    }
}



//Ok so for future reference
//A context in this is a requiremnt to accept, thus anything indexing this must be a superset of the element it accepts
//IE SERVER && MP cannot be indexed by SERVER but can by SERVER && MP && MP_BOX
//pub struct ContextFilter<F, E>{
//    //Initially was going to do fancy optimization but later
//    content: Vec<(F, E)>,
//}
//impl <F, E>ContextFilter<F, E>{
//    pub fn get(&self, provided: &F) -> Option<&E>{
//        for (context, element) in &self.content{
//            if context == provided{
//                return Some(element);
//            }
//        }
//        None
//    }
//}

pub struct ContextFilter<T: HasState>{//Todo: Genric this for testing
    content: Vec<Arc<T>>,
}
impl <T: HasState> ContextFilter<T>{
    pub fn get(&self, provided: &CompiledState) -> Option<Dependency<T>>{
        //Apply increasing filters to the context
        let content = self.content.clone();
        let filter = content.iter().filter(|(file)| {

            //If this context explicitly contradicts the provided context then skip it
            !provided.do_i_reject_explicit(&file.get_state())

        }).map(|x| x.clone()).collect::<Vec<_>>();

        //if will_allways_pass(provided, &filter.iter().map(|x| x.0.clone()).collect()){
        if provided.will_i_always_accept_one_of(&filter.iter().map(|x| x.get_state()).collect::<Vec<_>>()){
            //If we have a match then return the first one
            if filter.len() == 1{
                return Some(Dependency::Single(filter[0].clone()));
            }else if filter.len() > 1{
                //If we have multiple matches then return a multi dependency
                return Some(Dependency::Multi(filter));
            }
        }
        if filter.len() == 0{
            return None;
        }else{ //If we have multiple matches then return a multi dependency
            return Some(Dependency::Possible(filter));
        }
    }
    pub fn get_direct(&self) -> &Vec<Arc<T>>{
        return &self.content;//Should really replace this with an iterator but whatever
    }
}

//#[test]
//fn text_context_filter_subset(){
//    let server_and_mp = HashMap::from([("SERVER".to_string(), true), ("MP".to_string(), true)]);
//    let server_and_not_mp = HashMap::from([("SERVER".to_string(), true), ("MP".to_string(), false)]);
//    let server = HashMap::from([("SERVER".to_string(), true)]);
//
//    assert!(will_allways_pass(&server, &vec![server_and_mp, server_and_not_mp]));
//}
//
//#[test]
//fn text_context_filter_failed_subset(){
//    let server_and_mp = HashMap::from([("SERVER".to_string(), true), ("MP".to_string(), true)]);
//    let server_and_not_mp = HashMap::from([("SERVER".to_string(), true), ("MP".to_string(), false)]);
//    let server_and_mp_box = HashMap::from([("SERVER".to_string(), true), ("MP".to_string(), true), ("MP_BOX".to_string(), true)]);
//    let server = HashMap::from([("SERVER".to_string(), true)]);
//
//    assert!(!will_allways_pass(&server, &vec![server_and_mp, server_and_not_mp, server_and_mp_box]));
//}
//
//#[test]
//fn text_context_filter_no_required(){
//    let always_run = HashMap::new();
//    let server = HashMap::from([("SERVER".to_string(), true)]);
//
//    assert!(will_allways_pass(&server, &vec![always_run]));
//}

pub struct File{
    pub parse_type: ParseType,
    //pub parent: Option<Arc<File>>,
    pub variants: ContextFilter<FilePreAnalysis>,
    pub load: FileInfo,
}
impl Debug for File{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File")
            .field("load", &self.load)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseType{
    Full,
    PreAnalysis,
    Dont,//Irrelevant now
}

type ParsedData = Vec<(Arc<RunGlobalInfo>)>;
pub fn identify_globals(files: Vec<FileInfo>) -> Vec<(FileInfo, ParsedData)>{
    let content = files.into_par_iter().enumerate().map(|(id, file_info)| {
        //println!("Loading file: {:?}", file_info.name());
        let path = file_info.path();
        if &FileType::External == file_info.ftype() || file_info.ftype() == &FileType::NativeFuncs {
            //Time to do some wacky shenangans to treat this as a file
            //Random bullshit: GO!
            //TODO: Handle nativefuncs.json seperately 
            let native = file_info.ftype() == &FileType::NativeFuncs;
            let externals = match native{
                true => {
                    //hacky
                    let externals = parse_nativefuncs(&file_info.text());
                    let externals = externals.0.iter().map(|(cond, stuff)| {
                        (cond.clone(), stuff.iter().map(|x| {
                            ExternalResourceEntry{
                                origin: None,
                                resource: ExternalResourceType::Func(x.clone()),
                            }
                        }).collect::<Vec<_>>())
                    }).collect::<HashMap<_, _>>();
                    externals
                },
                false => {
                    parse_externalfuncs(&file_info.text()).0
                }
            };
            //Everything in an external is global by definition so 
            let mut outcomes = Vec::new();
            for (unparsed_condition, set) in externals{
                let parsed_conditions = force_get_states_statement(unparsed_condition);
        
                for cond in parsed_conditions{
                    let cond: CompiledState = cond.into();
                    let primitive_info = RunPrimitiveInfo::new(file_info.clone(), cond, id, vec![]);
                    let primitive_info = Arc::new(primitive_info);
                    let globals = set.iter().map(|x| {
                        let ast = Element::new(AST::ExternalReference(x.clone()), (0, 0));
                        let var = Arc::new(Variable::external(ast, primitive_info.clone()));
                        let name = x.name();
                        return (name, var);
                    }).collect::<Vec<_>>();
        
                    let globals = Globals{
                        globals: TraversableMap::from(globals),
                    };
        
                    //Create preanalysis for this file (it wont even be analysed but eh)
                    //I want to be able to do this once but i need to preserve context for global searching :(
                    let global_info = RunGlobalInfo{
                        globals: globals,
                        primitive: primitive_info.clone(),
                    };
                    let global_info = Arc::new(global_info);
                    outcomes.push(global_info);
                }
            }
            return (file_info, outcomes);
        }

        let asts = generate_asts(file_info.clone(), id);



        let pre_analyses = asts.iter().map(|run| {
            let ast = run.ast.clone();
            let context = run.context.clone();

            let primitive_info = RunPrimitiveInfo::new(file_info.clone(), context.clone(), id, ast);
            let primitive_info = Arc::new(primitive_info);
            let globals = get_globals(primitive_info.clone());
            let global_info = RunGlobalInfo{
                globals: globals,
                primitive: primitive_info.clone(),
            };
            Arc::new(global_info)
        }).collect::<Vec<_>>();
        return (file_info.clone(), pre_analyses);
        //return (file_info.clone(), pre_analyses);
    }).collect::<Vec<_>>();
    return content;
}

pub fn identify_file_tree(content: Vec<(FileInfo, ParsedData)>) -> Vec<Arc<File>>{
    //Todo: multithread this, pretty much all of this can be multithreaded

    //We assume by this point, files are in order
    //This name is probably misleading then but eh fight me
    let file_tree = content.par_iter().map(|(file_info, pre_analyses)| {

        let iter = pre_analyses.iter();
        //let last_thread = last.clone();//Kinda pointless but easier that a formal solution
        let pre_analyses = iter.map(|globals| {
            let context = globals.get_state();
            let dependency = Dependency::<RunGlobalInfo>::construct_all(&content, &context);
            //get globals
            let pre_analysis = FilePreAnalysis{
                //ast: ast.clone(),
                globalinfo: globals.clone(),
                dependencies: dependency,
                untyped: false, //Unimplemented 
            };
            return Arc::new(pre_analysis);
        });

        let mut parse_type = ParseType::Full;
        if file_info.ftype() == &FileType::External || file_info.ftype() == &FileType::NativeFuncs{
            parse_type = ParseType::PreAnalysis;
        }
        let file = File{
            parse_type: parse_type,
            //parent: last.clone(),
            variants: ContextFilter{
                content: pre_analyses.collect(),
            },
            load: file_info.clone(),
        };
        let file = Arc::new(file);
        //last = Some(file.clone());
        file
    }).collect::<Vec<_>>();



    return file_tree;
}

#[cfg(test)]
#[test]
fn basic_tree(){
    
    //let files = todo!();
    //let tree = identify_file_tree(files);
    //assert_eq!(tree.len(), 2);
    ////When forcing index here we must use the verbose conditions that are autogenerated (IE explicit contradictions)
    //let provided = HashMap::from([("SERVER".to_string(), true), ("MP".to_string(), true), ("CLIENT".to_string(), false), ("UI".to_string(), false)]);
    //let second = tree[1].variants.get(&CompiledState::from(provided));
    //if let Some(Dependency::Single(file)) = second{
    //    assert_eq!(file.globalinfo.primitive.file.name(), &"test2".to_string());
    //    assert_eq!(file.dependencies.len(), 1);
    //}else{
    //    panic!("Expected a single dependency but got {:?}", second);
    //}
}