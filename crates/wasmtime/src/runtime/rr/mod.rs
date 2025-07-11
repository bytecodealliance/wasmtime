//! Wasmtime's Record and Replay support
//!
//! This feature is currently experimental and hence not optimized.

use crate::config::{RecordConfig, RecordMetadata, ReplayConfig, ReplayMetadata};
use crate::prelude::*;
#[allow(unused_imports)]
use crate::runtime::Store;
use core::fmt;
use postcard;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::File;
#[allow(unused_imports)]
use std::io::{BufWriter, Seek, Write};

/// Encapsulation of event types comprising an [`RREvent`] sum type
pub mod events;
use events::*;

/// Macro template for [`RREvent`] and its conversion to/from specific
/// event types
macro_rules! rr_event {
        (
            $(
                $(#[doc = $doc:literal])*
                $variant:ident => $event:ty
            ),*
        ) => (
        /// A single, unified, low-level recording/replay event
        ///
        /// This type is the narrow waist for serialization/deserialization.
        /// Higher-level events (e.g. import calls consisting of lifts and lowers
        /// of parameter/return types) may drop down to one or more [`RREvent`]s
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub enum RREvent {
            $(
                $(#[doc = $doc])*
                $variant($event),
            )*
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

// Set of supported record/replay events
rr_event! {
    /// Call into host function from Core Wasm
    CoreHostFuncEntry => core_wasm::HostFuncEntryEvent,
    /// Return from host function to Core Wasm
    CoreHostFuncReturn => core_wasm::HostFuncReturnEvent,

    // REQUIRED events for replay
    //
    /// Return from host function to component
    ComponentHostFuncReturn => component_wasm::HostFuncReturnEvent,
    /// Component ABI realloc call in linear wasm memory
    ComponentReallocEntry => component_wasm::ReallocEntryEvent,
    /// Return from a type lowering operation
    ComponentLowerReturn => component_wasm::LowerReturnEvent,
    /// Return from a store during a type lowering operation
    ComponentLowerStoreReturn => component_wasm::LowerStoreReturnEvent,
    /// An attempt to obtain a mutable slice into Wasm linear memory
    ComponentMemorySliceBorrow => component_wasm::MemorySliceBorrowEvent,

    // OPTIONAL events for replay validation
    //
    /// Call into host function from component
    ComponentHostFuncEntry => component_wasm::HostFuncEntryEvent,
    /// Call into [Lower::lower] for type lowering
    ComponentLowerEntry => component_wasm::LowerEntryEvent,
    /// Call into [Lower::store] during type lowering
    ComponentLowerStoreEntry => component_wasm::LowerStoreEntryEvent,
    /// Return from Component ABI realloc call
    ComponentReallocReturn => component_wasm::ReallocReturnEvent
}

#[derive(Debug)]
pub enum ReplayError {
    EmptyBuffer,
    FailedFuncValidation,
    IncorrectEventVariant,
    EventActionError(EventActionError),
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBuffer => {
                write!(f, "replay buffer is empty!")
            }
            Self::FailedFuncValidation => {
                write!(f, "func replay event validation failed")
            }
            Self::IncorrectEventVariant => {
                write!(f, "event method invoked on incorrect variant")
            }
            Self::EventActionError(e) => {
                write!(f, "{:?}", e)
            }
        }
    }
}

impl std::error::Error for ReplayError {}

impl From<EventActionError> for ReplayError {
    fn from(value: EventActionError) -> Self {
        Self::EventActionError(value)
    }
}

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

pub trait Replayer: Iterator {
    type ReplayError;

    /// Constructs a reader on buffer
    fn new_replayer(cfg: ReplayConfig) -> Result<Self>
    where
        Self: Sized;

    /// Get metadata associated with the replay process
    fn metadata(&self) -> &ReplayMetadata;
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

impl Iterator for ReplayBuffer {
    type Item = RREvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.data.buf.pop_front()
    }
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

        let event = core_wasm::HostFuncEntryEvent::new(values.as_slice(), None);

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
        let event_pop = core_wasm::HostFuncEntryEvent::try_from(
            replayer.next().ok_or(ReplayError::EmptyBuffer)?,
        )?;
        // Replay matches record
        assert!(event == event_pop);

        // Queue is empty
        assert!(replayer.next().is_none());

        Ok(())
    }
}
