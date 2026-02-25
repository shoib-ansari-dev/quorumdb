use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use parking_lot::RwLock;

pub struct StorageEngine<K, V>
where
    K: Clone+ Eq+ Hash,
    V: Clone,
{
    data: Arc<RwLock<HashMap<K, V>>>,
}

impl<K, V> StorageEngine<K, V>
where
    K: Clone+ Eq+ Hash,
    V: Clone,
{

    pub fn new() -> Self{
        Self{
            data: Arc::new(RwLock::new(HashMap::new()))
        }
    }
    pub fn set(&self, key: K, value: V){
        self.data.write().insert(key, value);
    }
    pub fn get(&self, key: &K) -> Result<Option<V>, String>{
            Ok(self.data.read().get(key).cloned())
    }
    pub fn delete(&self, key: &K) -> Result<Option<V>, String>{
        Ok(self.data.write().remove(key))
    }
    pub fn is_empty(&self)-> Result<bool, String>{
        Ok(self.data.read().is_empty())
    }
    pub fn clear(&self) -> Result<(), String>{
        self.data.write().clear();
        Ok(())
    }
    pub fn len(&self) -> Result<u64, String>{
        Ok(self.data.read().len() as u64)
    }

}
impl<K, V> Clone for StorageEngine<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
        }
    }
}
impl<K, V> Default for StorageEngine<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}