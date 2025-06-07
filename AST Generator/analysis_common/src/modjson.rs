use std::{fs::read_to_string, path::{self, Path, PathBuf}};
//Tools for deserializing mod.json files
use super::*;
use common::{FileInfo, FileType};
use serde::{Deserialize, Serialize};


peg::parser!{
    pub grammar rson(rsonname: &String) for str {
        rule _() = quiet!{[' ' | '\t' | '\n' | '\r']*}



		rule ws() = quiet!{[' ' | '\t' | '\n' | '\r']*}
		rule ws1() = quiet!{[' ' | '\t']*}
		rule nl() = ws1() (comment()? ws1() "\n" ws1())+ ws1()
		rule comment() = quiet!{ws() "//" (!['\n'] [_])*} / quiet!{ws() "/*" (!"*/" [_])* "*/"}
		rule string() -> String
			= "\"" s:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '/' | '_' | '.' | '-' ]*) "\"" {s.to_string()}
		rule when_scripts() -> Vec<Script>
			= "When:" ws() "\"" run_on:$(['a'..='z' | 'A'..='Z' | ' ' | '|' | '(' | ')' | '&' | '0'..='9' | '_']*) "\"" ws() 
				"Scripts:" ws() "[" ws() s:loaded_file(run_on.to_string())**( ws1() ("\n\r" / "\r\n" / "\n" / "\r") ws()) ws() "]" {s}
		rule loaded_file(run_on: String) -> Script
			= name:ident() ws1() comment()? {Script{run_on, path: name}}

		rule ident() -> String
			= a:$quiet!{['a'..='z' | 'A'..='Z' | '0'..='9' | '/' | '_' | '.' | '-' ]+}  {a.to_string()}


		pub rule rson() -> ModPure
			= (ws() comment())* ws() files:when_scripts() ** (ws() (comment() ws() )*) ws() (comment() ws())* {
				let flat_files = files.into_iter().flatten().collect();
				ModPure {
                    load_priority: 0, //TODO: Implement load priority
                    name: rsonname.to_string(),
                    scripts: flat_files,
                }
			}

    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModPure{
    #[serde(rename = "LoadPriority")]
    load_priority: usize,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Scripts")]
    pub scripts: Vec<Script>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script{
    #[serde(rename = "Path")]
    pub path: String,
    #[serde(rename = "RunOn")]
    pub run_on: String,
}

pub struct Mod{
    pub name: String,
    pub scripts: Vec<FileInfo>,
    pub load_priority: usize,
    pub path: PathBuf,
}

pub fn load_base(path: PathBuf) -> Result<Mod, String>{
    //Mostly unimplemented
    //Assume path is to nativefuncs.json
    //Externals is a per-mod thing
    if std::fs::metadata(&path).is_err(){
        return Err(format!("File not found: {:?}", path));
    }
    let fileinfo = FileInfo::new(
        "nativefuncs.json".to_string(), path.clone(), "MP || UI".to_string(), FileType::NativeFuncs
    );
    let scripts = vec![fileinfo];
    return Ok(
        Mod { name: "base".to_string(), scripts, load_priority: 0, path: path.clone() }
    )
}

pub fn load_mods(path: PathBuf) -> Result<Vec<Mod>, String>{
    //Get all subfolders in path
    let mut mods = Vec::new();
    for entry in path.read_dir().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            if let Ok(modfile) = load_mod(path.clone()){
                mods.push(modfile);
            }
        }
    }
    if mods.is_empty(){
        //Presume we are in the mod
        mods = vec![load_mod(path.clone())?];
    }
    return Ok(mods)
}

pub fn load_rson(path: PathBuf) -> Result<Mod, String>{
    //For now im just going to assume this is scripts.rson
    //If you want to write your own rsons errrrrr please dont 
    let mut rson_file = path.join("scripts.rson");
    if !rson_file.exists() {
        rson_file = path.join("mod/scripts/vscripts/scripts.rson");
    }
    let text = read_to_string(&rson_file).map_err(|e| format!("Why no read :( {}", e))?;
    let mod_pure = rson::rson(&text, &path.file_name().unwrap().to_string_lossy().to_string()).map_err(|e| format!("Failed to parse RSON: {}", e))?;
    let mut scripts = Vec::new();
    let mut script_path = path.clone();
    if !script_path.ends_with("vscripts") {
        script_path = script_path.join("mod/scripts/vscripts");
    }
    scripts.extend(mod_pure.scripts.iter().map(|x| {
        let scriptpath = script_path.join(&x.path);
        let name = scriptpath.file_name().unwrap_or_default().to_string_lossy().to_string();//ew, gross
        FileInfo::new(
            name,
            scriptpath,
            x.run_on.clone(),
            FileType::RSquirrel,
        )
    }).collect::<Vec<_>>());
    
    let mod_full = Mod{
        name: mod_pure.name,
        scripts,
        load_priority: mod_pure.load_priority,
        path: path,
    };
    Ok(mod_full)
}

pub fn load_mod(path: PathBuf) -> Result<Mod, String>{

    //Get externals.json and nativefuncs.json
    let externals_path = path.join("externals.json");//TODO: Lol, just lol
    let natives_path = path.join("nativefuncs.json");
    let externals = std::fs::metadata(&externals_path).is_ok();
    let natives = std::fs::metadata(&natives_path).is_ok();
    let mut scripts = Vec::new();
    if externals{//Technically natives should come first but seeing as they cant actually reference eachother, who cares
        scripts.push(FileInfo::new(
            "externals.json".to_string(), externals_path, "MP || UI".to_string(), FileType::External)
        )
    }
    if natives{
        scripts.push(FileInfo::new(
            "nativefuncs.json".to_string(), natives_path, "MP || UI".to_string(), FileType::NativeFuncs)
        )
    }

    //add /mod.json
    let jsonpath = path.join("mod.json");
    let file = std::fs::File::open(jsonpath.clone()).map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = std::io::BufReader::new(file);
    let mod_json: ModPure = serde_json::from_reader(reader).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let scriptpath = path.join("mod/scripts/vscripts");
    scripts.extend(mod_json.scripts.iter().map(|x| {
        let scriptpath = scriptpath.join(&x.path);
        let name = scriptpath.file_name().unwrap_or_default().to_string_lossy().to_string();//ew, gross
        FileInfo::new(
            name,
            scriptpath,
            x.run_on.clone(),
            FileType::RSquirrel,
        )
    }).collect::<Vec<_>>());

    let mod_full = Mod{
        name: mod_json.name,
        scripts,
        load_priority: mod_json.load_priority,
        path: path,
    };
    Ok(mod_full)
}

#[cfg(test)]
#[test]
fn flipside(){
    let path = "../../TestSets/RealSets/Flipside/mod.json";
    let res = load_mod(PathBuf::from(path));
    match res{
        Ok(mod_json) => {
            println!("Mod name: {}", mod_json.name);
            println!("Load priority: {}", mod_json.load_priority);
            for script in mod_json.scripts{
                println!("Script path: {:?}", script.path());
                println!("Run on: {}", script.run_on());
            }
        },
        Err(e) => {
            println!("Error loading mod.json: {}", e);
        }
    }
}