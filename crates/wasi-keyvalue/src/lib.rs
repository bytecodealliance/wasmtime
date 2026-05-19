//! # Wasmtime's [wasi-keyvalue] Implementation
//!
//! This crate provides a Wasmtime host implementation of the [wasi-keyvalue]
//! API. With this crate, the runtime can run components that call APIs in
//! [wasi-keyvalue] and provide components with access to key-value storages.
//!
//! Currently supported storage backends:
//! * In-Memory (empty identifier `""`)
//! * redb — persistent embedded store (feature `redb`, identifier `"redb"` or `"redb:<bucket>"`)
//!
//! ## redb backend
//!
//! Enable with `features = ["redb"]`. Configure via [`WasiKeyValueCtxBuilder::redb_file`].
//! An identifier of `"redb"` opens the `"default"` bucket; `"redb:mybucket"` opens `"mybucket"`.
//! All buckets share one database file. The null-byte separator used internally (`\0`) is safe
//! because WIT identifiers cannot contain it.
//!
//! # Examples
//!
//! The usage of this crate is very similar to other WASI API implementations
//! such as [wasi:cli] and [wasi:http].
//!
//! A common scenario is accessing KV store in a [wasi:cli] component.
//! A standalone example of doing all this looks like:
//!
//! ```
//! use wasmtime::{
//!     component::{Linker, ResourceTable},
//!     Engine, Result, Store,
//! };
//! use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};
//! use wasmtime_wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtx, WasiKeyValueCtxBuilder};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let engine = Engine::default();
//!
//!     let mut store = Store::new(&engine, Ctx {
//!         table: ResourceTable::new(),
//!         wasi_ctx: WasiCtx::builder().build(),
//!         wasi_keyvalue_ctx: WasiKeyValueCtxBuilder::new().build(),
//!     });
//!
//!     let mut linker = Linker::<Ctx>::new(&engine);
//!     wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
//!     // add `wasi-keyvalue` world's interfaces to the linker
//!     wasmtime_wasi_keyvalue::add_to_linker(&mut linker, |h: &mut Ctx| {
//!         WasiKeyValue::new(&h.wasi_keyvalue_ctx, &mut h.table)
//!     })?;
//!
//!     // ... use `linker` to instantiate within `store` ...
//!
//!     Ok(())
//! }
//!
//! struct Ctx {
//!     table: ResourceTable,
//!     wasi_ctx: WasiCtx,
//!     wasi_keyvalue_ctx: WasiKeyValueCtx,
//! }
//!
//! impl WasiView for Ctx {
//!     fn ctx(&mut self) -> WasiCtxView<'_> {
//!         WasiCtxView { ctx: &mut self.wasi_ctx, table: &mut self.table }
//!     }
//! }
//! ```
//!
//! [wasi-keyvalue]: https://github.com/WebAssembly/wasi-keyvalue
//! [wasi:cli]: https://docs.rs/wasmtime-wasi/latest
//! [wasi:http]: https://docs.rs/wasmtime-wasi-http/latest

#![deny(missing_docs)]

mod generated {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "wasi:keyvalue/imports",
        imports: { default: trappable },
        with: {
            "wasi:keyvalue/store.bucket": crate::Bucket,
        },
        trappable_error_type: {
            "wasi:keyvalue/store.error" => crate::Error,
        },
    });
}

use self::generated::wasi::keyvalue;
use std::collections::HashMap;
use wasmtime::Result;
use wasmtime::component::{HasData, Resource, ResourceTable, ResourceTableError};

#[cfg(feature = "redb")]
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
#[cfg(feature = "redb")]
use std::sync::Arc;

/// Single redb table used by all buckets when the `redb` feature is enabled.
/// Keys are encoded as `{bucket_name}\0{user_key}`.
#[cfg(feature = "redb")]
const KV: TableDefinition<&str, &[u8]> = TableDefinition::new("wasi_kv");

#[doc(hidden)]
pub enum Error {
    NoSuchStore,
    AccessDenied,
    Other(String),
}

impl From<ResourceTableError> for Error {
    fn from(err: ResourceTableError) -> Self {
        Self::Other(err.to_string())
    }
}

#[cfg(feature = "redb")]
macro_rules! impl_from_redb_error {
    ($($t:ty),*) => {
        $(impl From<$t> for Error {
            fn from(e: $t) -> Self { Self::Other(e.to_string()) }
        })*
    }
}

#[cfg(feature = "redb")]
impl_from_redb_error!(
    redb::Error,
    redb::DatabaseError,
    redb::TableError,
    redb::StorageError,
    redb::TransactionError,
    redb::CommitError
);

#[doc(hidden)]
pub enum Bucket {
    InMemory(HashMap<String, Vec<u8>>),
    #[cfg(feature = "redb")]
    Redb(String),
}

/// Builder-style structure used to create a [`WasiKeyValueCtx`].
#[derive(Default)]
pub struct WasiKeyValueCtxBuilder {
    in_memory_data: HashMap<String, Vec<u8>>,
    #[cfg(feature = "redb")]
    redb: Option<Arc<Database>>,
}

impl WasiKeyValueCtxBuilder {
    /// Creates a builder for a new context with default parameters set.
    pub fn new() -> Self {
        Default::default()
    }

    /// Preset data for the In-Memory provider.
    pub fn in_memory_data<I, K, V>(mut self, data: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<Vec<u8>>,
    {
        self.in_memory_data = data
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }

    /// Open (or create) a redb database file at `path` for use with the `"redb"` identifier.
    ///
    /// Only available with the `redb` feature enabled.
    #[cfg(feature = "redb")]
    pub fn redb_file(mut self, path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let db = Database::create(path.as_ref())?;
        // Ensure the KV table exists.
        let txn = db.begin_write()?;
        txn.open_table(KV)?;
        txn.commit()?;
        self.redb = Some(Arc::new(db));
        Ok(self)
    }

    /// Uses the configured context so far to construct the final [`WasiKeyValueCtx`].
    pub fn build(self) -> WasiKeyValueCtx {
        WasiKeyValueCtx {
            in_memory_data: self.in_memory_data,
            #[cfg(feature = "redb")]
            redb: self.redb,
        }
    }
}

/// Capture the state necessary for use in the `wasi-keyvalue` API implementation.
pub struct WasiKeyValueCtx {
    in_memory_data: HashMap<String, Vec<u8>>,
    #[cfg(feature = "redb")]
    redb: Option<Arc<Database>>,
}

impl WasiKeyValueCtx {
    /// Convenience function for calling [`WasiKeyValueCtxBuilder::new`].
    pub fn builder() -> WasiKeyValueCtxBuilder {
        WasiKeyValueCtxBuilder::new()
    }
}

/// A wrapper capturing the needed internal `wasi-keyvalue` state.
pub struct WasiKeyValue<'a> {
    ctx: &'a WasiKeyValueCtx,
    table: &'a mut ResourceTable,
}

impl<'a> WasiKeyValue<'a> {
    /// Create a new view into the `wasi-keyvalue` state.
    pub fn new(ctx: &'a WasiKeyValueCtx, table: &'a mut ResourceTable) -> Self {
        Self { ctx, table }
    }
}

impl keyvalue::store::Host for WasiKeyValue<'_> {
    fn open(&mut self, identifier: String) -> Result<Resource<Bucket>, Error> {
        match identifier.as_str() {
            "" => Ok(self.table.push(Bucket::InMemory(
                self.ctx.in_memory_data.clone(),
            ))?),
            #[cfg(feature = "redb")]
            s if s == "redb" || s.starts_with("redb:") => {
                match &self.ctx.redb {
                    Some(_) => {
                        let bucket_name = if s == "redb" {
                            "default".to_string()
                        } else {
                            s["redb:".len()..].to_string()
                        };
                        Ok(self.table.push(Bucket::Redb(bucket_name))?)
                    }
                    None => Err(Error::NoSuchStore),
                }
            }
            _ => Err(Error::NoSuchStore),
        }
    }

    fn convert_error(&mut self, err: Error) -> Result<keyvalue::store::Error> {
        match err {
            Error::NoSuchStore => Ok(keyvalue::store::Error::NoSuchStore),
            Error::AccessDenied => Ok(keyvalue::store::Error::AccessDenied),
            Error::Other(e) => Ok(keyvalue::store::Error::Other(e)),
        }
    }
}

impl keyvalue::store::HostBucket for WasiKeyValue<'_> {
    fn get(&mut self, bucket: Resource<Bucket>, key: String) -> Result<Option<Vec<u8>>, Error> {
        match self.table.get_mut(&bucket)? {
            Bucket::InMemory(map) => Ok(map.get(&key).cloned()),
            #[cfg(feature = "redb")]
            Bucket::Redb(name) => {
                let encoded = redb_encode(name, &key);
                let db = self.ctx.redb.as_ref().unwrap();
                let txn = db.begin_read()?;
                let table = txn.open_table(KV)?;
                match table.get(encoded.as_str()).map_err(Error::from)? {
                    Some(g) => Ok(Some(g.value().to_vec())),
                    None => Ok(None),
                }
            }
        }
    }

    fn set(&mut self, bucket: Resource<Bucket>, key: String, value: Vec<u8>) -> Result<(), Error> {
        match self.table.get_mut(&bucket)? {
            Bucket::InMemory(map) => { map.insert(key, value); Ok(()) }
            #[cfg(feature = "redb")]
            Bucket::Redb(name) => {
                let encoded = redb_encode(name, &key);
                let db = self.ctx.redb.as_ref().unwrap();
                let txn = db.begin_write()?;
                { txn.open_table(KV)?.insert(encoded.as_str(), value.as_slice())?; }
                txn.commit()?;
                Ok(())
            }
        }
    }

    fn delete(&mut self, bucket: Resource<Bucket>, key: String) -> Result<(), Error> {
        match self.table.get_mut(&bucket)? {
            Bucket::InMemory(map) => { map.remove(&key); Ok(()) }
            #[cfg(feature = "redb")]
            Bucket::Redb(name) => {
                let encoded = redb_encode(name, &key);
                let db = self.ctx.redb.as_ref().unwrap();
                let txn = db.begin_write()?;
                { txn.open_table(KV)?.remove(encoded.as_str())?; }
                txn.commit()?;
                Ok(())
            }
        }
    }

    fn exists(&mut self, bucket: Resource<Bucket>, key: String) -> Result<bool, Error> {
        match self.table.get_mut(&bucket)? {
            Bucket::InMemory(map) => Ok(map.contains_key(&key)),
            #[cfg(feature = "redb")]
            Bucket::Redb(name) => {
                let encoded = redb_encode(name, &key);
                let db = self.ctx.redb.as_ref().unwrap();
                let txn = db.begin_read()?;
                let table = txn.open_table(KV)?;
                Ok(table.get(encoded.as_str()).map_err(Error::from)?.is_some())
            }
        }
    }

    fn list_keys(
        &mut self,
        bucket: Resource<Bucket>,
        cursor: Option<u64>,
    ) -> Result<keyvalue::store::KeyResponse, Error> {
        match self.table.get_mut(&bucket)? {
            Bucket::InMemory(map) => {
                let keys: Vec<String> = map.keys().cloned().collect();
                let cursor = cursor.unwrap_or(0) as usize;
                Ok(keyvalue::store::KeyResponse {
                    keys: keys[cursor..].to_vec(),
                    cursor: None,
                })
            }
            #[cfg(feature = "redb")]
            Bucket::Redb(name) => {
                let prefix = format!("{}\0", name);
                let end = format!("{}\x01", name);
                let prefix_len = prefix.len();
                let db = self.ctx.redb.as_ref().unwrap();
                let txn = db.begin_read()?;
                let table = txn.open_table(KV)?;
                let skip = cursor.unwrap_or(0) as usize;
                let mut keys = Vec::new();
                for entry in table.range(prefix.as_str()..end.as_str()).map_err(Error::from)?.skip(skip) {
                    let (k, _) = entry.map_err(Error::from)?;
                    keys.push(k.value()[prefix_len..].to_string());
                }
                Ok(keyvalue::store::KeyResponse { keys, cursor: None })
            }
        }
    }

    fn drop(&mut self, bucket: Resource<Bucket>) -> Result<()> {
        self.table.delete(bucket)?;
        Ok(())
    }
}

impl keyvalue::atomics::Host for WasiKeyValue<'_> {
    fn increment(
        &mut self,
        bucket: Resource<Bucket>,
        key: String,
        delta: u64,
    ) -> Result<u64, Error> {
        match self.table.get_mut(&bucket)? {
            Bucket::InMemory(map) => {
                let value = map.entry(key.clone()).or_insert("0".to_string().into_bytes());
                let current_value = String::from_utf8(value.clone())
                    .map_err(|e| Error::Other(e.to_string()))?
                    .parse::<u64>()
                    .map_err(|e| Error::Other(e.to_string()))?;
                let new_value = current_value + delta;
                *value = new_value.to_string().into_bytes();
                Ok(new_value)
            }
            #[cfg(feature = "redb")]
            Bucket::Redb(name) => {
                let encoded = redb_encode(name, &key);
                let db = self.ctx.redb.as_ref().unwrap();
                let txn = db.begin_write()?;
                let new_value = {
                    let mut table = txn.open_table(KV)?;
                    let current: u64 = match table.get(encoded.as_str()).map_err(Error::from)? {
                        Some(g) => std::str::from_utf8(g.value()).ok()
                            .and_then(|s| s.parse().ok()).unwrap_or(0),
                        None => 0,
                    };
                    let next = current.saturating_add(delta);
                    table.insert(encoded.as_str(), next.to_string().as_bytes())?;
                    next
                };
                txn.commit()?;
                Ok(new_value)
            }
        }
    }
}

impl keyvalue::batch::Host for WasiKeyValue<'_> {
    fn get_many(
        &mut self,
        bucket: Resource<Bucket>,
        keys: Vec<String>,
    ) -> Result<Vec<Option<(String, Vec<u8>)>>, Error> {
        match self.table.get_mut(&bucket)? {
            Bucket::InMemory(map) => Ok(keys.into_iter()
                .map(|key| map.get(&key).map(|v| (key.clone(), v.clone())))
                .collect()),
            #[cfg(feature = "redb")]
            Bucket::Redb(name) => {
                let db = self.ctx.redb.as_ref().unwrap();
                let txn = db.begin_read()?;
                let table = txn.open_table(KV)?;
                let mut results = Vec::with_capacity(keys.len());
                for key in keys {
                    let encoded = redb_encode(name, &key);
                    let entry = match table.get(encoded.as_str()).map_err(Error::from)? {
                        Some(g) => Some((key.clone(), g.value().to_vec())),
                        None => None,
                    };
                    results.push(entry);
                }
                Ok(results)
            }
        }
    }

    fn set_many(
        &mut self,
        bucket: Resource<Bucket>,
        key_values: Vec<(String, Vec<u8>)>,
    ) -> Result<(), Error> {
        match self.table.get_mut(&bucket)? {
            Bucket::InMemory(map) => {
                for (key, value) in key_values { map.insert(key, value); }
                Ok(())
            }
            #[cfg(feature = "redb")]
            Bucket::Redb(name) => {
                let db = self.ctx.redb.as_ref().unwrap();
                let txn = db.begin_write()?;
                {
                    let mut table = txn.open_table(KV)?;
                    for (key, value) in &key_values {
                        let encoded = redb_encode(name, key);
                        table.insert(encoded.as_str(), value.as_slice())?;
                    }
                }
                txn.commit()?;
                Ok(())
            }
        }
    }

    fn delete_many(&mut self, bucket: Resource<Bucket>, keys: Vec<String>) -> Result<(), Error> {
        match self.table.get_mut(&bucket)? {
            Bucket::InMemory(map) => {
                for key in keys { map.remove(&key); }
                Ok(())
            }
            #[cfg(feature = "redb")]
            Bucket::Redb(name) => {
                let db = self.ctx.redb.as_ref().unwrap();
                let txn = db.begin_write()?;
                {
                    let mut table = txn.open_table(KV)?;
                    for key in &keys {
                        let encoded = redb_encode(name, key);
                        table.remove(encoded.as_str())?;
                    }
                }
                txn.commit()?;
                Ok(())
            }
        }
    }
}

#[cfg(feature = "redb")]
fn redb_encode(bucket: &str, key: &str) -> String {
    format!("{}\0{}", bucket, key)
}

/// Add all the `wasi-keyvalue` world's interfaces to a [`wasmtime::component::Linker`].
pub fn add_to_linker<T: Send + 'static>(
    l: &mut wasmtime::component::Linker<T>,
    f: fn(&mut T) -> WasiKeyValue<'_>,
) -> Result<()> {
    keyvalue::store::add_to_linker::<_, HasWasiKeyValue>(l, f)?;
    keyvalue::atomics::add_to_linker::<_, HasWasiKeyValue>(l, f)?;
    keyvalue::batch::add_to_linker::<_, HasWasiKeyValue>(l, f)?;
    Ok(())
}

struct HasWasiKeyValue;

impl HasData for HasWasiKeyValue {
    type Data<'a> = WasiKeyValue<'a>;
}
