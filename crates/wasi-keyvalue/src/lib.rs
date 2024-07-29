//! # Wasmtime's [wasi-keyvalue] Implementation
//!
//! This crate provides a Wasmtime host implementation of the [wasi-keyvalue]
//! API. With this crate, the runtime can run components that call APIs in
//! [wasi-keyvalue] and provide components with access to key-value storages.
//!
//! Currently supported storage backends:
//! * In-Memory (empty identifier)
//! * Redis, supported identifier format:
//!   * `redis://[<username>][:<password>@]<hostname>[:port][/<db>]`
//!   * `redis+unix:///<path>[?db=<db>[&pass=<password>][&user=<username>]]`
//!
//! # Examples
//!
//! The usage of this crate is very similar to other WASI API implementations
//! such as [wasi:cli] and [wasi:http].
//!
//! A common scenario is accessing redis in a [wasi:cli] component.
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
//!     wasmtime_wasi_keyvalue::add_to_linker_async(&mut linker, |h: &mut Ctx| {
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

mod bindings;
mod provider;

use self::bindings::{sync::wasi::keyvalue as keyvalue_sync, wasi::keyvalue};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::fmt::Display;
use url::Url;
use wasmtime::component::{Resource, ResourceTable, ResourceTableError};
use wasmtime_wasi::runtime::in_tokio;

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

pub(crate) fn to_other_error(e: impl Display) -> Error {
    Error::Other(e.to_string())
}

#[doc(hidden)]
pub struct Bucket {
    inner: Box<dyn Host + Send>,
}

#[async_trait]
trait Host {
    async fn get(&mut self, key: String) -> Result<Option<Vec<u8>>, Error>;

    async fn set(&mut self, key: String, value: Vec<u8>) -> Result<(), Error>;

    async fn delete(&mut self, key: String) -> Result<(), Error>;

    async fn exists(&mut self, key: String) -> Result<bool, Error>;

    async fn list_keys(
        &mut self,
        cursor: Option<u64>,
    ) -> Result<keyvalue::store::KeyResponse, Error>;

    async fn increment(&mut self, key: String, delta: u64) -> Result<u64, Error>;

    async fn get_many(
        &mut self,
        keys: Vec<String>,
    ) -> Result<Vec<Option<(String, Vec<u8>)>>, Error>;

    async fn set_many(&mut self, key_values: Vec<(String, Vec<u8>)>) -> Result<(), Error>;

    async fn delete_many(&mut self, keys: Vec<String>) -> Result<(), Error>;
}

/// Builder-style structure used to create a [`WasiKeyValueCtx`].
#[derive(Default)]
pub struct WasiKeyValueCtxBuilder {
    in_memory_data: HashMap<String, Vec<u8>>,
    #[cfg(feature = "redis")]
    allowed_redis_hosts: Vec<String>,
    #[cfg(feature = "redis")]
    redis_connection_timeout: Option<std::time::Duration>,
    #[cfg(feature = "redis")]
    redis_response_timeout: Option<std::time::Duration>,
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

    /// Appends a list of Redis hosts to the allow-listed set each component gets
    /// access to. It can be in the format `<hostname>[:port]` or a unix domain
    /// socket path.
    ///
    /// # Examples
    ///
    /// ```
    /// use wasmtime_wasi_keyvalue::WasiKeyValueCtxBuilder;
    ///
    /// # fn main() {
    /// let ctx = WasiKeyValueCtxBuilder::new()
    ///     .allow_redis_hosts(&["localhost:1234", "/var/run/redis.sock"])
    ///     .build();
    /// # }
    /// ```
    #[cfg(feature = "redis")]
    pub fn allow_redis_hosts(mut self, hosts: &[impl AsRef<str>]) -> Self {
        self.allowed_redis_hosts
            .extend(hosts.iter().map(|h| h.as_ref().to_owned()));
        self
    }

    /// Sets the connection timeout parameter for the Redis provider.
    #[cfg(feature = "redis")]
    pub fn redis_connection_timeout(mut self, t: std::time::Duration) -> Self {
        self.redis_connection_timeout = Some(t);
        self
    }

    /// Sets the response timeout parameter for the Redis provider.
    #[cfg(feature = "redis")]
    pub fn redis_response_timeout(mut self, t: std::time::Duration) -> Self {
        self.redis_response_timeout = Some(t);
        self
    }

    /// Uses the configured context so far to construct the final [`WasiKeyValueCtx`].
    pub fn build(self) -> WasiKeyValueCtx {
        WasiKeyValueCtx {
            in_memory_data: self.in_memory_data,
            #[cfg(feature = "redis")]
            allowed_redis_hosts: self.allowed_redis_hosts,
            #[cfg(feature = "redis")]
            redis_connection_timeout: self.redis_connection_timeout,
            #[cfg(feature = "redis")]
            redis_response_timeout: self.redis_response_timeout,
        }
    }
}

/// Capture the state necessary for use in the `wasi-keyvalue` API implementation.
pub struct WasiKeyValueCtx {
    in_memory_data: HashMap<String, Vec<u8>>,
    #[cfg(feature = "redis")]
    allowed_redis_hosts: Vec<String>,
    #[cfg(feature = "redis")]
    redis_connection_timeout: Option<std::time::Duration>,
    #[cfg(feature = "redis")]
    redis_response_timeout: Option<std::time::Duration>,
}

impl WasiKeyValueCtx {
    /// Convenience function for calling [`WasiKeyValueCtxBuilder::new`].
    pub fn builder() -> WasiKeyValueCtxBuilder {
        WasiKeyValueCtxBuilder::new()
    }

    #[cfg(feature = "redis")]
    fn allow_redis_host(&self, u: &Url) -> bool {
        let host = match u.host() {
            Some(h) => match u.port() {
                Some(port) => format!("{}:{}", h, port),
                None => h.to_string(),
            },
            // unix domain socket path
            None => u.path().to_string(),
        };
        self.allowed_redis_hosts.contains(&host)
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

#[async_trait]
impl keyvalue::store::Host for WasiKeyValue<'_> {
    async fn open(&mut self, identifier: String) -> Result<Resource<Bucket>, Error> {
        if identifier == "" {
            return Ok(self.table.push(Bucket {
                inner: Box::new(provider::inmemory::InMemory::new(
                    self.ctx.in_memory_data.clone(),
                )),
            })?);
        }

        let u = Url::parse(&identifier).map_err(to_other_error)?;
        match u.scheme() {
            "redis" | "redis+unix" => {
                #[cfg(not(feature = "redis"))]
                {
                    return Err(Error::Other(
                        "Cannot enable Redis support when the crate is not compiled with this feature."
                            .to_string(),
                    ));
                }
                #[cfg(feature = "redis")]
                {
                    if !self.ctx.allow_redis_host(&u) {
                        return Err(Error::Other(format!(
                            "the identifier {} is not in the allowed list",
                            identifier
                        )));
                    }

                    let host = provider::redis::open(
                        identifier,
                        self.ctx.redis_response_timeout,
                        self.ctx.redis_connection_timeout,
                    )
                    .await?;
                    Ok(self.table.push(Bucket {
                        inner: Box::new(host),
                    })?)
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

#[async_trait]
impl keyvalue::store::HostBucket for WasiKeyValue<'_> {
    async fn get(
        &mut self,
        bucket: Resource<Bucket>,
        key: String,
    ) -> Result<Option<Vec<u8>>, Error> {
        let bucket = self.table.get_mut(&bucket)?;
        bucket.inner.get(key).await
    }

    async fn set(
        &mut self,
        bucket: Resource<Bucket>,
        key: String,
        value: Vec<u8>,
    ) -> Result<(), Error> {
        let bucket = self.table.get_mut(&bucket)?;
        bucket.inner.set(key, value).await
    }

    async fn delete(&mut self, bucket: Resource<Bucket>, key: String) -> Result<(), Error> {
        let bucket = self.table.get_mut(&bucket)?;
        bucket.inner.delete(key).await
    }

    async fn exists(&mut self, bucket: Resource<Bucket>, key: String) -> Result<bool, Error> {
        let bucket = self.table.get_mut(&bucket)?;
        bucket.inner.exists(key).await
    }

    async fn list_keys(
        &mut self,
        bucket: Resource<Bucket>,
        cursor: Option<u64>,
    ) -> Result<keyvalue::store::KeyResponse, Error> {
        let bucket = self.table.get_mut(&bucket)?;
        bucket.inner.list_keys(cursor).await
    }

    fn drop(&mut self, bucket: Resource<Bucket>) -> Result<()> {
        self.table.delete(bucket)?;
        Ok(())
    }
}

#[async_trait]
impl keyvalue::atomics::Host for WasiKeyValue<'_> {
    async fn increment(
        &mut self,
        bucket: Resource<Bucket>,
        key: String,
        delta: u64,
    ) -> Result<u64, Error> {
        let bucket = self.table.get_mut(&bucket)?;
        bucket.inner.increment(key, delta).await
    }
}

#[async_trait]
impl keyvalue::batch::Host for WasiKeyValue<'_> {
    async fn get_many(
        &mut self,
        bucket: Resource<Bucket>,
        keys: Vec<String>,
    ) -> Result<Vec<Option<(String, Vec<u8>)>>, Error> {
        let bucket = self.table.get_mut(&bucket)?;
        bucket.inner.get_many(keys).await
    }

    async fn set_many(
        &mut self,
        bucket: Resource<Bucket>,
        key_values: Vec<(String, Vec<u8>)>,
    ) -> Result<(), Error> {
        let bucket = self.table.get_mut(&bucket)?;
        bucket.inner.set_many(key_values).await
    }

    async fn delete_many(
        &mut self,
        bucket: Resource<Bucket>,
        keys: Vec<String>,
    ) -> Result<(), Error> {
        let bucket = self.table.get_mut(&bucket)?;
        bucket.inner.delete_many(keys).await
    }
}

/// Add all the `wasi-keyvalue` world's interfaces to a [`wasmtime::component::Linker`].
///
/// This function will add the `async` variant of all interfaces into the
/// `Linker` provided. By `async` this means that this function is only
/// compatible with [`Config::async_support(true)`][wasmtime::Config::async_support].
/// For embeddings with async support disabled see [`add_to_linker_sync`] instead.
pub fn add_to_linker_async<T: Send>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl Fn(&mut T) -> WasiKeyValue<'_> + Send + Sync + Copy + 'static,
) -> Result<()> {
    keyvalue::store::add_to_linker_get_host(l, f)?;
    keyvalue::atomics::add_to_linker_get_host(l, f)?;
    keyvalue::batch::add_to_linker_get_host(l, f)?;
    Ok(())
}

impl keyvalue_sync::store::Host for WasiKeyValue<'_> {
    fn open(&mut self, identifier: String) -> Result<Resource<Bucket>, Error> {
        in_tokio(async { keyvalue::store::Host::open(self, identifier).await })
    }

    fn convert_error(&mut self, err: Error) -> Result<keyvalue_sync::store::Error> {
        match err {
            Error::NoSuchStore => Ok(keyvalue_sync::store::Error::NoSuchStore),
            Error::AccessDenied => Ok(keyvalue_sync::store::Error::AccessDenied),
            Error::Other(e) => Ok(keyvalue_sync::store::Error::Other(e)),
        }
    }
}

impl keyvalue_sync::store::HostBucket for WasiKeyValue<'_> {
    fn get(&mut self, bucket: Resource<Bucket>, key: String) -> Result<Option<Vec<u8>>, Error> {
        in_tokio(async { keyvalue::store::HostBucket::get(self, bucket, key).await })
    }

    fn set(&mut self, bucket: Resource<Bucket>, key: String, value: Vec<u8>) -> Result<(), Error> {
        in_tokio(async { keyvalue::store::HostBucket::set(self, bucket, key, value).await })
    }

    fn delete(&mut self, bucket: Resource<Bucket>, key: String) -> Result<(), Error> {
        in_tokio(async { keyvalue::store::HostBucket::delete(self, bucket, key).await })
    }

    fn exists(&mut self, bucket: Resource<Bucket>, key: String) -> Result<bool, Error> {
        in_tokio(async { keyvalue::store::HostBucket::exists(self, bucket, key).await })
    }

    fn list_keys(
        &mut self,
        bucket: Resource<Bucket>,
        cursor: Option<u64>,
    ) -> Result<keyvalue_sync::store::KeyResponse, Error> {
        in_tokio(async {
            let resp = keyvalue::store::HostBucket::list_keys(self, bucket, cursor).await?;
            Ok(keyvalue_sync::store::KeyResponse {
                keys: resp.keys,
                cursor: resp.cursor,
            })
        })
    }

    fn drop(&mut self, bucket: Resource<Bucket>) -> Result<()> {
        keyvalue::store::HostBucket::drop(self, bucket)
    }
}

impl keyvalue_sync::atomics::Host for WasiKeyValue<'_> {
    fn increment(
        &mut self,
        bucket: Resource<Bucket>,
        key: String,
        delta: u64,
    ) -> Result<u64, Error> {
        in_tokio(async { keyvalue::atomics::Host::increment(self, bucket, key, delta).await })
    }
}

impl keyvalue_sync::batch::Host for WasiKeyValue<'_> {
    fn get_many(
        &mut self,
        bucket: Resource<Bucket>,
        keys: Vec<String>,
    ) -> Result<Vec<Option<(String, Vec<u8>)>>, Error> {
        in_tokio(async { keyvalue::batch::Host::get_many(self, bucket, keys).await })
    }

    fn set_many(
        &mut self,
        bucket: Resource<Bucket>,
        key_values: Vec<(String, Vec<u8>)>,
    ) -> Result<(), Error> {
        in_tokio(async { keyvalue::batch::Host::set_many(self, bucket, key_values).await })
    }

    fn delete_many(&mut self, bucket: Resource<Bucket>, keys: Vec<String>) -> Result<(), Error> {
        in_tokio(async { keyvalue::batch::Host::delete_many(self, bucket, keys).await })
    }
}

/// Add all the `wasi-keyvalue` world's interfaces to a [`wasmtime::component::Linker`].
///
/// This function will add the `sync` variant of all interfaces into the
/// `Linker` provided. For embeddings with async support see
/// [`add_to_linker_async`] instead.
pub fn add_to_linker_sync<T>(
    l: &mut wasmtime::component::Linker<T>,
    f: impl Fn(&mut T) -> WasiKeyValue<'_> + Send + Sync + Copy + 'static,
) -> Result<()> {
    keyvalue_sync::store::add_to_linker_get_host(l, f)?;
    keyvalue_sync::atomics::add_to_linker_get_host(l, f)?;
    keyvalue_sync::batch::add_to_linker_get_host(l, f)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "redis")]
    fn test_allow_redis_host() {
        let ctx = super::WasiKeyValueCtx::builder()
            .allow_redis_hosts(&["127.0.0.1:1234", "localhost", "/var/run/redis.sock"])
            .build();
        assert!(ctx.allow_redis_host(&super::Url::parse("redis://127.0.0.1:1234/db").unwrap()));
        assert!(ctx.allow_redis_host(&super::Url::parse("redis://localhost").unwrap()));
        assert!(!ctx.allow_redis_host(&super::Url::parse("redis://192.168.0.1").unwrap()));
        assert!(ctx.allow_redis_host(
            &super::Url::parse("redis+unix:///var/run/redis.sock?db=db").unwrap()
        ));
        assert!(!ctx.allow_redis_host(
            &super::Url::parse("redis+unix:///var/local/redis.sock?db=db").unwrap()
        ));
    }
}
