//! Implementation of wasip2 version of `wasi:keyvalue` package

mod generated {
    wasmtime::component::bindgen!({
        path: "src/p2/wit",
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
use wasmtime::component::Resource;

use crate::{Bucket, Error, WasiKeyValue};

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
