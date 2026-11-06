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
use wasmtime::component::{HasData, HostHeapUsage, Resource, ResourceTable, ResourceTableError};

#[doc(hidden)]
#[derive(Debug)]
pub enum Error {
    NoSuchStore,
    AccessDenied,
    Other(String),
    ResourceTable(ResourceTableError),
}

impl From<ResourceTableError> for Error {
    fn from(err: ResourceTableError) -> Self {
        Self::ResourceTable(err)
    }
}

#[doc(hidden)]
pub struct Bucket {
    in_memory_data: HashMap<String, Vec<u8>>,
    // Cached heap bytes of keys and values, so insert/remove stay O(1)
    // instead of rescanning `in_memory_data` on every update.
    child_bytes: usize,
}

impl Bucket {
    fn new(in_memory_data: HashMap<String, Vec<u8>>) -> Self {
        let child_bytes = in_memory_data
            .iter()
            .map(|(k, v)| k.heap_size_of_children() + v.heap_size_of_children())
            .sum();
        Self {
            in_memory_data,
            child_bytes,
        }
    }

    fn insert(&mut self, key: String, value: Vec<u8>) {
        let key_bytes = key.heap_size_of_children();
        self.child_bytes += value.heap_size_of_children();
        match self.in_memory_data.insert(key, value) {
            Some(old) => self.child_bytes -= old.heap_size_of_children(),
            None => self.child_bytes += key_bytes,
        }
    }

    fn remove(&mut self, key: &str) {
        if let Some((old_k, old_v)) = self.in_memory_data.remove_entry(key) {
            self.child_bytes -= old_k.heap_size_of_children() + old_v.heap_size_of_children();
        }
    }
}

impl HostHeapUsage for Bucket {
    fn heap_size_of_children(&self) -> usize {
        let table_bytes =
            self.in_memory_data.capacity() * (size_of::<String>() + size_of::<Vec<u8>>() + 1);
        table_bytes + self.child_bytes
    }
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
            "" => Ok(self
                .table
                .push(Bucket::new(self.ctx.in_memory_data.clone()))?),
            _ => Err(Error::NoSuchStore),
        }
    }

    fn convert_error(&mut self, err: Error) -> Result<keyvalue::store::Error> {
        match err {
            Error::NoSuchStore => Ok(keyvalue::store::Error::NoSuchStore),
            Error::AccessDenied => Ok(keyvalue::store::Error::AccessDenied),
            Error::Other(e) => Ok(keyvalue::store::Error::Other(e)),
            Error::ResourceTable(e) => Err(e.into()),
        }
    }
}

impl keyvalue::store::HostBucket for WasiKeyValue<'_> {
    fn get(&mut self, bucket: Resource<Bucket>, key: String) -> Result<Option<Vec<u8>>, Error> {
        let bucket = self.table.get(&bucket)?;
        Ok(bucket.in_memory_data.get(&key).cloned())
    }

    fn set(&mut self, bucket: Resource<Bucket>, key: String, value: Vec<u8>) -> Result<(), Error> {
        self.table.update(&bucket, |b| {
            b.insert(key, value);
        })?;
        Ok(())
    }

    fn delete(&mut self, bucket: Resource<Bucket>, key: String) -> Result<(), Error> {
        self.table.update(&bucket, |b| {
            b.remove(&key);
        })?;
        Ok(())
    }

    fn exists(&mut self, bucket: Resource<Bucket>, key: String) -> Result<bool, Error> {
        let bucket = self.table.get(&bucket)?;
        Ok(bucket.in_memory_data.contains_key(&key))
    }

    fn list_keys(
        &mut self,
        bucket: Resource<Bucket>,
        cursor: Option<u64>,
    ) -> Result<keyvalue::store::KeyResponse, Error> {
        let bucket = self.table.get(&bucket)?;
        let keys: Vec<String> = bucket.in_memory_data.keys().cloned().collect();
        // `cursor` is guest-controlled, so clamp it to the number of keys before
        // slicing. `try_from` also guards against a `u64` that does not fit in a
        // `usize` on 32-bit hosts, which the old `as usize` cast would truncate.
        let cursor = usize::try_from(cursor.unwrap_or(0))
            .unwrap_or(usize::MAX)
            .min(keys.len());
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
        self.table.update(&bucket, |bucket| {
            let current_bytes = bucket
                .in_memory_data
                .get(&key)
                .map_or(&b"0"[..], Vec::as_slice);
            let current_value = std::str::from_utf8(current_bytes)
                .map_err(|e| Error::Other(e.to_string()))?
                .parse::<u64>()
                .map_err(|e| Error::Other(e.to_string()))?;
            let new_value = current_value
                .checked_add(delta)
                .ok_or_else(|| Error::Other("overflow incrementing value".to_string()))?;
            bucket.insert(key, new_value.to_string().into_bytes());
            Ok(new_value)
        })?
    }
}

impl keyvalue::batch::Host for WasiKeyValue<'_> {
    fn get_many(
        &mut self,
        bucket: Resource<Bucket>,
        keys: Vec<String>,
    ) -> Result<Vec<Option<(String, Vec<u8>)>>, Error> {
        let bucket = self.table.get(&bucket)?;
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
        self.table.update(&bucket, |b| {
            for (key, value) in key_values {
                b.insert(key, value);
            }
        })?;
        Ok(())
    }

    fn delete_many(&mut self, bucket: Resource<Bucket>, keys: Vec<String>) -> Result<(), Error> {
        self.table.update(&bucket, |b| {
            for key in keys {
                b.remove(&key);
            }
        })?;
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use keyvalue::atomics::Host as _;
    use keyvalue::store::{Host as _, HostBucket as _};
    use wasmtime::component::ResourceTable;

    fn ctx_with(data: &[(&str, &str)]) -> WasiKeyValueCtx {
        WasiKeyValueCtxBuilder::new()
            .in_memory_data(data.iter().map(|(k, v)| (k.to_string(), v.to_string())))
            .build()
    }

    #[test]
    fn list_keys_cursor_past_end_returns_empty() {
        let ctx = ctx_with(&[("k", "v")]);
        let mut table = ResourceTable::new();
        let mut kv = WasiKeyValue::new(&ctx, &mut table);
        let bucket = match kv.open(String::new()) {
            Ok(bucket) => bucket,
            Err(_) => panic!("open failed"),
        };
        // A guest-supplied cursor past the last key must not panic.
        match kv.list_keys(bucket, Some(1_000)) {
            Ok(resp) => assert!(resp.keys.is_empty()),
            Err(_) => panic!("list_keys returned an error"),
        }
    }

    #[test]
    fn increment_overflow_is_reported_as_error() {
        let ctx = ctx_with(&[("c", "18446744073709551615")]); // u64::MAX
        let mut table = ResourceTable::new();
        let mut kv = WasiKeyValue::new(&ctx, &mut table);
        let bucket = match kv.open(String::new()) {
            Ok(bucket) => bucket,
            Err(_) => panic!("open failed"),
        };
        assert!(kv.increment(bucket, "c".to_string(), 1).is_err());
    }

    #[test]
    fn set_exceeds_host_heap_limit() {
        let ctx = ctx_with(&[("k", "v")]);
        let mut table = ResourceTable::new();
        let mut kv = WasiKeyValue::new(&ctx, &mut table);
        let bucket = kv.open(String::new()).expect("open failed");
        let baseline = kv.table.current_host_heap_usage();
        kv.table.set_max_host_heap_usage(Some(baseline + 16));

        let borrow = Resource::new_borrow(bucket.rep());
        kv.set(borrow, "a".to_string(), b"1".to_vec())
            .expect("small set within limit");

        let borrow = Resource::new_borrow(bucket.rep());
        let err = kv
            .set(borrow, "large".to_string(), vec![b'x'; 64])
            .err()
            .expect("expected limit error");
        assert!(matches!(
            err,
            Error::ResourceTable(ResourceTableError::HostMemoryLimitExceeded)
        ));
    }
}
