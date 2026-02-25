use quorumdb_core::StorageEngine;

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use super::*;

    #[test]
    fn test_set_and_get(){
        let engine= StorageEngine::<String, String>::new();
        engine.set("key1".to_string(), "value1".to_string());
        assert_eq!(engine.get(&"key1".to_string()), Ok(Some("value1".to_string())));
    }
    #[test]
    fn test_get_non_existing_key(){
        let engine= StorageEngine::<String, String>::new();
        assert_eq!(engine.get(&"non_existing".to_string()), Ok(None));
    }
    #[test]
    fn test_delete(){
        let engine= StorageEngine::<String, String>::new();
        engine.set("key1".to_string(), "value1".to_string());
        assert_eq!(engine.delete(&"key1".to_string()), Ok(Some("value1".to_string())));
        assert_eq!(engine.get(&"key1".to_string()), Ok(None));
    }
    #[test]
    fn test_thread_safety(){
        let engine= Arc::new(StorageEngine::<String, String>::new());
        let handles: Vec<_> = (0..10).map(|i| {
            let engine_clone = engine.clone();
            std::thread::spawn(move || {
                let key = format!("key{}", i);
                let value = format!("value{}", i);
                engine_clone.set(key.clone(), value.clone());
                assert_eq!(engine_clone.get(&key), Ok(Some(value)));
            })
        }).collect();

        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(engine.len().unwrap_or(0), 10);
    }
}
