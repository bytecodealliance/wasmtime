//! Wasmtime's Record and Replay support
//!
//! This feature is currently experimental and hence not optimized.
//! In particular, the following opportunities are immediately identifiable:
//! * Switch [RRFuncArgTypes] to use [Vec<WasmValType>]
//!
//! Flexibility can also be improved with:
//! * Support for generic writers beyond [File] (will require a generic on [Store])

use crate::ValRaw;
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

/// Buffer to read/write record/replay data respectively
#[derive(Debug)]
pub struct RRBuffer {
    inner: VecDeque<RREvent>,
    rw: File,
}

impl RRBuffer {
    /// Constructs a writer on new, filesystem-backed buffer (record)
    pub fn write_fs(path: String) -> Result<Self> {
        Ok(RRBuffer {
            inner: VecDeque::new(),
            rw: File::create(path)?,
        })
    }

    /// Constructs a reader on filesystem-backed buffer (replay)
    pub fn read_fs(path: String) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut events = VecDeque::<RREvent>::new();
        // Read till EOF
        while file.stream_position()? != file.metadata()?.len() {
            let (event, _): (RREvent, _) = postcard::from_io((&mut file, &mut [0; 0]))?;
            events.push_back(event);
        }
        Ok(RRBuffer {
            inner: events,
            rw: file,
        })
    }

    /// Appends a new [`RREvent`] to the buffer (record)
    pub fn append(&mut self, event: RREvent) {
        self.inner.push_back(event)
    }

    /// Retrieve the head of the buffer (replay)
    pub fn pop_front(&mut self) -> RREvent {
        self.inner
            .pop_front()
            .expect("Incomplete replay trace. Event buffer is empty prior to completion")
    }

    /// Flush all the contents of the entire buffer to a writer
    ///
    /// Buffer is emptied during this process
    pub fn flush_to_file(&mut self) -> Result<()> {
        // Seralizing each event independently prevents checking for vector sizes
        // during deserialization
        for v in &self.inner {
            postcard::to_io(&v, &mut self.rw)?;
        }
        self.rw.flush()?;
        self.inner.clear();
        Ok(())
    }
}
