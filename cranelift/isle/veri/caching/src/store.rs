//! Backing object stores for the cache.

use std::collections::BTreeMap;
use std::io;
use std::sync::Mutex;

/// A flat key-value object store, in the style of S3: whole objects are
/// written and read atomically, and there is no rename or multi-object
/// transaction. Object names are relative to whatever prefix the store was
/// configured with.
pub trait ObjectStore: Send + Sync {
    /// List the names of all objects whose name starts with `prefix`.
    fn list(&self, prefix: &str) -> io::Result<Vec<String>>;
    /// Fetch an object, or `None` if it does not exist.
    fn get(&self, name: &str) -> io::Result<Option<Vec<u8>>>;
    /// Create or replace an object.
    fn put(&self, name: &str, body: &[u8]) -> io::Result<()>;
    /// Delete an object. Deleting a nonexistent object is not an error.
    fn delete(&self, name: &str) -> io::Result<()>;
    /// A human-readable description of the store's location.
    fn describe(&self) -> String;
}

/// An in-memory store, for tests.
#[derive(Default)]
pub struct MemoryStore {
    objects: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl ObjectStore for MemoryStore {
    fn list(&self, prefix: &str) -> io::Result<Vec<String>> {
        let objects = self.objects.lock().unwrap();
        Ok(objects
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect())
    }

    fn get(&self, name: &str) -> io::Result<Option<Vec<u8>>> {
        Ok(self.objects.lock().unwrap().get(name).cloned())
    }

    fn put(&self, name: &str, body: &[u8]) -> io::Result<()> {
        self.objects
            .lock()
            .unwrap()
            .insert(name.to_string(), body.to_vec());
        Ok(())
    }

    fn delete(&self, name: &str) -> io::Result<()> {
        self.objects.lock().unwrap().remove(name);
        Ok(())
    }

    fn describe(&self) -> String {
        "(in-memory)".to_string()
    }
}
