//! # Wasmtime's [wasi-keyvalue] Implementation
//!
//! This crate provides a Wasmtime host implementation of the [wasi-keyvalue]
//! API. With this crate, the runtime can run components that call APIs in
//! [wasi-keyvalue] and provide components with access to key-value storages.
//!
//! Currently supported storage backends:
//! * In-Memory (empty identifier)
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
//!     Config, Engine, Result, Store,
//! };
//! use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};
//! use wasmtime_wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtx, WasiKeyValueCtxBuilder};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut config = Config::new();
//!     config.async_support(true);
//!     let engine = Engine::new(&config)?;
//!
//!     let mut store = Store::new(&engine, Ctx {
//!         table: ResourceTable::new(),
//!         wasi_ctx: WasiCtxBuilder::new().build(),
//!         wasi_keyvalue_ctx: WasiKeyValueCtxBuilder::new().build(),
//!     });
//!
//!     let mut linker = Linker::<Ctx>::new(&engine);
//!     wasmtime_wasi::add_to_linker_async(&mut linker)?;
//!     // add `wasi-runtime-config` world's interfaces to the linker
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
//!     fn table(&mut self) -> &mut ResourceTable { &mut self.table }
//!     fn ctx(&mut self) -> &mut WasiCtx { &mut self.wasi_ctx }
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
        trappable_imports: true,
        with: {
            "wasi:keyvalue/store/bucket": crate::Bucket,
        },
        trappable_error_type: {
            "wasi:keyvalue/store/error" => crate::Error,
        },
    });
}

use self::generated::wasi::keyvalue;
use anyhow::Result;
use std::collections::HashMap;
use wasmtime::component::{Resource, ResourceTable, ResourceTableError};

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

#[doc(hidden)]
pub struct Bucket {
    in_memory_data: HashMap<String, Vec<u8>>,
}

/// Builder-style structure used to create a [`WasiKeyValueCtx`].
#[derive(Default)]
pub struct WasiKeyValueCtxBuilder {
    in_memory_data: HashMap<String, Vec<u8>>,
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

    /// Uses the configured context so far to construct the final [`WasiKeyValueCtx`].
    pub fn build(self) -> WasiKeyValueCtx {
        WasiKeyValueCtx {
            in_memory_data: self.in_memory_data,
        }
    }
}

/// Capture the state necessary for use in the `wasi-keyvalue` API implementation.
pub struct WasiKeyValueCtx {
    in_memory_data: HashMap<String, Vec<u8>>,
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
            "" => Ok(self.table.push(Bucket {
                in_memory_data: self.ctx.in_memory_data.clone(),
            })?),
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
        let bucket = self.table.get_mut(&bucket)?;
        Ok(bucket.in_memory_data.get(&key).cloned())
    }

    fn set(&mut self, bucket: Resource<Bucket>, key: String, value: Vec<u8>) -> Result<(), Error> {
        let bucket = self.table.get_mut(&bucket)?;
        bucket.in_memory_data.insert(key, value);
        Ok(())
    }

    fn delete(&mut self, bucket: Resource<Bucket>, key: String) -> Result<(), Error> {
        let bucket = self.table.get_mut(&bucket)?;
        bucket.in_memory_data.remove(&key);
        Ok(())
    }

    fn exists(&mut self, bucket: Resource<Bucket>, key: String) -> Result<bool, Error> {
        let bucket = self.table.get_mut(&bucket)?;
        Ok(bucket.in_memory_data.contains_key(&key))
    }

    fn list_keys(
        &mut self,
        bucket: Resource<Bucket>,
        cursor: Option<u64>,
    ) -> Result<keyvalue::store::KeyResponse, Error> {
        let bucket = self.table.get_mut(&bucket)?;
        let keys: Vec<String> = bucket.in_memory_data.keys().cloned().collect();
        let cursor = cursor.unwrap_or(0) as usize;
        let keys_slice = &keys[cursor..];
        Ok(keyvalue::store::KeyResponse {
            keys: keys_slice.to_vec(),
            cursor: None,
        })
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
        let bucket = self.table.get_mut(&bucket)?;
        let value = bucket
            .in_memory_data
            .entry(key.clone())
            .or_insert("0".to_string().into_bytes());
        let current_value = String::from_utf8(value.clone())
            .map_err(|e| Error::Other(e.to_string()))?
            .parse::<u64>()
            .map_err(|e| Error::Other(e.to_string()))?;
        let new_value = current_value + delta;
        *value = new_value.to_string().into_bytes();
        Ok(new_value)
    }
}

impl keyvalue::batch::Host for WasiKeyValue<'_> {
    fn get_many(
        &mut self,
        bucket: Resource<Bucket>,
        keys: Vec<String>,
    ) -> Result<Vec<Option<(String, Vec<u8>)>>, Error> {
        let bucket = self.table.get_mut(&bucket)?;
        Ok(keys
            .into_iter()
            .map(|key| {
                bucket
                    .in_memory_data
                    .get(&key)
                    .map(|value| (key.clone(), value.clone()))
            })
            .collect())
    }

    fn set_many(
        &mut self,
        bucket: Resource<Bucket>,
        key_values: Vec<(String, Vec<u8>)>,
    ) -> Result<(), Error> {
        let bucket = self.table.get_mut(&bucket)?;
        for (key, value) in key_values {
            bucket.in_memory_data.insert(key, value);
        }
        Ok(())
    }

    fn delete_many(&mut self, bucket: Resource<Bucket>, keys: Vec<String>) -> Result<(), Error> {
        let bucket = self.table.get_mut(&bucket)?;
        for key in keys {
            bucket.in_memory_data.remove(&key);
        }
        Ok(())
    }
}

/// Add all the `wasi-keyvalue` world's interfaces to a [`wasmtime::component::Linker`].
pub fn add_to_linker<T: Send>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl Fn(&mut T) -> WasiKeyValue<'_> + Send + Sync + Copy + 'static,
) -> Result<()> {
    keyvalue::store::add_to_linker_get_host(l, f)?;
    keyvalue::atomics::add_to_linker_get_host(l, f)?;
    keyvalue::batch::add_to_linker_get_host(l, f)?;
    Ok(())
}
