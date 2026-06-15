//! Emission of compiled-artifact metadata describing where epoch-end checks
//! occur in the code when using MMU-based epoch interruption. This lets the
//! signal handler distinguish epoch interruptions from general segfaults.

use crate::obj::ELF_WASMTIME_EPOCH_CHECKS;
use crate::prelude::*;
use object::SectionKind;
use object::write::{Object, StandardSegment};
use std::ops::Range;

/// Offset of an epoch check (in bytes) within the text section. Specifically,
/// this points to the load instruction that may trigger the epoch-ending
/// segfault.
///
/// This is parallel to cranelift's CodeOffset and exists to (1) avoid making it
/// a dependency, (2) pin it down to <= 32 bits, since the format of the
/// custom-section we're emitting depends on that, and (3) hold documentation.
type EpochCheckOffset = u32;

/// A builder and emitter of the custom section which houses MMU-based
/// epoch-end-check locations in a native binary.
#[derive(Default)]
pub struct EpochCheckSection {
    /// Offset of the start of the load instruction which effects the epoch
    /// check
    starts: Vec<u32>,
    /// Packed bits, parallel to elements of `starts`, which tell the length of
    /// the load instruction: 0 if 3 bytes, 1 if 4
    ends: Vec<u8>,
    /// The largest (and most recent, because we accept them only in order)
    /// offset received so far for enforcing ordering. This is relative to the
    /// start of the code (text) section so we can make sure functions are
    /// ordered too.
    last_offset: u32,
}

impl EpochCheckSection {
    /// Adds an epoch-check location to the section.
    ///
    /// Calls to this must be ordered by the location of `func`, and
    /// offsets must be ordered (within each function) as well.
    pub fn push(&mut self, func: Range<u64>, checks: &[Range<EpochCheckOffset>]) {
        // Check that functions have been pushed in order so our section is
        // sorted for free.
        let func_start = u32::try_from(func.start).unwrap();
        let func_end = u32::try_from(func.end).unwrap();
        assert!(func_start >= self.last_offset);

        // Remember each offset, ensuring they are in order.
        for check in checks {
            let text_section_relative = func_start + check.start;
            assert!(text_section_relative > self.last_offset);
            let bit_number = self.starts.len();
            self.starts.push(text_section_relative);
            let load_is_long = match check.len() {
                3 => false,
                4 => true,
                _ => panic!(
                    "Unexpected length of epoch-checking load instruction: {}",
                    check.len()
                ),
            };
            set_bit(&mut self.ends, bit_number, load_is_long);
            self.last_offset = text_section_relative;
        }
        self.last_offset = func_end;
    }

    /// Encodes this section into an object.
    pub fn append_to(self, obj: &mut Object) {
        let section = obj.add_section(
            obj.segment_name(StandardSegment::Data).to_vec(),
            ELF_WASMTIME_EPOCH_CHECKS.as_bytes().to_vec(),
            SectionKind::ReadOnlyData,
        );

        let num_checks: u32 = self.starts.len().try_into()
            .expect("there should be few enough epoch checks to be indexed by a u32, as the Wasm itself is only that long");
        // Number of epoch checks:
        obj.append_section_data(
            section,
            object::bytes_of(&num_checks),
            4, // For speed, avoid splitting across a cache line.
        );
        // Align to 4 so we can use from_raw_parts() when reading:
        obj.append_section_data(section, object::bytes_of_slice(&self.starts), 4);
        // We'll be querying this a byte at a time so don't care about alignment:
        obj.append_section_data(section, object::bytes_of_slice(&self.ends), 1);
    }
}

/// Returns the offset at which to resume after the given epoch check.
///
/// If there is no epoch check at that offset, return None.
pub fn return_offset_for_epoch_check(
    section: &[u8],
    check: EpochCheckOffset,
) -> Option<EpochCheckOffset> {
    let (num_checks, rest) = object::from_bytes::<u32>(section)
        .expect(".wasmtime.epochchecks section should be long enough to contain count");
    let (starts, ends) = object::slice_from_bytes::<u32>(rest, *num_checks as usize)
        .expect(".wasmtime.epochchecks section should be long enough to contain starts");

    starts
        .binary_search(&check)
        .ok()
        .map(|epoch_index| check + (if get_bit(ends, epoch_index) { 4 } else { 3 }))
}

/// Sets bit `dest_bit_number` (0-based) in `dest_slice` to `value`.
fn set_bit(dest_slice: &mut Vec<u8>, dest_bit_number: usize, value: bool) {
    let byte = dest_bit_number / 8;
    if byte >= dest_slice.len() {
        dest_slice.resize(byte + 1, 0);
    }
    let bit = dest_bit_number % 8;
    if value {
        dest_slice[byte] |= 1 << bit;
    } else {
        dest_slice[byte] &= !(1 << bit);
    }
}

/// Returns bit `bit_number` (0-based) of `bytes`.
fn get_bit(bytes: &[u8], bit_number: usize) -> bool {
    let byte = bit_number / 8;
    let bit = bit_number % 8;
    bytes[byte] & (1 << bit) != 0
}

#[test]
fn test_get_and_set_bit() {
    let mut bits = vec![];
    set_bit(&mut bits, 0, true); // first
    set_bit(&mut bits, 2, true); // middle
    set_bit(&mut bits, 18, true); // something beyond first byte
    assert_eq!(bits, vec![5, 0, 4]);
    assert!(get_bit(&bits, 0));
    assert!(get_bit(&bits, 18));
    assert!(!get_bit(&bits, 3));
}

#[test]
fn test_set_bit_clear() {
    let mut bits = vec![0xFF];
    set_bit(&mut bits, 1, false);
    assert_eq!(bits, vec![0xFD]);
}
