//! Private, versioned wire format. All lengths and numbers are little endian.
//!
//! Each record has a one-byte tag and a four-byte payload length. The format is
//! versioned: changing it requires changing MAGIC.

use crate::prelude::*;
use crate::{Trap, ValRaw, ValType};

pub(super) const MAGIC: &[u8; 8] = b"WTRR\0\0\0\x09";
pub(super) const END: u8 = 0;
pub(super) const ENTER_WASM: u8 = 1;
pub(super) const LEAVE_WASM: u8 = 2;
pub(super) const ENTER_HOST: u8 = 3;
pub(super) const LEAVE_HOST: u8 = 4;
pub(super) const WRITE: u8 = 5;
pub(super) const RESIZE: u8 = 6;

pub(super) const HOST: u8 = 7;
/// A module's first instantiation: `[id, SHA-256 of its bytecode]`.
pub(super) const MODULE: u8 = 8;
pub(super) const INSTANCE: u8 = 9;
pub(super) const GLOBAL: u8 = 10;
pub(super) const MEMORY: u8 = 11;
pub(super) const TABLE: u8 = 12;

const UNSUPPORTED: &str = "record/replay does not support GC or typed reference boundaries";

// Reference-valued globals carry a nullable flag and a function ID.
pub(super) const FUNCREF: u8 = Kind::FuncRef as u8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum Kind {
    I32,
    I64,
    F32,
    F64,
    V128,
    FuncRef,
    /// GC and typed references, which functions may use internally but which
    /// cannot cross the record/replay boundary.
    Unsupported,
}

impl Kind {
    pub fn new(ty: ValType) -> Result<Self> {
        Ok(match ty {
            ValType::I32 => Self::I32,
            ValType::I64 => Self::I64,
            ValType::F32 => Self::F32,
            ValType::F64 => Self::F64,
            ValType::V128 => Self::V128,
            ValType::Ref(r) if r.is_nullable() && r.heap_type().is_func() => Self::FuncRef,
            ValType::Ref(_) => Self::Unsupported,
        })
    }

    pub fn size(self) -> usize {
        match self {
            Self::I32 | Self::F32 | Self::FuncRef => 4,
            Self::I64 | Self::F64 => 8,
            Self::V128 => 16,
            Self::Unsupported => 0,
        }
    }
}

pub(super) fn reserve(bytes: &mut Vec<u8>, additional: usize) -> Result<()> {
    bytes
        .try_reserve(additional)
        .map_err(|_| OutOfMemory::new(additional))?;
    Ok(())
}

pub(super) fn blob(bytes: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    let len = u32::try_from(value.len())?;
    reserve(
        bytes,
        4usize
            .checked_add(value.len())
            .ok_or_else(|| format_err!("trace length overflow"))?,
    )?;
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(())
}

pub(super) fn record(bytes: &mut Vec<u8>, tag: u8, len: usize) -> Result<()> {
    let len32 = u32::try_from(len)?;
    reserve(
        bytes,
        len.checked_add(5)
            .ok_or_else(|| format_err!("trace length overflow"))?,
    )?;
    bytes.push(tag);
    bytes.extend_from_slice(&len32.to_le_bytes());
    Ok(())
}

/// Only reads the initialized part of each slot, as specified by its type.
pub(super) unsafe fn values(
    bytes: &mut Vec<u8>,
    kinds: &[Kind],
    raw: *const ValRaw,
    mut encode_ref: impl FnMut(*mut core::ffi::c_void) -> Result<u32>,
) -> Result<()> {
    for (i, kind) in kinds.iter().enumerate() {
        // SAFETY: the caller guarantees these slots have the specified types.
        let value = unsafe { &*raw.add(i) };
        match kind {
            Kind::I32 => bytes.extend_from_slice(&value.get_i32().to_le_bytes()),
            Kind::I64 => bytes.extend_from_slice(&value.get_i64().to_le_bytes()),
            Kind::F32 => bytes.extend_from_slice(&value.get_f32().to_le_bytes()),
            Kind::F64 => bytes.extend_from_slice(&value.get_f64().to_le_bytes()),
            Kind::V128 => bytes.extend_from_slice(&value.get_v128().to_le_bytes()),
            Kind::FuncRef => {
                bytes.extend_from_slice(&encode_ref(value.get_funcref())?.to_le_bytes())
            }
            Kind::Unsupported => bail!(UNSUPPORTED),
        }
    }
    Ok(())
}

pub(super) fn values_len(kinds: &[Kind]) -> usize {
    kinds.iter().map(|k| k.size()).sum()
}

/// `raw` contains initialized slots of `kinds` when `result` is successful.
pub(super) unsafe fn outcome(
    bytes: &mut Vec<u8>,
    tag: u8,
    call: usize,
    result: &Result<()>,
    kinds: &[Kind],
    raw: *const ValRaw,
    encode_ref: impl FnMut(*mut core::ffi::c_void) -> Result<u32>,
) -> Result<()> {
    let call = u32::try_from(call)?.to_le_bytes();
    match result {
        Ok(()) => {
            record(bytes, tag, 5 + values_len(kinds))?;
            bytes.extend_from_slice(&call);
            bytes.push(0);
            // SAFETY: callers supply initialized results only on success.
            unsafe { values(bytes, kinds, raw, encode_ref)? };
        }
        Err(e) => {
            if let Some(trap) = e.downcast_ref::<Trap>() {
                record(bytes, tag, 6)?;
                bytes.extend_from_slice(&call);
                bytes.extend_from_slice(&[1, *trap as u8]);
            } else {
                // Context, such as a Wasm backtrace, depends on the activation's
                // callers and runtime configuration, so only the root cause of
                // an error is reproduced.
                let message = e.root_cause().to_string();
                record(bytes, tag, 5 + message.len())?;
                bytes.extend_from_slice(&call);
                bytes.push(2);
                bytes.extend_from_slice(message.as_bytes());
            }
        }
    }
    Ok(())
}

pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(len)
            .ok_or_else(|| format_err!("trace length overflow"))?;
        let result = self.bytes.get(self.position..end).ok_or_else(|| {
            format_err!("truncated record/replay trace at byte {}", self.position)
        })?;
        self.position = end;
        Ok(result)
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    pub fn blob(&mut self) -> Result<&'a [u8]> {
        let len = usize::try_from(self.u32()?)?;
        self.take(len)
    }

    pub fn checksum(&mut self) -> Result<super::Checksum> {
        Ok(self.take(32)?.try_into().unwrap())
    }

    pub fn record(&mut self) -> Result<(u8, Reader<'a>)> {
        let tag = self.u8()?;
        let body = self.blob()?;
        Ok((tag, Reader::new(body)))
    }

    pub fn end(&self) -> Result<()> {
        ensure!(
            self.position == self.bytes.len(),
            "unexpected trailing trace data"
        );
        Ok(())
    }
}
