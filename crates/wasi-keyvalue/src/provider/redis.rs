use crate::{bindings::wasi::keyvalue::store::KeyResponse, Error, Host};
use anyhow::Result;
use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands, RedisError};
use std::time::Duration;

struct Redis {
    conn: MultiplexedConnection,
}

impl From<RedisError> for Error {
    fn from(err: RedisError) -> Self {
        Self::Other(err.to_string())
    }
}

pub(crate) async fn open(
    identifier: String,
    response_timeout: Option<Duration>,
    connection_timeout: Option<Duration>,
) -> Result<impl Host, RedisError> {
    let client = redis::Client::open(identifier)?;
    let conn = client
        .get_multiplexed_async_connection_with_timeouts(
            response_timeout.unwrap_or(Duration::MAX),
            connection_timeout.unwrap_or(Duration::MAX),
        )
        .await?;
    Ok(Redis { conn })
}

#[async_trait]
impl Host for Redis {
    async fn get(&mut self, key: String) -> Result<Option<Vec<u8>>, Error> {
        let v: Option<Vec<u8>> = self.conn.get(key).await?;
        Ok(v)
    }

    async fn set(&mut self, key: String, value: Vec<u8>) -> Result<(), Error> {
        let _: () = self.conn.set(key, value).await?;
        Ok(())
    }

    async fn delete(&mut self, key: String) -> Result<(), Error> {
        let _: () = self.conn.del(key).await?;
        Ok(())
    }

    async fn exists(&mut self, key: String) -> Result<bool, Error> {
        let exists: bool = self.conn.exists(key).await?;
        Ok(exists)
    }

    async fn list_keys(&mut self, cursor: Option<u64>) -> Result<KeyResponse, Error> {
        let cursor = cursor.unwrap_or(0);
        let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .query_async(&mut self.conn)
            .await?;

        Ok(KeyResponse {
            keys,
            cursor: if new_cursor == 0 {
                None
            } else {
                Some(new_cursor)
            },
        })
    }

    async fn increment(&mut self, key: String, delta: u64) -> Result<u64, Error> {
        let v: u64 = self.conn.incr(key, delta).await?;
        Ok(v)
    }

    async fn get_many(
        &mut self,
        keys: Vec<String>,
    ) -> Result<Vec<Option<(String, Vec<u8>)>>, Error> {
        let values: Vec<Option<Vec<u8>>> = self.conn.get(keys.clone()).await?;

        Ok(keys
            .into_iter()
            .zip(values.into_iter())
            .map(|(key, value)| value.map(|v| (key, v)))
            .collect())
    }

    async fn set_many(&mut self, key_values: Vec<(String, Vec<u8>)>) -> Result<(), Error> {
        let mut pipe = redis::pipe();
        for (key, value) in key_values {
            pipe.set(key, value).ignore();
        }
        let _: () = pipe.query_async(&mut self.conn).await?;
        Ok(())
    }

    async fn delete_many(&mut self, keys: Vec<String>) -> Result<(), Error> {
        let mut pipe = redis::pipe();
        for key in keys {
            pipe.del(key).ignore();
        }
        let _: () = pipe.query_async(&mut self.conn).await?;
        Ok(())
    }
}
