//! Wasmtime's Record and Replay support
//!
//! This feature is currently experimental and hence not optimized.

use crate::config::{RecordConfig, RecordMetadata, ReplayConfig, ReplayMetadata};
use crate::prelude::*;
#[allow(unused_imports)]
use crate::runtime::Store;
use postcard;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufWriter, Seek, Write};

/// Encapsulation of event types comprising an [`RREvent`] sum type
mod events;
pub use events::*;

pub trait Recorder {
    /// Constructs a writer on new buffer
    fn new_recorder(cfg: RecordConfig) -> Result<Self>
    where
        Self: Sized;

    /// Push a newly record event [`RREvent`] to the buffer
    fn push_event(&mut self, event: RREvent) -> ();

    /// Flush memory contents to underlying persistent storage
    ///
    /// Buffer should be emptied during this process
    fn flush_to_file(&mut self) -> Result<()>;

    /// Get metadata associated with the recording process
    fn metadata(&self) -> &RecordMetadata;
}

pub trait Replayer {
    type ReplayError;

    /// Constructs a reader on buffer
    fn new_replayer(cfg: ReplayConfig) -> Result<Self>
    where
        Self: Sized;

    /// Pop the next [`RREvent`] from the buffer
    /// Events should be FIFO
    fn pop_event(&mut self) -> Result<RREvent, ReplayError>;

    /// Get metadata associated with the replay process
    fn metadata(&self) -> &ReplayMetadata;
}

/// Macro template for [`RREvent`] and its conversion to/from specific
/// event types
macro_rules! rr_event {
    ( $( $variant:ident($event:ty) ),* ) => (
        /// A single, unified, low-level recording/replay event
        ///
        /// This type is the narrow waist for serialization/deserialization.
        /// Higher-level events (e.g. import calls consisting of lifts and lowers
        /// of parameter/return types) may drop down to one or more [`RREvent`]s
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub enum RREvent {
            $($variant($event),)*
        }
        $(
            impl From<$event> for RREvent {
                fn from(value: $event) -> Self {
                    RREvent::$variant(value)
                }
            }
            impl TryFrom<RREvent> for $event {
                type Error = ReplayError;
                fn try_from(value: RREvent) -> Result<Self, Self::Error> {
                    if let RREvent::$variant(x) = value {
                        Ok(x)
                    } else {
                        Err(ReplayError::IncorrectEventVariant)
                    }
                }
            }
        )*
   );
}

// Set of supported events
rr_event! {
    CoreHostFuncEntry(CoreHostFuncEntryEvent),
    CoreHostFuncReturn(CoreHostFuncReturnEvent)
}

/// The underlying serialized/deserialized type
type RRBufferData = VecDeque<RREvent>;

/// Common data for recorders and replayers
///
/// Flexibility of this struct can also be improved with:
/// * Support for generic writers beyond [File] (will require a generic on [Store])
#[derive(Debug)]
pub struct RRDataCommon {
    /// Ordered list of record/replay events
    buf: RRBufferData,
    /// Persistent storage-backed handle
    rw: File,
}

#[derive(Debug)]
/// Buffer to write recording data
pub struct RecordBuffer {
    data: RRDataCommon,
    metadata: RecordMetadata,
}

impl Recorder for RecordBuffer {
    fn new_recorder(cfg: RecordConfig) -> Result<Self> {
        Ok(RecordBuffer {
            data: RRDataCommon {
                buf: VecDeque::new(),
                rw: File::create(cfg.path)?,
            },
            metadata: cfg.metadata,
        })
    }

    fn push_event(&mut self, event: RREvent) {
        self.data.buf.push_back(event)
    }

    fn flush_to_file(&mut self) -> Result<()> {
        // Seralizing each event independently prevents checking for vector sizes
        // during deserialization
        let data = &mut self.data;
        for v in &data.buf {
            postcard::to_io(&v, &mut data.rw)?;
        }
        data.rw.flush()?;
        data.buf.clear();
        println!(
            "Record flush | File size: {:?} bytes",
            data.rw.metadata()?.len()
        );
        Ok(())
    }

    #[inline]
    fn metadata(&self) -> &RecordMetadata {
        &self.metadata
    }
}

#[derive(Debug)]
/// Buffer to read replay data
pub struct ReplayBuffer {
    data: RRDataCommon,
    metadata: ReplayMetadata,
}

impl Replayer for ReplayBuffer {
    type ReplayError = ReplayError;

    fn new_replayer(cfg: ReplayConfig) -> Result<Self> {
        let mut file = File::open(cfg.path)?;
        let mut events = VecDeque::<RREvent>::new();
        // Read till EOF
        while file.stream_position()? != file.metadata()?.len() {
            let (event, _): (RREvent, _) = postcard::from_io((&mut file, &mut [0; 0]))?;
            events.push_back(event);
        }
        Ok(ReplayBuffer {
            data: RRDataCommon {
                buf: events,
                rw: file,
            },
            metadata: cfg.metadata,
        })
    }

    fn pop_event(&mut self) -> Result<RREvent, ReplayError> {
        self.data
            .buf
            .pop_front()
            .ok_or(Self::ReplayError::EmptyBuffer.into())
    }

    #[inline]
    fn metadata(&self) -> &ReplayMetadata {
        &self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValRaw;
    use core::mem::MaybeUninit;
    use std::path::Path;
    use tempfile::{NamedTempFile, TempPath};

    #[test]
    fn rr_buffers() -> Result<()> {
        let tmp = NamedTempFile::new()?;

        let tmppath = tmp.path().to_str().expect("Filename should be UTF-8");
        let record_cfg = RecordConfig {
            path: String::from(tmppath),
            metadata: RecordMetadata {
                add_validation: true,
            },
        };

        let values = vec![ValRaw::i32(1), ValRaw::f32(2), ValRaw::i64(3)]
            .into_iter()
            .map(|x| MaybeUninit::new(x))
            .collect::<Vec<_>>();

        let event = CoreHostFuncEntryEvent::new(values.as_slice(), None);

        // Record values
        let mut recorder = RecordBuffer::new_recorder(record_cfg)?;
        recorder.push_event(event.clone().into());
        recorder.flush_to_file()?;

        let tmp = tmp.into_temp_path();
        let tmppath = <TempPath as AsRef<Path>>::as_ref(&tmp)
            .to_str()
            .expect("Filename should be UTF-8");

        // Assert that replayed values are identical
        let replay_cfg = ReplayConfig {
            path: String::from(tmppath),
            metadata: ReplayMetadata { validate: true },
        };
        let mut replayer = ReplayBuffer::new_replayer(replay_cfg)?;
        let event_pop = CoreHostFuncEntryEvent::try_from(replayer.pop_event()?)?;
        // Replay matches record
        assert!(event == event_pop);

        // Queue is empty
        let event = replayer.pop_event();
        assert!(event.is_err() && matches!(event.unwrap_err(), ReplayError::EmptyBuffer));

        Ok(())
    }
}
