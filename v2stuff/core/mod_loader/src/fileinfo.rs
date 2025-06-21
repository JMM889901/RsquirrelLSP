use crate::ModID;
use std::fmt::Display;
use std::{
    io::Read,
    path::PathBuf,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone)]
pub struct ModScriptEntry {
    pub rel_path: PathBuf,
    pub run_on: String,
    //server_callbacka: Option<Callback>,
    //client_callback: Option<Callback>,
    //ui_callback: Option<Callback>,
}
impl ModScriptEntry {
    pub fn get_name(&self) -> String {
        //Get the name of the script, this is just the file name without the extension
        self.rel_path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "This is less than ideal".to_string())
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedFile {
    pub load_position: usize, //File's position in the overall load order
    pub file_type: FileType,
    pub file_info: FileInfo,
}
impl Display for LoadedFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LoadedFile {{ load_position: {}, file_type: {:?}, file_info: {} }}",
            self.load_position, self.file_type, self.file_info
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Script,
    Externals(ExternalType),
}

/// Maybe at some point, but not now
pub struct SquirrelAmbiguiousFile {
    pub loaded_by: ModID, //Specifically the mod.json that says we load a file in this place
    pub files: Vec<SquirrelFile>,
}
pub struct SquirrelFile {}

#[derive(Debug, Clone, PartialEq)]
pub enum ExternalType {
    Externals,
    NativeFuncs,
}
#[derive(Debug, Clone)]
pub struct ExternalsFileInfo {
    pub external_type: ExternalType,
}

#[derive(Debug, Clone)]
pub struct FileInfo(Arc<FileInfoInternal>);
//TODO: This is also just copied from old version
#[derive(Debug)]
pub struct FileInfoInternal {
    pub text: RwLock<Option<Arc<String>>>,
    pub offsets: RwLock<Option<Vec<usize>>>,
    pub length: RwLock<Option<usize>>,
    pub name: String,
    pub path: PathBuf,
    pub run_on: String,
    pub loaded_by: ModID, //The mod from which this file was found
    #[cfg(feature = "timed")]
    pub preprocess_time: RwLock<std::time::Duration>,
    #[cfg(feature = "timed")]
    pub sq_parse_time: RwLock<std::time::Duration>,
}
impl PartialEq for FileInfo {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)//Look into what the optimal partial eq is for this, Not sure if i can actually guarantee this
    }
}
impl Display for FileInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FileInfo {{ name: {}, path: {:?}, run_on: {}, loaded_by: {} }}",
            self.0.name,
            self.0.path,
            self.0.run_on,
            self.loaded_by()
        )
    }
}

impl FileInfo {
    pub fn loaded_by(&self) -> ModID {
        self.0.loaded_by
    }

    pub fn new(name: String, path: PathBuf, run_on: String, loaded_by: ModID) -> Self {
        FileInfo(Arc::new(FileInfoInternal {
            text: RwLock::new(None),
            offsets: RwLock::new(None),
            length: RwLock::new(None),
            name,
            path,
            run_on,
            loaded_by,
            #[cfg(feature = "timed")]
            preprocess_time: RwLock::new(std::time::Duration::new(0, 0)),
            #[cfg(feature = "timed")]
            sq_parse_time: RwLock::new(std::time::Duration::new(0, 0)),
        }))
    }
}

/// Old implementations, these do a lot of scary unwraps and I'm not a huge fan:
impl FileInfo {
    #[cfg(feature = "timed")]
    pub fn set_preproc_time(&self, time: std::time::Duration) {
        *self.0.preprocess_time.try_write().unwrap() = time;
    }
    #[cfg(feature = "timed")]
    pub fn set_sq_parse_time(&self, time: std::time::Duration) {
        *self.0.sq_parse_time.try_write().unwrap() = time;
    }
    #[cfg(feature = "timed")]
    pub fn get_preproc_time(&self) -> std::time::Duration {
        return *self.0.preprocess_time.read().unwrap();
    }
    #[cfg(feature = "timed")]
    pub fn get_sq_parse_time(&self) -> std::time::Duration {
        return *self.0.sq_parse_time.read().unwrap();
    }
    pub fn name(&self) -> &String {
        &self.0.name
    }
    pub fn path(&self) -> &PathBuf {
        &self.0.path
    }
    pub fn run_on(&self) -> &String {
        &self.0.run_on
    }

    pub fn text(&self) -> Arc<String> {
        let read = self.0.text.read().unwrap();
        if let Some(text) = read.as_ref() {
            return text.clone();
        }
        drop(read); //Explicit drop because readtext deadlocks
                    // Read that MF file boyos
        self.read_text();
        self.text()
    }

    pub fn offsets(&self) -> Vec<usize> {
        let read = self.0.offsets.read().unwrap();
        if let Some(offsets) = read.as_ref() {
            return offsets.clone();
        }
        drop(read); //Explicit drop because readtext deadlocks
                    //Read that MF file boyos
        self.read_text();
        self.offsets()
    }

    pub fn text_len(&self) -> usize {
        let read = self.0.length.read().unwrap();
        if let Some(length) = read.as_ref() {
            return *length;
        }
        drop(read); //Explicit drop because readtext deadlocks
                    // Read that MF file boyos
        self.read_text();
        self.text_len()
    }

    pub fn purge(&self) {
        //This is a really heavy word for this idk why i make it sound like a big deal
        //Purge the text and offsets
        *self.0.text.try_write().unwrap() = None;
        *self.0.offsets.try_write().unwrap() = None;
        *self.0.length.try_write().unwrap() = None;
    }

    fn read_text(&self) {
        //Read the file
        let file = std::fs::File::open(&self.0.path);
        let contents;

        if let Ok(mut file) = file {
            //file.read_to_string(&mut contents).expect(format!("Failed to read file {:?}", self.0.path).as_str());
            let mut buf = vec![];
            file.read_to_end(&mut buf)
                .unwrap_or_else(|_| panic!("Failed to read file {:?}", self.0.path));
            contents = String::from_utf8_lossy(&buf).to_string(); //Generated nut files include the copyright symbol
                                                                  //Lol
        } else {
            contents = String::new();
            //Not the biggest fan, but we need to because northstar loads sh_gamemode_fw
        }

        let lines = contents.split("\n").collect::<Vec<_>>();
        let mut offset = 0;
        let mut offsets = vec![0];
        for line in lines {
            offset += line.len() + 1;
            offsets.push(offset);
        }

        //Set the length
        *self.0.length.try_write().unwrap() = Some(contents.len());
        //Set the text
        *self.0.text.try_write().unwrap() = Some(Arc::new(contents));
        //Set the offsets
        *self.0.offsets.try_write().unwrap() = Some(offsets);
    }

    pub fn set_text(&self, text: String) {
        //Set the length
        *self.0.length.try_write().unwrap() = Some(text.len());
        //Set the offsets
        let lines = text.split("\n").collect::<Vec<_>>();
        let mut offset = 0;
        let mut offsets = vec![0];
        for line in lines {
            offset += line.len() + 1;
            offsets.push(offset);
        }
        *self.0.offsets.try_write().unwrap() = Some(offsets);
        *self.0.text.try_write().unwrap() = Some(Arc::new(text));
    }

    pub fn offset_to_linecol(&self, offset: usize) -> (usize, usize) {
        //Convert the offset to a line and column number
        let read = self.0.offsets.read().unwrap();
        if let Some(offsets) = read.as_ref() {
            for (i, o) in offsets.iter().enumerate() {
                if *o > offset {
                    return (i - 1, offset - offsets[i - 1]);
                }
            }
        }
        drop(read); //Explicit drop because readtext deadlocks
        self.read_text();
        self.offset_to_linecol(offset)
    }

    pub fn linecol_to_offset(&self, line: usize, col: usize) -> usize {
        //Convert the line and column number to an offset
        let read = self.0.offsets.read().unwrap();
        if let Some(offsets) = read.as_ref() {
            if line < offsets.len() {
                return offsets[line] + col;
            }
        }
        drop(read); //Explicit drop because readtext deadlocks
        self.read_text();
        self.linecol_to_offset(line, col)
    }
}
