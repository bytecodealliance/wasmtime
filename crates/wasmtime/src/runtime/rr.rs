//! Wasmtime's Record and Replay support

use crate::ValRaw;
use crate::prelude::*;
use core::fmt;
use core::mem::{self, MaybeUninit};
use postcard;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufWriter, Seek, Write};

const VAL_RAW_SIZE: usize = mem::size_of::<ValRaw>();

/// Transmutable byte arrays necessary to serialize unions
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

/// A single recording/replay event
#[derive(Debug, Serialize, Deserialize)]
pub enum RREvent {
    ExternCall(Vec<ValRawSer>),
    ExternReturn(Vec<ValRawSer>),
}

impl RREvent {
    fn raw_to_vec(args: &[MaybeUninit<ValRaw>]) -> Vec<ValRawSer> {
        args.iter()
            .map(|x| unsafe { ValRawSer::from(x.assume_init()) })
            .collect::<Vec<_>>()
    }

    pub fn extern_call_from_valraw_slice(args: &[MaybeUninit<ValRaw>]) -> Self {
        Self::ExternCall(Self::raw_to_vec(args))
    }
    pub fn extern_return_from_valraw_slice(args: &[MaybeUninit<ValRaw>]) -> Self {
        Self::ExternReturn(Self::raw_to_vec(args))
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
        while file.stream_position()? != file.metadata()?.len() {
            let (event, _): (RREvent, _) = postcard::from_io((&mut file, &mut [0; 0]))?;
            events.push_back(event);
        }
        // Check that file is at EOF
        //assert_eq!(file.stream_position()?, file.metadata()?.len());
        println!("Read from file: {:?}", events);
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
        println!("Flushing to file: {:?}", self.inner);
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
