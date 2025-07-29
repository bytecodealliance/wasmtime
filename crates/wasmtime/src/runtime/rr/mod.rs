//! Wasmtime's Record and Replay support.
//!
//! This feature is currently not optimized and under development
//!
//! ## Notes
//!
//! This module does NOT support RR for component builtins yet.

use crate::config::{
    ModuleVersionStrategy, RecordMetadata, RecordWriter, ReplayMetadata, ReplayReader,
};
use crate::prelude::*;
use core::fmt;
use postcard;
use serde::{Deserialize, Serialize};

/// Encapsulation of event types comprising an [`RREvent`] sum type
pub mod events;
use events::*;

/// Macro template for [`RREvent`] and its conversion to/from specific
/// event types
macro_rules! rr_event {
        (
            $(
                $(#[doc = $doc:literal])*
                $variant:ident($event:ty)
            ),*
        ) => (
        /// A single, unified, low-level recording/replay event
        ///
        /// This type is the narrow waist for serialization/deserialization.
        /// Higher-level events (e.g. import calls consisting of lifts and lowers
        /// of parameter/return types) may drop down to one or more [`RREvent`]s
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub enum RREvent {
            /// Event signalling the end of a trace
            Eof,
            $(
                $(#[doc = $doc])*
                $variant($event),
            )*
        }

        impl fmt::Display for RREvent {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    Self::Eof => write!(f, "Eof event"),
                    $(
                    Self::$variant(e) => write!(f, "{:?}", e),
                    )*
                }
            }
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
    CoreHostFuncEntry(core_wasm::HostFuncEntryEvent),
    /// Return from host function to Core Wasm
    CoreHostFuncReturn(core_wasm::HostFuncReturnEvent),

    // REQUIRED events for replay
    //
    /// Instantiation of a component
    ComponentInstantiation(component_wasm::InstantiationEvent),
    /// Return from host function to component
    ComponentHostFuncReturn(component_wasm::HostFuncReturnEvent),
    /// Component ABI realloc call in linear wasm memory
    ComponentReallocEntry(component_wasm::ReallocEntryEvent),
    /// Return from a type lowering operation
    ComponentLowerReturn(component_wasm::LowerReturnEvent),
    /// Return from a store during a type lowering operation
    ComponentLowerStoreReturn(component_wasm::LowerStoreReturnEvent),
    /// An attempt to obtain a mutable slice into Wasm linear memory
    ComponentMemorySliceWrite(component_wasm::MemorySliceWriteEvent),

    // OPTIONAL events for replay validation
    //
    /// Call into host function from component
    ComponentHostFuncEntry(component_wasm::HostFuncEntryEvent),
    /// Call into [Lower::lower] for type lowering
    ComponentLowerEntry(component_wasm::LowerEntryEvent),
    /// Call into [Lower::store] during type lowering
    ComponentLowerStoreEntry(component_wasm::LowerStoreEntryEvent),
    /// Return from Component ABI realloc call
    ComponentReallocReturn(component_wasm::ReallocReturnEvent)
}

/// Error type signalling failures during a replay run
#[derive(Debug, PartialEq, Eq)]
pub enum ReplayError {
    EmptyBuffer,
    FailedFuncValidation,
    FailedModuleValidation,
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
            Self::FailedModuleValidation => {
                write!(f, "module load replay event validation failed")
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

/// This trait provides the interface for a FIFO recorder
pub trait Recorder {
    /// Construct a recorder with the writer backend
    fn new_recorder(writer: Box<dyn RecordWriter>, metadata: RecordMetadata) -> Result<Self>
    where
        Self: Sized;

    /// Record the event generated by `f`
    ///
    /// ## Error
    ///
    /// Propogates from underlying writer
    fn record_event<T, F>(&mut self, f: F) -> Result<()>
    where
        T: Into<RREvent>,
        F: FnOnce(&RecordMetadata) -> T;

    /// Trigger an explicit flush of any buffered data to the writer
    ///
    /// Buffer should be emptied during this process
    fn flush(&mut self) -> Result<()>;

    /// Get metadata associated with the recording process
    fn metadata(&self) -> &RecordMetadata;

    // Provided methods

    /// Conditionally [`record_event`](Self::record_event) when `pred` is true
    fn record_event_if<T, P, F>(&mut self, pred: P, f: F) -> Result<()>
    where
        T: Into<RREvent>,
        P: FnOnce(&RecordMetadata) -> bool,
        F: FnOnce(&RecordMetadata) -> T,
    {
        if pred(self.metadata()) {
            self.record_event(f)?;
        }
        Ok(())
    }
}

/// This trait provides the interface for a FIFO replayer that
/// essentially operates as an iterator over the recorded events
pub trait Replayer: Iterator<Item = RREvent> {
    /// Constructs a reader on buffer
    fn new_replayer(reader: Box<dyn ReplayReader>, metadata: ReplayMetadata) -> Result<Self>
    where
        Self: Sized;

    /// Get metadata associated with the replay process
    fn metadata(&self) -> &ReplayMetadata;

    /// Get the metadata embedded within the trace during recording
    fn trace_metadata(&self) -> &RecordMetadata;

    // Provided Methods

    /// Pop the next replay event
    ///
    /// ## Errors
    ///
    /// Returns a `ReplayError::EmptyBuffer` if the buffer is empty
    #[inline]
    fn next_event(&mut self) -> Result<RREvent, ReplayError> {
        let event = self.next().ok_or(ReplayError::EmptyBuffer);
        if let Ok(e) = &event {
            log::debug!("Replay Event => {}", e);
        }
        event
    }

    /// Pop the next replay event with an attemped type conversion to expected
    /// event type
    ///
    /// ## Errors
    ///
    /// See [`next_event_and`](Self::next_event_and)
    #[inline]
    fn next_event_typed<T>(&mut self) -> Result<T, ReplayError>
    where
        T: TryFrom<RREvent>,
        ReplayError: From<<T as TryFrom<RREvent>>::Error>,
    {
        T::try_from(self.next_event()?).map_err(|e| e.into())
    }

    /// Pop the next replay event and calls `f` with a desired type conversion
    ///
    /// ## Errors
    ///
    /// Returns a [`ReplayError::EmptyBuffer`] if the buffer is empty or a
    /// [`ReplayError::IncorrectEventVariant`] if it failed to convert type safely
    #[inline]
    fn next_event_and<T, F>(&mut self, f: F) -> Result<(), ReplayError>
    where
        T: TryFrom<RREvent>,
        ReplayError: From<<T as TryFrom<RREvent>>::Error>,
        F: FnOnce(T, &ReplayMetadata) -> Result<(), ReplayError>,
    {
        let call_event = self.next_event_typed()?;
        Ok(f(call_event, self.metadata())?)
    }

    /// Conditionally execute [`next_event_and`](Self::next_event_and) when `pred` is true
    #[inline]
    fn next_event_if<T, P, F>(&mut self, pred: P, f: F) -> Result<(), ReplayError>
    where
        T: TryFrom<RREvent>,
        ReplayError: From<<T as TryFrom<RREvent>>::Error>,
        P: FnOnce(&ReplayMetadata, &RecordMetadata) -> bool,
        F: FnOnce(T, &ReplayMetadata) -> Result<(), ReplayError>,
    {
        if pred(self.metadata(), self.trace_metadata()) {
            self.next_event_and(f)
        } else {
            Ok(())
        }
    }
}

/// Buffer to write recording data.
///
/// This type can be optimized for [`RREvent`] data configurations.
pub struct RecordBuffer {
    /// In-memory event buffer to enable windows for coalescing
    buf: Vec<RREvent>,
    /// Writer to store data into
    writer: Box<dyn RecordWriter>,
    /// Metadata for record configuration
    metadata: RecordMetadata,
}

impl RecordBuffer {
    /// Push a new record event [`RREvent`] to the buffer
    fn push_event(&mut self, event: RREvent) -> Result<()> {
        self.buf.push(event);
        if self.buf.len() >= self.metadata().event_window_size {
            self.flush()?;
        }
        Ok(())
    }
}

impl Drop for RecordBuffer {
    fn drop(&mut self) {
        // Insert End of trace delimiter
        self.push_event(RREvent::Eof).unwrap();
        self.flush().unwrap();
    }
}

impl Recorder for RecordBuffer {
    fn new_recorder(mut writer: Box<dyn RecordWriter>, metadata: RecordMetadata) -> Result<Self> {
        // Replay requires the Module version and RecordMetadata configuration
        postcard::to_io(ModuleVersionStrategy::WasmtimeVersion.as_str(), &mut writer)?;
        postcard::to_io(&metadata, &mut writer)?;
        Ok(RecordBuffer {
            buf: Vec::new(),
            writer: writer,
            metadata: metadata,
        })
    }

    #[inline]
    fn record_event<T, F>(&mut self, f: F) -> Result<()>
    where
        T: Into<RREvent>,
        F: FnOnce(&RecordMetadata) -> T,
    {
        let event = f(self.metadata()).into();
        log::debug!("Recording event => {}", &event);
        self.push_event(event)
    }

    fn flush(&mut self) -> Result<()> {
        log::debug!("Flushing record buffer...");
        for e in self.buf.drain(..) {
            postcard::to_io(&e, &mut self.writer)?;
        }
        return Ok(());
    }

    #[inline]
    fn metadata(&self) -> &RecordMetadata {
        &self.metadata
    }
}

/// Buffer to read replay data
pub struct ReplayBuffer {
    /// Reader to read replay trace from
    reader: Box<dyn ReplayReader>,
    /// Metadata for replay configuration
    metadata: ReplayMetadata,
    /// Metadata for record configuration (encoded in the trace)
    trace_metadata: RecordMetadata,
}

impl Iterator for ReplayBuffer {
    type Item = RREvent;

    fn next(&mut self) -> Option<Self::Item> {
        // Check for EoF
        let result = postcard::from_io((&mut self.reader, &mut [0; 0]));
        match result {
            Err(e) => {
                log::error!("Erroneous replay read: {}", e);
                None
            }
            Ok((event, _)) => {
                if let RREvent::Eof = event {
                    None
                } else {
                    Some(event)
                }
            }
        }
    }
}

impl Drop for ReplayBuffer {
    fn drop(&mut self) {
        if let Some(event) = self.next() {
            if let RREvent::Eof = event {
            } else {
                log::warn!(
                    "Replay buffer is dropped with {} remaining events, and is likely an invalid execution",
                    self.count() - 1
                );
            }
        }
    }
}

impl Replayer for ReplayBuffer {
    fn new_replayer(mut reader: Box<dyn ReplayReader>, metadata: ReplayMetadata) -> Result<Self> {
        // Ensure module versions match
        let mut scratch = [0u8; 12];
        let (version, _) = postcard::from_io::<&str, _>((&mut reader, &mut scratch))?;
        assert_eq!(
            version,
            ModuleVersionStrategy::WasmtimeVersion.as_str(),
            "Wasmtime version mismatch between engine used for record and replay"
        );

        // Read the recording metadata
        let (trace_metadata, _) = postcard::from_io((&mut reader, &mut [0; 0]))?;

        Ok(ReplayBuffer {
            reader: reader,
            metadata: metadata,
            trace_metadata: trace_metadata,
        })
    }

    #[inline]
    fn metadata(&self) -> &ReplayMetadata {
        &self.metadata
    }

    #[inline]
    fn trace_metadata(&self) -> &RecordMetadata {
        &self.trace_metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValRaw;
    use std::fs::File;
    use std::path::Path;
    use tempfile::{NamedTempFile, TempPath};

    #[test]
    fn rr_buffers() -> Result<()> {
        let record_metadata = RecordMetadata::default();
        let tmp = NamedTempFile::new()?;
        let tmppath = tmp.path().to_str().expect("Filename should be UTF-8");

        let values = vec![ValRaw::i32(1), ValRaw::f32(2), ValRaw::i64(3)];

        // Record values
        let mut recorder =
            RecordBuffer::new_recorder(Box::new(File::create(tmppath)?), record_metadata)?;
        let event = component_wasm::HostFuncReturnEvent::new(
            values.as_slice(),
            #[cfg(feature = "rr-type-validation")]
            None,
        );
        recorder.record_event(|_| event.clone())?;
        recorder.flush()?;

        let tmp = tmp.into_temp_path();
        let tmppath = <TempPath as AsRef<Path>>::as_ref(&tmp)
            .to_str()
            .expect("Filename should be UTF-8");
        let replay_metadata = ReplayMetadata { validate: true };

        // Assert that replayed values are identical
        let mut replayer =
            ReplayBuffer::new_replayer(Box::new(File::open(tmppath)?), replay_metadata)?;
        replayer.next_event_and(|store_event: component_wasm::HostFuncReturnEvent, _| {
            // Check replay matches record
            assert!(store_event == event);
            Ok(())
        })?;

        // Check queue is empty
        assert!(replayer.next().is_none());

        Ok(())
    }
}
