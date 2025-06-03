use core::panic;
use std::{io::Read, path::{Path, PathBuf}, str, sync::{Arc, RwLock}};

#[derive(Debug, Clone)]
pub struct FileInfo(Arc<FileInfoInternal>);

//Represents a given file
//TODO: I make like a "purge after X point" function so i can keep relevant results from previous runs
#[derive(Debug)]
struct FileInfoInternal {
    pub text: RwLock<Option<Arc<String>>>,
    //This is a lot of text to be copying so lets avoid that
    pub ftype: FileType,
    pub name: String,
    pub path: PathBuf,//This is expected to be the FULL path, as in readable
    pub offsets: RwLock<Option<Vec<usize>>>,//Line number offsets to make VSCode play nice
    pub length: RwLock<Option<usize>>, //This is the length of the file, in bytes.
    //I specify this becuase prior to this i have a complete shitshow of relative and absolute paths
    pub run_on: String,
    #[cfg(feature = "timed")]
    pub preprocess_time: RwLock<std::time::Duration>,
    #[cfg(feature = "timed")]
    pub sq_parse_time: RwLock<std::time::Duration>,
}

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
        return &self.0.name;
    }
    pub fn path(&self) -> &PathBuf {
        return &self.0.path;
    }
    pub fn run_on(&self) -> &String {
        return &self.0.run_on;
    }
    pub fn ftype(&self) -> &FileType {
        return &self.0.ftype;
    }
    #[cfg(feature = "timed")]
    pub fn new(name: String, path: PathBuf, run_on: String, ftype: FileType) -> Self {
        FileInfo(Arc::new(FileInfoInternal {
            text: RwLock::new(None),
            offsets: RwLock::new(None),
            length: RwLock::new(None),
            ftype: ftype,
            name,
            path,
            run_on,
            preprocess_time: RwLock::new(std::time::Duration::new(0, 0)),
            sq_parse_time: RwLock::new(std::time::Duration::new(0, 0)),
        }))
    }


    #[cfg(not(feature = "timed"))]
    pub fn new(name: String, path: PathBuf, run_on: String, ftype: FileType) -> Self {
        FileInfo(Arc::new(FileInfoInternal {
            text: RwLock::new(None),
            offsets: RwLock::new(None),
            length: RwLock::new(None),
            ftype: ftype,
            name,
            path,
            run_on,
        }))
    }

    #[cfg(all(test, not(feature = "timed")))]
    pub fn new_testing(text: String, name: String, path: String, run_on: String, ftype: FileType) -> Self {
        FileInfo(Arc::new(FileInfoInternal {
            text: RwLock::new(Some(Arc::new(text))),
            offsets: RwLock::new(None),
            length: RwLock::new(None),
            ftype: ftype,
            name,
            path: PathBuf::from(path),
            run_on,
        }))
    }

    pub fn text(&self) -> Arc<String> {
        let read =  self.0.text.read().unwrap();
        if let Some(text) = read.as_ref() {
            return text.clone();
        }
        drop(read); //Explicit drop because readtext deadlocks
        // Read that MF file boyos
        self.read_text();
        return self.text();
        panic!("uh, thats a problem"); // This probably won't happen? depends on how I rework this bit
    }

    pub fn offsets(&self) -> Vec<usize> {
        let read = self.0.offsets.read().unwrap();
        if let Some(offsets) = read.as_ref() {
            return offsets.clone();
        }
        drop(read); //Explicit drop because readtext deadlocks
        //Read that MF file boyos
        self.read_text();
        return self.offsets();
        return vec![];//This probably wont happen? depends on how i rework this bit
    }

    pub fn len(&self) -> usize {
        let read = self.0.length.read().unwrap();
        if let Some(length) = read.as_ref(){
            return *length;
        }
        drop(read); //Explicit drop because readtext deadlocks 
        // Read that MF file boyos
        self.read_text();
        return self.len();
        panic!("uh, thats a problem"); // This probably won't happen? depends on how I rework this bit
    }

    pub fn purge(&self) {
        //This is a really heavy word for this idk why i make it sound like a big deal
        //Purge the text and offsets
        *self.0.text.try_write().unwrap() = None;
        *self.0.offsets.try_write().unwrap() = None;
        *self.0.length.try_write().unwrap() = None;
    }

    fn read_text(&self){
        //Read the file
        let mut file = std::fs::File::open(&self.0.path);
        let mut contents = String::new();

        if let Ok(mut file) = file{
            file.read_to_string(&mut contents).expect("Failed to read file");
        } else {
            contents = String::new();
            //Not the biggest fan, but we need to because northstar loads sh_gamemode_fw
        }

        let lines = contents.split("\n").collect::<Vec<_>>();
        let mut offset = 0;
        let mut offsets = vec![0];
        for line in lines{
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
        for line in lines{
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
                    return (i-1, offset - offsets[i - 1]);
                }
            }
        }
        drop(read); //Explicit drop because readtext deadlocks
        self.read_text();
        return self.offset_to_linecol(offset);
        panic!("uh, thats a problem"); // This probably won't happen? depends on how I rework this bit
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
        return self.linecol_to_offset(line, col);
        panic!("uh, thats a problem"); // This probably won't happen? depends on how I rework this bit
    }
}




#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    External,
    NativeFuncs,
    RSquirrel,
}