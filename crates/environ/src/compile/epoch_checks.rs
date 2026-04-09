//! Emission of compiled-artifact metadata describing where epoch-end checks
//! occur in the code when using MMU-based epoch interruption. This lets the
//! signal handler distinguish epoch interruptions from general segfaults.

use crate::obj::ELF_WASMTIME_EPOCH_CHECKS;
use crate::prelude::*;
use object::write::{Object, StandardSegment};
use object::{LittleEndian, SectionKind, U32Bytes};
use std::ops::Range;

/// Offset of an epoch check within its function, in bytes. Specifically, this
/// points to The instruction after the one that does the epoch-end check: the
/// one at which to resume execution.
///
/// This is parallel to cranelift's CodeOffset and exists to (1) avoid making it
/// a dependency, (2) pin it down to <= 32 bits, since the format of the
/// custom-section we're emitting depends on that, and (3) hold documentation.
type EpochCheckOffset = u32;

/// A builder and emitter of the custom section which houses MMU-based
/// epoch-end-check locations in a native binary.
#[derive(Default)]
pub struct EpochCheckSection {
    /// Offset of the instruction to resume at after the epoch end and task switch
    return_offsets: Vec<U32Bytes<LittleEndian>>,
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
    /// `check_offsets` must be ordered (within each function) as well.
    pub fn push(&mut self, func: Range<u64>, check_offsets: &[EpochCheckOffset]) {
        // Check that functions have been pushed in order so our section is
        // sorted for free.
        let func_start = u32::try_from(func.start).unwrap();
        let func_end = u32::try_from(func.end).unwrap();
        assert!(func_start >= self.last_offset);

        // Remember each offset, ensuring they are in order.
        for offset in check_offsets {
            let text_section_relative = func_start + offset;
            assert!(text_section_relative > self.last_offset);
            self.return_offsets
                .push(U32Bytes::new(LittleEndian, text_section_relative));
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

        // Append length.
        obj.append_section_data(
            section,
            &u32::try_from(self.return_offsets.len())
                .unwrap()
                .to_le_bytes(),
            1,
        );

        // Append offsets.
        obj.append_section_data(section, object::bytes_of_slice(&self.return_offsets), 1);
    }
}
