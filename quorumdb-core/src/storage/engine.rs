use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::RwLock as TokioRwLock;
use crate::storage::wal::{LogEntry, WriteAheadLog};

pub struct StorageEngine<K, V>
where
    K: Clone+ Eq+ Hash,
    V: Clone,
{
    data: Arc<RwLock<HashMap<K, V>>>,
    wal: Arc<TokioRwLock<Option<WriteAheadLog>>>,
}

impl<K, V> StorageEngine<K, V>
where
    K: Clone+ Eq+ Hash,
    V: Clone,
{

    pub fn new() -> Self{
        Self{
            data: Arc::new(RwLock::new(HashMap::new())),
            wal: Arc::new(TokioRwLock::new(None)),
        }
    }

    pub async fn init_wal(&self, path: &str) -> Result<(), String>{
        let wal = WriteAheadLog::new(path)
            .await
            .map_err(|e| format!("Failed to init WAL: {}", e))?;

        *self.wal.write().await = Some(wal);
        Ok(())
    }

    pub async fn set(&self, key: K, value: V) -> Result<(), String>
    where
        K: std::fmt::Display,
        V: std::fmt::Display,
    {
        let entry = LogEntry::Set {
            key: key.to_string(),
            value: value.to_string(),
        };

        // Write to WAL if it exists
        if let Some(wal) = self.wal.write().await.as_mut() {
            wal.write(&entry).await?;
        }

        // Apply to RAM
        self.data.write().insert(key, value);
        Ok(())
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
            wal: Arc::clone(&self.wal),
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