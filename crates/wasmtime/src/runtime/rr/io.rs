use crate::prelude::*;
use postcard;
use serde::{Deserialize, Serialize};

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::io::{Write, Read};
        /// An [`Write`] usable for recording in RR
        ///
        /// This supports `no_std`, but must be [Send] and [Sync]
        pub trait RecordWriter: Write + Send + Sync {}
        impl<T: Write + Send + Sync> RecordWriter for T {}

        /// An [`Read`] usable for replaying in RR
        pub trait ReplayReader: Read + Send + Sync {}
        impl<T: Read + Send + Sync> ReplayReader for T {}

    } else {
        // `no_std` configuration
        use embedded_io::{Read, Write};

        /// An [`Write`] usable for recording in RR
        ///
        /// This supports `no_std`, but must be [Send] and [Sync]
        pub trait RecordWriter: Write + Send + Sync {}
        impl<T: Write + Send + Sync> RecordWriter for T {}

        /// An [`Read`] usable for replaying in RR
        ///
        /// This supports `no_std`, but must be [Send] and [Sync]
        pub trait ReplayReader: Read + Send + Sync {}
        impl<T: Read + Send + Sync> ReplayReader for T {}
    }
}

/// Serialize and write `value` to a `RecordWriter`
///
/// Currently uses `postcard` serializer
pub fn to_record_writer<T, W>(value: &T, writer: W) -> Result<()>
where
    T: Serialize + ?Sized,
    W: RecordWriter,
{
    cfg_if::cfg_if! {
        if #[cfg(feature = "std")] {
            postcard::to_io(value, writer)?;
        } else {
            postcard::to_eio(value, writer)?;
        }
    }
    Ok(())
}

/// Read and deserialize a `value` from a `ReplayReader`.
///
/// Currently uses `postcard` deserializer, with optional scratch
/// buffer to deserialize into
pub fn from_replay_reader<'a, T, R>(reader: R, scratch: &'a mut [u8]) -> Result<T>
where
    T: Deserialize<'a>,
    R: ReplayReader + 'a,
{
    cfg_if::cfg_if! {
        if #[cfg(feature = "std")] {
            Ok(postcard::from_io((reader, scratch))?.0)
        } else {
            Ok(postcard::from_eio((reader, scratch))?.0)
        }
    }
}
