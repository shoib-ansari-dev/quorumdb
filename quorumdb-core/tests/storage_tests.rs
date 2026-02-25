use quorumdb_core::StorageEngine;

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use super::*;

    #[tokio::test]
    async fn test_set_and_get(){
        let engine= StorageEngine::<String, String>::new();
        engine.set("key1".to_string(), "value1".to_string()).await.expect("Error");
        assert_eq!(engine.get(&"key1".to_string()), Ok(Some("value1".to_string())));
    }
    #[test]
    fn test_get_non_existing_key(){
        let engine= StorageEngine::<String, String>::new();
        assert_eq!(engine.get(&"non_existing".to_string()), Ok(None));
    }
    #[tokio::test]
    async fn test_delete(){
        let engine= StorageEngine::<String, String>::new();
        engine.set("key1".to_string(), "value1".to_string()).await.expect("Error");
        assert_eq!(engine.delete(&"key1".to_string()), Ok(Some("value1".to_string())));
        assert_eq!(engine.get(&"key1".to_string()), Ok(None));
    }
    #[tokio::test]
    async fn test_thread_safety(){
        let engine= Arc::new(StorageEngine::<String, String>::new());
        let mut handles = vec![];

        for i in 0..10 {
            let engine_clone = engine.clone();
            let handle = tokio::spawn(async move {
                let key = format!("key{}", i);
                let value = format!("value{}", i);
                engine_clone.set(key.clone(), value.clone()).await.expect("Error");
                assert_eq!(engine_clone.get(&key), Ok(Some(value)));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }
        assert_eq!(engine.len().unwrap_or(0), 10);
    }
}
