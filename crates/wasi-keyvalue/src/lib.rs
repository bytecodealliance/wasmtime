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
        named_imports: {
            "wasi:keyvalue/store": wasmtime_wasi::NamedId,
            "wasi:keyvalue/atomics": wasmtime_wasi::NamedId,
            "wasi:keyvalue/batch": wasmtime_wasi::NamedId,
        },
    });
}

use self::generated::wasi::keyvalue;
use std::collections::HashMap;
use std::marker;
use wasmtime::Result;
use wasmtime::component::{
    Component, HasData, Linker, Resource, ResourceTable, ResourceTableError,
};
use wasmtime_wasi::{NamedId, WasiCtxNamedView};

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
        let bucket = self.table.get_mut(&bucket)?;
        let value = bucket
            .in_memory_data
            .entry(key.clone())
            .or_insert("0".to_string().into_bytes());
        let current_value = String::from_utf8(value.clone())
            .map_err(|e| Error::Other(e.to_string()))?
            .parse::<u64>()
            .map_err(|e| Error::Other(e.to_string()))?;
        let new_value = current_value
            .checked_add(delta)
            .ok_or_else(|| Error::Other("overflow incrementing value".to_string()))?;
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

/// A trait used to look up a specific `wasi-keyvalue` context for a named
/// import.
///
/// This is used by [`add_named_to_linker`] to resolve a [`NamedId`], allocated
/// by the embedder, to the [`WasiKeyValue`] that a particular named import
/// operates on.
///
/// Resources such as buckets can flow between named and unnamed imports, so
/// the [`ResourceTable`] referenced by the returned [`WasiKeyValue`] should be
/// the same table that's used for all other imports.
///
/// # Examples
///
/// ```no_run
/// use std::collections::HashMap;
/// use wasmtime::component::{Component, Linker, ResourceTable};
/// use wasmtime::{Engine, Result};
/// use wasmtime_wasi::NamedId;
/// use wasmtime_wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtx, WasiKeyValueNamedView};
///
/// struct MyState {
///     table: ResourceTable,
///     stores: Vec<WasiKeyValueCtx>,
/// }
///
/// impl WasiKeyValueNamedView for MyState {
///     fn keyvalue(&mut self, id: NamedId) -> WasiKeyValue<'_> {
///         WasiKeyValue::new(&self.stores[id.0], &mut self.table)
///     }
/// }
///
/// fn main() -> Result<()> {
///     let engine = Engine::default();
///     let component = Component::from_file(&engine, "my-component.wasm")?;
///     let mut linker = Linker::<MyState>::new(&engine);
///
///     let mut names = HashMap::new();
///     wasmtime_wasi_keyvalue::add_named_to_linker(&mut linker, &component, |_, name| {
///         let len = names.len();
///         Ok(NamedId(*names.entry(name.to_string()).or_insert(len)))
///     })?;
///
///     // ... create a `WasiKeyValueCtx` per entry in `names` and instantiate ...
///
///     Ok(())
/// }
/// ```
pub trait WasiKeyValueNamedView: Send + 'static {
    /// Returns the [`WasiKeyValue`] to use for the named import identified
    /// by `id`.
    fn keyvalue(&mut self, id: NamedId) -> WasiKeyValue<'_>;
}

struct WasiKeyValueNamed<T>(marker::PhantomData<fn() -> T>);

impl<T: WasiKeyValueNamedView> HasData for WasiKeyValueNamed<T> {
    type Data<'a> = WasiCtxNamedView<'a, T>;
}

/// Interfaces that are added via [`add_named_to_linker`].
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Interface {
    /// `wasi:keyvalue/store`
    KeyvalueStore,
    /// `wasi:keyvalue/atomics`
    KeyvalueAtomics,
    /// `wasi:keyvalue/batch`
    KeyvalueBatch,
}

/// Add all the `wasi-keyvalue` world's interfaces to `linker` for any named
/// imports of them that `component` has.
///
/// This is the named-import counterpart of [`add_to_linker`]: `component`'s
/// imports are inspected and, for each import that implements a
/// `wasi:keyvalue` interface under a different name, `lookup` is invoked with
/// the [`Interface`] being bound and the name of the import. The [`NamedId`]
/// returned is later passed to [`WasiKeyValueNamedView::keyvalue`] when that
/// import is used. If `lookup` returns an error then this function returns
/// that error.
pub fn add_named_to_linker<T>(
    linker: &mut Linker<T>,
    component: &Component,
    mut lookup: impl FnMut(Interface, &str) -> Result<NamedId>,
) -> Result<()>
where
    T: WasiKeyValueNamedView,
{
    use self::generated::named_imports::wasi::keyvalue::{atomics, batch, store};

    store::add_to_linker::<_, WasiKeyValueNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::KeyvalueStore, name),
        |x| WasiCtxNamedView(x),
    )?;
    atomics::add_to_linker::<_, WasiKeyValueNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::KeyvalueAtomics, name),
        |x| WasiCtxNamedView(x),
    )?;
    batch::add_to_linker::<_, WasiKeyValueNamed<T>>(
        linker,
        component,
        |name| lookup(Interface::KeyvalueBatch, name),
        |x| WasiCtxNamedView(x),
    )?;
    Ok(())
}

mod named {
    use super::generated::named_imports::wasi::keyvalue::{atomics, batch, store};
    use super::keyvalue::store::KeyResponse;
    use super::{Bucket, Error, WasiKeyValueNamedView, keyvalue};
    use wasmtime::Result;
    use wasmtime::component::Resource;
    use wasmtime_wasi::{NamedId, WasiCtxNamedView};

    impl<T: WasiKeyValueNamedView> store::Host for WasiCtxNamedView<'_, T> {
        fn open(&mut self, id: NamedId, identifier: String) -> Result<Resource<Bucket>, Error> {
            keyvalue::store::Host::open(&mut self.0.keyvalue(id), identifier)
        }

        fn convert_error(&mut self, id: NamedId, err: Error) -> Result<keyvalue::store::Error> {
            keyvalue::store::Host::convert_error(&mut self.0.keyvalue(id), err)
        }
    }

    impl<T: WasiKeyValueNamedView> store::HostBucket for WasiCtxNamedView<'_, T> {
        fn get(
            &mut self,
            id: NamedId,
            bucket: Resource<Bucket>,
            key: String,
        ) -> Result<Option<Vec<u8>>, Error> {
            keyvalue::store::HostBucket::get(&mut self.0.keyvalue(id), bucket, key)
        }

        fn set(
            &mut self,
            id: NamedId,
            bucket: Resource<Bucket>,
            key: String,
            value: Vec<u8>,
        ) -> Result<(), Error> {
            keyvalue::store::HostBucket::set(&mut self.0.keyvalue(id), bucket, key, value)
        }

        fn delete(
            &mut self,
            id: NamedId,
            bucket: Resource<Bucket>,
            key: String,
        ) -> Result<(), Error> {
            keyvalue::store::HostBucket::delete(&mut self.0.keyvalue(id), bucket, key)
        }

        fn exists(
            &mut self,
            id: NamedId,
            bucket: Resource<Bucket>,
            key: String,
        ) -> Result<bool, Error> {
            keyvalue::store::HostBucket::exists(&mut self.0.keyvalue(id), bucket, key)
        }

        fn list_keys(
            &mut self,
            id: NamedId,
            bucket: Resource<Bucket>,
            cursor: Option<u64>,
        ) -> Result<KeyResponse, Error> {
            keyvalue::store::HostBucket::list_keys(&mut self.0.keyvalue(id), bucket, cursor)
        }

        fn drop(&mut self, id: NamedId, bucket: Resource<Bucket>) -> Result<()> {
            keyvalue::store::HostBucket::drop(&mut self.0.keyvalue(id), bucket)
        }
    }

    impl<T: WasiKeyValueNamedView> atomics::Host for WasiCtxNamedView<'_, T> {
        fn increment(
            &mut self,
            id: NamedId,
            bucket: Resource<Bucket>,
            key: String,
            delta: u64,
        ) -> Result<u64, Error> {
            keyvalue::atomics::Host::increment(&mut self.0.keyvalue(id), bucket, key, delta)
        }

        fn convert_error(&mut self, id: NamedId, err: Error) -> Result<keyvalue::store::Error> {
            keyvalue::store::Host::convert_error(&mut self.0.keyvalue(id), err)
        }
    }

    impl<T: WasiKeyValueNamedView> batch::Host for WasiCtxNamedView<'_, T> {
        fn get_many(
            &mut self,
            id: NamedId,
            bucket: Resource<Bucket>,
            keys: Vec<String>,
        ) -> Result<Vec<Option<(String, Vec<u8>)>>, Error> {
            keyvalue::batch::Host::get_many(&mut self.0.keyvalue(id), bucket, keys)
        }

        fn set_many(
            &mut self,
            id: NamedId,
            bucket: Resource<Bucket>,
            key_values: Vec<(String, Vec<u8>)>,
        ) -> Result<(), Error> {
            keyvalue::batch::Host::set_many(&mut self.0.keyvalue(id), bucket, key_values)
        }

        fn delete_many(
            &mut self,
            id: NamedId,
            bucket: Resource<Bucket>,
            keys: Vec<String>,
        ) -> Result<(), Error> {
            keyvalue::batch::Host::delete_many(&mut self.0.keyvalue(id), bucket, keys)
        }

        fn convert_error(&mut self, id: NamedId, err: Error) -> Result<keyvalue::store::Error> {
            keyvalue::store::Host::convert_error(&mut self.0.keyvalue(id), err)
        }
    }
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
}
