//! Helpers for workign with encoding/decoding bytes w.r.t. wasmtime's
//! custom-encoded sections.

use alloc::vec::Vec;

/// Writes the uleb-encoded `value` to `data`.
pub fn write_uleb(data: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        data.push(0x80 | (value as u8 & 0x7f));
        value >>= 7;
    }
    data.push(value as u8);
}

/// Writes the sleb-encoded `value` to `data`.
pub fn write_sleb(data: &mut Vec<u8>, mut value: i64) {
    loop {
        let byte = value.cast_unsigned() as u8 & 0x7f;
        value >>= 7;
        // Termination requires that the remaining bits of `value` all match
        // the encoded sign bit, i.e. that sign extension of what's been
        // written reproduces `value` exactly.
        let done = (value == 0 && byte & 0x40 == 0) || (value == -1 && byte & 0x40 != 0);
        if done {
            data.push(byte);
            return;
        }
        data.push(byte | 0x80);
    }
}

/// Reads a uleb-encoded value from `data`, returning the value and consuming
/// the bytes read from `data`. Returns `None` if the encoding is invalid.
pub fn read_uleb(data: &mut &[u8]) -> Option<u64> {
    let mut result = 0;
    let mut shift = 0;
    while shift < 64 {
        let byte = pop(data)?;
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
    }
    None
}

/// Reads a sleb-encoded value from `data`, returning the value and consuming
/// the bytes read from `data`. Returns `None` if the encoding is invalid.
pub fn read_sleb(data: &mut &[u8]) -> Option<i64> {
    let mut result = 0;
    let mut shift = 0;
    while shift < 64 {
        let byte = pop(data)?;
        result |= i64::from(byte & 0x7f) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            // Sign-extend from the topmost bit that was encoded.
            if shift < 64 && byte & 0x40 != 0 {
                result |= -1 << shift;
            }
            return Some(result);
        }
    }
    None
}

/// Pops a single byte from the front of `data`, returning it and consuming it
/// from `data`. Returns `None` if `data` is empty.
pub fn pop(data: &mut &[u8]) -> Option<u8> {
    let (&byte, rest) = data.split_first()?;
    *data = rest;
    Some(byte)
}
