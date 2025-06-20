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
use wasmtime_environ::{WasmFuncType, WasmValType};

const VAL_RAW_SIZE: usize = mem::size_of::<ValRaw>();

type RRFuncArgVals = Vec<ValRawSer>;
type RRFuncArgTypes = WasmFuncType;

fn raw_to_func_argvals(args: &[MaybeUninit<ValRaw>]) -> RRFuncArgVals {
    args.iter()
        .map(|x| unsafe { ValRawSer::from(x.assume_init()) })
        .collect::<Vec<_>>()
}

#[derive(Debug)]
pub enum ReplayError {
    EmptyBuffer,
    FailedValidation,
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBuffer => {
                write!(f, "replay buffer is empty!")
            }
            Self::FailedValidation => {
                write!(f, "replay event validation check failed")
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
    fn pop_event(&mut self) -> Result<RREvent>;
}

/// Transmutable byte array used to serialize [`ValRaw`] union
#[derive(Serialize, Deserialize)]
pub struct ValRawSer([u8; VAL_RAW_SIZE]);

impl From<ValRaw> for ValRawSer {
    fn from(value: ValRaw) -> Self {
        unsafe { Self(mem::transmute(value)) }
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

/// Arguments for function call/return events
#[derive(Debug, Serialize, Deserialize)]
pub struct RRFuncArgs {
    /// Raw values passed across the call/return boundary
    args: RRFuncArgVals,
    /// Optional param/return types (required to support replay validation)
    types: Option<RRFuncArgTypes>,
}

/// A single, low-level recording/replay event
///
/// A high-level event (e.g. import calls consisting of lifts and lowers
/// of parameter/return types) may consist of multiple of these lower-level
/// [`RREvent`]s
#[derive(Debug, Serialize, Deserialize)]
pub enum RREvent {
    /// A function call from Wasm to Host
    HostFuncEntry(RRFuncArgs),
    /// A function return from Host to Wasm.
    ///
    /// Matches 1:1 with a prior [`RREvent::HostFuncEntry`] event
    HostFuncReturn(RRFuncArgs),
}

impl RREvent {
    /// Construct a [`RREvent::HostFuncEntry`] event from raw slice
    pub fn host_func_entry(args: &[MaybeUninit<ValRaw>], types: Option<WasmFuncType>) -> Self {
        Self::HostFuncEntry(RRFuncArgs {
            args: raw_to_func_argvals(args),
            types: types,
        })
    }
    /// Construct a [`RREvent::HostFuncReturn`] event from raw slice
    pub fn host_func_return(args: &[MaybeUninit<ValRaw>], types: Option<WasmFuncType>) -> Self {
        Self::HostFuncReturn(RRFuncArgs {
            args: raw_to_func_argvals(args),
            types: types,
        })
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
        println!("Record flush: {:?} bytes", data.rw.metadata()?.len());
        Ok(())
    }

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

    fn pop_event(&mut self) -> Result<RREvent> {
        self.data
            .buf
            .pop_front()
            .ok_or(Self::ReplayError::EmptyBuffer.into())
    }
}
