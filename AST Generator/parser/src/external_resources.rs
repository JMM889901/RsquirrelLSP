use std::{collections::HashMap, path::PathBuf};

use json_spanned_value::spanned;
use serde::{Deserialize, Serialize};
//Custom JSON format for specifying functions and such, 
//Resembles the nativefuncs.json format used by northstar (TODO: Is this titanfall thing or northstar thing)
//see https://github.com/R2Northstar/NorthstarMods/blob/main/.github/nativefuncs.json
//unlike nativefuncs this can also include none-funcs, hence the name

//Both nativefuncs and externals do not intuit conditions
//For example, if something is listed under SERVER and not SERVER && MP we will assume that whatever reason, it is not loaded in that case
//This is partially to match the behaviour of nativefuncs, and also just to simplify everything
//^ This will not be practical long term

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ExternalResource(pub HashMap<String, Vec<ExternalResourceEntry>>);

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ExternalResourceEntry{
    pub origin: Option<Origin>,
    #[serde(flatten)]
    pub resource: ExternalResourceType,
}
impl ExternalResourceEntry{
    pub fn name(&self) -> String{
        match &self.resource{
            ExternalResourceType::Func(func) => func.name.to_string(),
            ExternalResourceType::Var(var) => var.name.to_string(),
            ExternalResourceType::Struct(struc) => struc.name.to_string(),
        }
    }
    pub fn range(&self) -> (usize, usize){
        match &self.origin{
            Some(origin) => origin.pos,
            None => self.resource.range(),
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum ExternalResourceType{
    Func(ExternalFunc),
    Var(ExernalVar),
    Struct(ExternalStruct),
}
impl ExternalResourceType{
    pub fn range(&self) -> (usize, usize){
        let range = match self{
            ExternalResourceType::Func(func) => func.name.range(),
            ExternalResourceType::Var(var) => var.name.range(),
            ExternalResourceType::Struct(struc) => struc.name.range(),
        };
        return (range.start, range.end);
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ExternalStruct{
    pub name: spanned::String,
    #[serde(default)]
    pub helpText: String,
    #[serde(default)]
    pub fields: Vec<ExternalStructField>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ExternalStructField{
    pub name: spanned::String,
    #[serde(default)]
    pub helpText: String,
    #[serde(default)]
    pub fieldType: String,
}

//Fuck it, lets also just include some code to load nativefuncs.json anyways
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct NativeFuncs(pub HashMap<String, Vec<ExternalFunc>>);

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ExternalFunc{
    pub name: spanned::String,
    #[serde(default)]
    pub helpText: String,
    pub returnTypeString: String,
    #[serde(default)]
    pub argTypes: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Origin{
    pub path: String,
    pub pos: (usize, usize)
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ExernalVar{
    pub name: spanned::String,
    #[serde(default)]
    pub description: String
}

pub fn parse_externalfuncs(text: &String) -> ExternalResource{
    //let file = std::fs::File::open(path.clone()).map_err(|e| format!("Failed to open file: {}", e)).unwrap();
    //let reader = std::io::BufReader::new(file);
    //let natives: ExternalResource = serde_json::from_reader(reader).map_err(|e| format!("Failed to parse JSON: {}", e)).unwrap();
    let natives: ExternalResource = json_spanned_value::from_str(text.as_str()).unwrap();
    return natives;
}

pub fn parse_nativefuncs(text: &String) -> NativeFuncs{
    //let file = std::fs::File::open(path.clone()).map_err(|e| format!("Failed to open file: {}", e)).unwrap();
    //let reader = std::io::BufReader::new(file);
    //let natives: NativeFuncs = serde_json::from_reader(reader).map_err(|e| format!("Failed to parse JSON: {}", e)).unwrap();
    let natives: NativeFuncs = json_spanned_value::from_str(text.as_str()).unwrap();
    return natives;
}

#[test]
fn test_parse_nativefuncs(){
    let path = "../../TestSets/ValidStructure/external/nativefuncs.json";
    let file = std::fs::File::open(path.clone()).map_err(|e| format!("Failed to open file: {}", e)).unwrap();
    let reader = std::io::BufReader::new(file);
    let natives: NativeFuncs = serde_json::from_reader(reader).map_err(|e| format!("Failed to parse JSON: {}", e)).unwrap();
    println!("{:?}", natives);


}

#[test]
fn test_parse_externalfuncs(){
    let path = "../../TestSets/ValidStructure/external/externals.json";
    let file = std::fs::File::open(path.clone()).map_err(|e| format!("Failed to open file: {}", e)).unwrap();
    let reader = std::io::BufReader::new(file);
    let natives: ExternalResource = serde_json::from_reader(reader).map_err(|e| format!("Failed to parse JSON: {}", e)).unwrap();
    println!("{:?}", natives);


}