//! Wasmtime's Record and Replay support
//!
//! This feature is currently experimental and hence not optimized.
//! In particular, the following opportunities are immediately identifiable:
//! * Switch [RRFuncArgTypes] to use [Vec<WasmValType>]

use crate::ValRaw;
use crate::config::{RecordConfig, RecordMetadata, ReplayConfig, ReplayMetadata};
use crate::prelude::*;
#[allow(unused_imports)]
use crate::runtime::Store;
use core::fmt;
use core::mem::{self, MaybeUninit};
use postcard;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufWriter, Seek, Write};
use wasmtime_environ::component::InterfaceType;
use wasmtime_environ::{WasmFuncType, WasmValType};

const VAL_RAW_SIZE: usize = mem::size_of::<ValRaw>();

type RRFuncArgVals = Vec<ValRawSer>;
type RRFuncArgTypes = WasmFuncType;

/// Transmutable byte array used to serialize [`ValRaw`] union
///
/// Maintaining the exact layout is crucial for zero-copy transmutations
/// between [`ValRawSer`] and [`ValRaw`]
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
#[repr(C)]
pub struct ValRawSer([u8; VAL_RAW_SIZE]);

impl From<ValRaw> for ValRawSer {
    fn from(value: ValRaw) -> Self {
        unsafe { Self(mem::transmute(value)) }
    }
}

impl From<ValRawSer> for ValRaw {
    fn from(value: ValRawSer) -> Self {
        unsafe { mem::transmute(value.0) }
    }
}

impl fmt::Debug for ValRawSer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex_digits_per_byte = 2;
        let _ = write!(f, "0x..");
        for b in self.0.iter().rev() {
            let _ = write!(f, "{:0width$x}", b, width = hex_digits_per_byte);
        }
        Ok(())
    }
}

/// Construct [`RRFuncArgVals`] from raw value buffer
fn func_argvals_from_raw_slice(args: &[MaybeUninit<ValRaw>]) -> RRFuncArgVals {
    args.iter()
        .map(|x| unsafe { ValRawSer::from(x.assume_init()) })
        .collect::<Vec<_>>()
}

/// Encode [`RRFuncArgVals`] back into raw value buffer
fn func_argvals_into_raw_slice(rr_args: RRFuncArgVals, raw_args: &mut [MaybeUninit<ValRaw>]) {
    for (src, dst) in rr_args.into_iter().zip(raw_args.iter_mut()) {
        *dst = MaybeUninit::new(src.into());
    }
}

/// Typechecking validation for replay, if `src_types` exist
///
/// Returns [`ReplayError::FailedFuncValidation`] if typechecking fails
fn replay_args_typecheck<T>(src_types: Option<&T>, expect_types: &T) -> Result<(), ReplayError>
where
    T: PartialEq,
{
    if let Some(types) = src_types {
        if types == expect_types {
            Ok(())
        } else {
            Err(ReplayError::FailedFuncValidation)
        }
    } else {
        println!(
            "Warning: Replay typechecking cannot be performed 
                            since recorded trace is missing validation data"
        );
        Ok(())
    }
}

#[derive(Debug)]
pub enum ReplayError {
    EmptyBuffer,
    FailedFuncValidation,
    IncorrectEventVariant,
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBuffer => {
                write!(f, "replay buffer is empty!")
            }
            Self::FailedFuncValidation => {
                write!(f, "func replay event typecheck validation failed")
            }
            Self::IncorrectEventVariant => {
                write!(f, "event methods invoked on incorrect variant")
            }
        }
    }
}

impl std::error::Error for ReplayError {}

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

pub struct ComponentRRFuncArgs {
    args: RRFuncArgVals,
    types: Option<InterfaceType>,
}

/// A call event from a Core Wasm module into the host
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreHostFuncEntryEvent {
    /// Raw values passed across the call/return boundary
    args: RRFuncArgVals,
    /// Optional param/return types (required to support replay validation)
    types: Option<RRFuncArgTypes>,
}

impl CoreHostFuncEntryEvent {
    // Record
    pub fn new(args: &[MaybeUninit<ValRaw>], types: Option<WasmFuncType>) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
            types: types,
        }
    }
    // Replay
    pub fn validate(&self, expect_types: &RRFuncArgTypes) -> Result<(), ReplayError> {
        replay_args_typecheck(self.types.as_ref(), expect_types)
    }
}

/// A return event after a host call for a Core Wasm
///
/// Matches 1:1 with [`CoreHostFuncEntryEvent`]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreHostFuncReturnEvent {
    /// Raw values passed across the call/return boundary
    args: RRFuncArgVals,
    /// Optional param/return types (required to support replay validation)
    types: Option<RRFuncArgTypes>,
}

impl CoreHostFuncReturnEvent {
    // Record
    pub fn new(args: &[MaybeUninit<ValRaw>], types: Option<WasmFuncType>) -> Self {
        Self {
            args: func_argvals_from_raw_slice(args),
            types: types,
        }
    }
    // Replay
    /// Consume the caller event and encode it back into the slice with an optional
    /// typechecking validation of the event.
    pub fn move_into_slice(
        self,
        args: &mut [MaybeUninit<ValRaw>],
        expect_types: Option<&WasmFuncType>,
    ) -> Result<(), ReplayError> {
        if let Some(e) = expect_types {
            replay_args_typecheck(self.types.as_ref(), e)?;
        }
        func_argvals_into_raw_slice(self.args, args);
        Ok(())
    }
}

/// A single, unified, low-level recording/replay event
///
/// This type is the narrow waist for serialization/deserialization.
/// Higher-level events (e.g. import calls consisting of lifts and lowers
/// of parameter/return types) may drop down to one or more [`RREvent`]s
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RREvent {
    CoreHostFuncEntry(CoreHostFuncEntryEvent),
    CoreHostFuncReturn(CoreHostFuncReturnEvent),
}

impl From<CoreHostFuncEntryEvent> for RREvent {
    fn from(value: CoreHostFuncEntryEvent) -> Self {
        RREvent::CoreHostFuncEntry(value)
    }
}
impl TryFrom<RREvent> for CoreHostFuncEntryEvent {
    type Error = ReplayError;
    fn try_from(value: RREvent) -> Result<Self, Self::Error> {
        if let RREvent::CoreHostFuncEntry(x) = value {
            Ok(x)
        } else {
            Err(ReplayError::IncorrectEventVariant)
        }
    }
}

impl From<CoreHostFuncReturnEvent> for RREvent {
    fn from(value: CoreHostFuncReturnEvent) -> Self {
        RREvent::CoreHostFuncReturn(value)
    }
}
impl TryFrom<RREvent> for CoreHostFuncReturnEvent {
    type Error = ReplayError;
    fn try_from(value: RREvent) -> Result<Self, Self::Error> {
        if let RREvent::CoreHostFuncReturn(x) = value {
            Ok(x)
        } else {
            Err(ReplayError::IncorrectEventVariant)
        }
    }
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
            .map(|x| ValRawSer::from(x))
            .collect::<Vec<_>>();

        let event = CoreHostFuncEntryEvent {
            args: values,
            types: None,
        };

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
