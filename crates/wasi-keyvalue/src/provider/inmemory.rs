use crate::{bindings::wasi::keyvalue::store::KeyResponse, to_other_error, Error, Host};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub(crate) struct InMemory {
    store: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl InMemory {
    pub(crate) fn new(data: HashMap<String, Vec<u8>>) -> Self {
        Self {
            store: Arc::new(Mutex::new(data)),
        }
    }
}

#[async_trait]
impl Host for InMemory {
    async fn get(&mut self, key: String) -> Result<Option<Vec<u8>>, Error> {
        let store = self.store.lock().unwrap();
        Ok(store.get(&key).cloned())
    }

    async fn set(&mut self, key: String, value: Vec<u8>) -> Result<(), Error> {
        let mut store = self.store.lock().unwrap();
        store.insert(key, value);
        Ok(())
    }

    async fn delete(&mut self, key: String) -> Result<(), Error> {
        let mut store = self.store.lock().unwrap();
        store.remove(&key);
        Ok(())
    }

    async fn exists(&mut self, key: String) -> Result<bool, Error> {
        let store = self.store.lock().unwrap();
        Ok(store.contains_key(&key))
    }

    async fn list_keys(&mut self, cursor: Option<u64>) -> Result<KeyResponse, Error> {
        let store = self.store.lock().unwrap();
        let keys: Vec<String> = store.keys().cloned().collect();
        let cursor = cursor.unwrap_or(0) as usize;
        let keys_slice = &keys[cursor..];
        Ok(KeyResponse {
            keys: keys_slice.to_vec(),
            cursor: None,
        })
    }

    async fn increment(&mut self, key: String, delta: u64) -> Result<u64, Error> {
        let mut store = self.store.lock().unwrap();
        let value = store
            .entry(key.clone())
            .or_insert("0".to_string().into_bytes());
        let current_value = String::from_utf8(value.clone())
            .map_err(to_other_error)?
            .parse::<u64>()
            .map_err(to_other_error)?;
        let new_value = current_value + delta;
        *value = new_value.to_string().into_bytes();
        Ok(new_value)
    }

    async fn get_many(
        &mut self,
        keys: Vec<String>,
    ) -> Result<Vec<Option<(String, Vec<u8>)>>, Error> {
        let store = self.store.lock().unwrap();
        Ok(keys
            .into_iter()
            .map(|key| store.get(&key).map(|value| (key.clone(), value.clone())))
            .collect())
    }

    async fn set_many(&mut self, key_values: Vec<(String, Vec<u8>)>) -> Result<(), Error> {
        let mut store = self.store.lock().unwrap();
        for (key, value) in key_values {
            store.insert(key, value);
        }
        Ok(())
    }

    async fn delete_many(&mut self, keys: Vec<String>) -> Result<(), Error> {
        let mut store = self.store.lock().unwrap();
        for key in keys {
            store.remove(&key);
        }
        Ok(())
    }
}
