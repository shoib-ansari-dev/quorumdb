mod storage;

pub use storage::engine::StorageEngine;

// Type alias for the concrete implementation (String key-value store)
pub type KvStore = StorageEngine<String, String>;
