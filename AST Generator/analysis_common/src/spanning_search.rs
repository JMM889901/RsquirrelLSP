use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::{Arc, RwLock}};

pub trait SpanningSearch<T>{
    fn range(&self) -> (usize, usize);
    //Generic trait for nested elements with ranges
    //IE scopes should fall into this
    fn range_passes(&self, pos: usize, leeway: usize) -> bool{
        let range = self.range();
        return range.0 <= pos + leeway && range.1 + leeway >= pos;
    }
}

pub struct Traversable<T: SpanningSearch<T>>(RwLock<Vec<Arc<T>>>);
impl <T: SpanningSearch<T>> Traversable<T>{
    pub fn new() -> Self{
        Traversable(RwLock::new(Vec::new()))
    }
    pub fn add(&self, item: T){
        let mut items = self.0.write().unwrap();
        items.push(Arc::new(item));
    }
    pub fn add_arc(&self, item: Arc<T>){
        let mut items = self.0.write().unwrap();
        items.push(item);
    }
    pub fn get(&self) -> Vec<Arc<T>>{
        self.0.read().unwrap().clone()//Is there a good way to like, not clone that?
    }
    pub fn get_pos(&self, pos: usize, leeway: usize) -> Vec<Arc<T>>{
        let items = self.0.read().unwrap();
        items.iter().filter(|x| x.range_passes(pos, leeway) ).cloned().collect()
        //TODO: Binary search is cool and good
    }
    pub fn clear(&self){
        let mut items = self.0.write().unwrap();
        items.clear();
    }
    pub fn remove_ptr(&self, item: &Arc<T>){
        let mut items = self.0.write().unwrap();
        items.retain(|x| !Arc::ptr_eq(x, item));
    }
    pub fn remove_pos(&self, pos: usize, leeway: usize){
        let mut items = self.0.write().unwrap();
        items.retain(|x| !x.range_passes(pos, leeway));
    }
}
impl <T: SpanningSearch<T> + Debug> Debug for Traversable<T>{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Traversable")
            .field("items", &self.0)
            .finish()
    }
}

pub struct TraversableMap<A: Clone, T: SpanningSearch<T>>(RwLock<HashMap<A, Arc<T>>>);
impl <A: Clone+ Eq + Hash, T: SpanningSearch<T>>TraversableMap<A, T>{
    pub fn new() -> Self{
        TraversableMap(RwLock::new(HashMap::new()))
    }
    pub fn add(&self, key: A, item: T){
        let mut items = self.0.write().unwrap();
        items.insert(key, Arc::new(item));
    }
    pub fn add_arc(&self, key: A, item: Arc<T>){
        let mut items = self.0.write().unwrap();
        items.insert(key, item);
    }
    pub fn get(&self) -> HashMap<A, Arc<T>>{
        self.0.read().unwrap().clone()
    }
    pub fn get_pos(&self, pos: usize, leeway: usize) -> Vec<Arc<T>>{
        let items = self.0.read().unwrap();
        let mut result = Vec::new();
        for (key, item) in items.iter(){
            if item.range_passes(pos, leeway){
                result.push(item.clone());
            }
        }
        return result;
    }
    pub fn index(&self, key: &A) -> Option<Arc<T>>{
        let items = self.0.read().unwrap();
        if let Some(item) = items.get(key){
            return Some(item.clone());
        }
        return None;
    }
}
impl <A: Clone + Eq + Hash + Debug, T: SpanningSearch<T> + Debug> Debug for TraversableMap<A, T>{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Traversable")
            .field("items", &self.0)
            .finish()
   
    }
}
impl <A: Clone+ Eq + Hash, T: SpanningSearch<T>> From<Vec<(A, Arc<T>)>> for TraversableMap<A, T>{
    fn from(items: Vec<(A, Arc<T>)>) -> Self {
        let map = HashMap::from_iter(items);
        let map = TraversableMap(RwLock::new(map));
        return map;
    }
}