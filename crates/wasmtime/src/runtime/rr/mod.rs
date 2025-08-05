#![cfg(feature = "rr")]
//! Wasmtime's Record and Replay support.
//!
//! This feature is currently not optimized and under development
//!
//! ## Notes
//!
//! This module does NOT support RR for component builtins yet.

use crate::config::{ModuleVersionStrategy, RecordSettings, ReplaySettings};
use crate::prelude::*;
use core::fmt;
use events::EventActionError;
use serde::{Deserialize, Serialize};
// Use component/core events internally even without feature flags enabled
// so that [`RREvent`] has a well-defined serialization format, but export
// it for other modules only when enabled
pub use events::Validate;
use events::component_events as __component_events;
#[cfg(feature = "rr-component")]
pub use events::component_events;
use events::core_events as __core_events;
#[cfg(feature = "rr-core")]
pub use events::core_events;
pub use io::{RecordWriter, ReplayReader};

/// Encapsulation of event types comprising an [`RREvent`] sum type
mod events;
/// I/O support for reading and writing traces
mod io;

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
        #[derive(Debug, Clone, Serialize, Deserialize)]
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
    CoreHostFuncEntry(__core_events::HostFuncEntryEvent),
    /// Return from host function to Core Wasm
    CoreHostFuncReturn(__core_events::HostFuncReturnEvent),

    // REQUIRED events for replay
    //
    /// Instantiation of a component
    ComponentInstantiation(__component_events::InstantiationEvent),
    /// Return from host function to component
    ComponentHostFuncReturn(__component_events::HostFuncReturnEvent),
    /// Component ABI realloc call in linear wasm memory
    ComponentReallocEntry(__component_events::ReallocEntryEvent),
    /// Return from a type lowering operation
    ComponentLowerReturn(__component_events::LowerReturnEvent),
    /// Return from a store during a type lowering operation
    ComponentLowerStoreReturn(__component_events::LowerStoreReturnEvent),
    /// An attempt to obtain a mutable slice into Wasm linear memory
    ComponentMemorySliceWrite(__component_events::MemorySliceWriteEvent),

    // OPTIONAL events for replay validation
    //
    // ReallocReturn is optional because we can assume the realloc is deterministic
    // and the error message is subsumed by the containing LowerReturn/LowerStoreReturn
    /// Return from Component ABI realloc call
    ComponentReallocReturn(__component_events::ReallocReturnEvent),
    /// Call into host function from component
    ComponentHostFuncEntry(__component_events::HostFuncEntryEvent),
    /// Call into [Lower::lower] for type lowering
    ComponentLowerEntry(__component_events::LowerEntryEvent),
    /// Call into [Lower::store] during type lowering
    ComponentLowerStoreEntry(__component_events::LowerStoreEntryEvent)
}

/// Error type signalling failures during a replay run
#[derive(Debug, PartialEq, Eq)]
pub enum ReplayError {
    EmptyBuffer,
    FailedValidation,
    IncorrectEventVariant,
    InvalidOrdering,
    EventActionError(EventActionError),
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBuffer => {
                write!(f, "replay buffer is empty!")
            }
            Self::FailedValidation => {
                write!(f, "replay event validation failed")
            }
            Self::IncorrectEventVariant => {
                write!(f, "event method invoked on incorrect variant")
            }
            Self::EventActionError(e) => {
                write!(f, "{:?}", e)
            }
            Self::InvalidOrdering => {
                write!(f, "event occured at an invalid position in the trace")
            }
        }
    }
}

impl core::error::Error for ReplayError {}

impl From<EventActionError> for ReplayError {
    fn from(value: EventActionError) -> Self {
        Self::EventActionError(value)
    }
}

/// This trait provides the interface for a FIFO recorder
pub trait Recorder {
    /// Construct a recorder with the writer backend
    fn new_recorder(writer: Box<dyn RecordWriter>, settings: RecordSettings) -> Result<Self>
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
        F: FnOnce() -> T;

    /// Trigger an explicit flush of any buffered data to the writer
    ///
    /// Buffer should be emptied during this process
    fn flush(&mut self) -> Result<()>;

    /// Get settings associated with the recording process
    fn settings(&self) -> &RecordSettings;

    // Provided methods

    /// Record a event only when validation is requested
    #[inline]
    fn record_event_validation<T, F>(&mut self, f: F) -> Result<()>
    where
        T: Into<RREvent>,
        F: FnOnce() -> T,
    {
        let settings = self.settings();
        if settings.add_validation {
            self.record_event(f)?;
        }
        Ok(())
    }
}

/// This trait provides the interface for a FIFO replayer that
/// essentially operates as an iterator over the recorded events
pub trait Replayer: Iterator<Item = RREvent> {
    /// Constructs a reader on buffer
    fn new_replayer(reader: Box<dyn ReplayReader>, settings: ReplaySettings) -> Result<Self>
    where
        Self: Sized;

    /// Get settings associated with the replay process
    fn settings(&self) -> &ReplaySettings;

    /// Get the settings (embedded within the trace) during recording
    fn trace_settings(&self) -> &RecordSettings;

    // Provided Methods

    /// Pop the next replay event
    ///
    /// ## Errors
    ///
    /// Returns a [`ReplayError::EmptyBuffer`] if the buffer is empty
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
    /// See [`next_event_and`](Replayer::next_event_and)
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
        F: FnOnce(T) -> Result<(), ReplayError>,
    {
        let call_event = self.next_event_typed()?;
        Ok(f(call_event)?)
    }

    /// Conditionally process the next validation recorded event and if
    /// replay validation is enabled, run the validation check
    ///
    /// ## Errors
    ///
    /// In addition to errors in [`next_event_typed`](Replayer::next_event_typed),
    /// validation errors can be thrown
    #[inline]
    fn next_event_validation<T, Y>(&mut self, expect: &Y) -> Result<(), ReplayError>
    where
        T: TryFrom<RREvent> + Validate<Y>,
        ReplayError: From<<T as TryFrom<RREvent>>::Error>,
    {
        if self.trace_settings().add_validation {
            let event = self.next_event_typed::<T>()?;
            if self.settings().validate {
                event.validate(expect)
            } else {
                Ok(())
            }
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
    /// Settings in record configuration
    settings: RecordSettings,
}

impl RecordBuffer {
    /// Push a new record event [`RREvent`] to the buffer
    fn push_event(&mut self, event: RREvent) -> Result<()> {
        self.buf.push(event);
        if self.buf.len() >= self.settings().event_window_size {
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
    fn new_recorder(mut writer: Box<dyn RecordWriter>, settings: RecordSettings) -> Result<Self> {
        // Replay requires the Module version and record settings
        io::to_record_writer(ModuleVersionStrategy::WasmtimeVersion.as_str(), &mut writer)?;
        io::to_record_writer(&settings, &mut writer)?;
        Ok(RecordBuffer {
            buf: Vec::new(),
            writer: writer,
            settings: settings,
        })
    }

    #[inline]
    fn record_event<T, F>(&mut self, f: F) -> Result<()>
    where
        T: Into<RREvent>,
        F: FnOnce() -> T,
    {
        let event = f().into();
        log::debug!("Recording event => {}", &event);
        self.push_event(event)
    }

    fn flush(&mut self) -> Result<()> {
        log::debug!("Flushing record buffer...");
        for e in self.buf.drain(..) {
            io::to_record_writer(&e, &mut self.writer)?;
        }
        return Ok(());
    }

    #[inline]
    fn settings(&self) -> &RecordSettings {
        &self.settings
    }
}

/// Buffer to read replay data
pub struct ReplayBuffer {
    /// Reader to read replay trace from
    reader: Box<dyn ReplayReader>,
    /// Settings in replay configuration
    settings: ReplaySettings,
    /// Settings for record configuration (encoded in the trace)
    trace_settings: RecordSettings,
}

impl Iterator for ReplayBuffer {
    type Item = RREvent;

    fn next(&mut self) -> Option<Self::Item> {
        // Check for EoF
        let result = io::from_replay_reader(&mut self.reader, &mut [0; 0]);
        match result {
            Err(e) => {
                log::error!("Erroneous replay read: {}", e);
                None
            }
            Ok(event) => {
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
                    self.count()
                );
            }
        }
    }
}

impl Replayer for ReplayBuffer {
    fn new_replayer(mut reader: Box<dyn ReplayReader>, settings: ReplaySettings) -> Result<Self> {
        // Ensure module versions match
        let mut scratch = [0u8; 12];
        let version = io::from_replay_reader::<&str, _>(&mut reader, &mut scratch)?;
        assert_eq!(
            version,
            ModuleVersionStrategy::WasmtimeVersion.as_str(),
            "Wasmtime version mismatch between engine used for record and replay"
        );

        // Read the recording settings
        let trace_settings: RecordSettings = io::from_replay_reader(&mut reader, &mut [0; 0])?;

        if settings.validate && !trace_settings.add_validation {
            log::warn!(
                "Replay validation will be omitted since the recorded trace has no validation metadata..."
            );
        }

        Ok(ReplayBuffer {
            reader,
            settings,
            trace_settings,
        })
    }

    #[inline]
    fn settings(&self) -> &ReplaySettings {
        &self.settings
    }

    #[inline]
    fn trace_settings(&self) -> &RecordSettings {
        &self.trace_settings
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
        let record_settings = RecordSettings::default();
        let tmp = NamedTempFile::new()?;
        let tmppath = tmp.path().to_str().expect("Filename should be UTF-8");

        let values = vec![ValRaw::i32(1), ValRaw::f32(2), ValRaw::i64(3)];

        // Record values
        let mut recorder =
            RecordBuffer::new_recorder(Box::new(File::create(tmppath)?), record_settings)?;
        let event = component_wasm::HostFuncReturnEvent::new(values.as_slice(), None);
        recorder.record_event(event.clone())?;
        recorder.flush()?;

        let tmp = tmp.into_temp_path();
        let tmppath = <TempPath as AsRef<Path>>::as_ref(&tmp)
            .to_str()
            .expect("Filename should be UTF-8");
        let replay_settings = ReplaySettings { validate: true };

        // Assert that replayed values are identical
        let mut replayer =
            ReplayBuffer::new_replayer(Box::new(File::open(tmppath)?), replay_settings)?;
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
