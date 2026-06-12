use crate::TrapInformation;
use crate::bytes::write_uleb;
use crate::obj::ELF_WASMTIME_TRAPS;
use crate::prelude::*;
use crate::trap_encoding::TRAP_BLOCK_SIZE;
use object::write::{Object, StandardSegment};
use object::{LittleEndian, SectionKind, U32};
use std::ops::Range;

/// A helper structure to build the custom-encoded section of a wasmtime
/// compilation image which encodes trap information.
///
/// This structure is incrementally fed the results of compiling individual
/// functions and handles all the encoding internally, allowing usage of
/// `lookup_trap_code` with the resulting section.
///
/// # Section format
///
/// The section encodes a sequence of `(text_offset, trap_code)` entries,
/// sorted by `text_offset`, where `text_offset` is the location of a
/// trapping instruction relative to the start of the text section and
/// `trap_code` is the byte encoding of its `CompiledTrap`. This format is
/// optimized to enable cheap (O(log n)) lookup given an offset to find a trap
/// code while also being relatively compact as this is included in all modules
/// by default. To satisfy this the section is encoded as two major pieces: an
/// index and a sequence of blocks.
///
/// The index is used to perform a binary search given a particular
/// `text_offset` to find a particular block. The index stores text offsets as
/// well as byte offsets in the "block bodies" section. Once a block is found
/// each block contains up to `TRAP_BLOCK_SIZE` entries encoded next to each
/// other. Blocks take up a variable width of bytes to encode. More information
/// on decoding each block is below, but the general layout of the section looks
/// like:
///
/// ```text
/// ┌───────────────────────────────────┐
/// │ entry_count: u32                  │
/// │ block_count: u32                  │
/// ├───────────────────────────────────┤
/// │ block index                       │
/// │ ┌───────────────────────────────┐ │
/// │ │ first_offset: u32             │ │  one pair per block, sorted by
/// │ │ data_pos: u32                 │ │  `first_offset`; `data_pos` is
/// │ ├───────────────────────────────┤ │  relative to the start of the
/// │ │ ...                           │ │  block bodies area below
/// │ └───────────────────────────────┘ │
/// ├───────────────────────────────────┤
/// │ block bodies                      │
/// │ ┌───────────────────────────────┐ │
/// │ │ default_code: u8              │ │
/// │ ├───────────────────────────────┤ │
/// │ │ entry: uleb token             │ │  one entry per trap in the
/// │ │ [trap_code: u8]               │ │  block, `TRAP_BLOCK_SIZE` max
/// │ ├───────────────────────────────┤ │
/// │ │ ...                           │ │
/// │ └───────────────────────────────┘ │
/// │ ┌───────────────────────────────┐ │
/// │ │ ...                           │ │
/// │ └───────────────────────────────┘ │
/// └───────────────────────────────────┘
/// ```
///
/// * `entry_count` is the total number of entries (pc/trap combos) in the
///   section and `block_count` is the number of blocks, `ceil(entry_count /
///   TRAP_BLOCK_SIZE)`.
/// * In the block index, `first_offset` is the `text_offset` of the block's
///   first entry and `data_pos` is the position of the block's body,
///   relative to the start of the bodies area (i.e. the end of the index).
/// * Each block body starts with `default_code`, the block's "default" trap
///   code, chosen as the most common code among the block's entries.
/// * Each entry is a uleb-encoded token `(pc_delta << 1) | code_differs`.
///   Here `pc_delta` is this entry's `text_offset` minus the previous
///   entry's (the first entry's delta is relative to the block's
///   `first_offset` and is therefore 0). If `code_differs` is set the token
///   is followed by one byte holding this entry's trap code, otherwise the
///   entry has the block's default code.
///
/// Lookup (`lookup_trap_code`) binary searches the fixed-width block index
/// for the last block whose `first_offset` is `<=` the pc in question, then
/// linearly decodes at most `TRAP_BLOCK_SIZE` entries of that block's body
/// looking for an exact match.
///
/// This encoding leans on two properties of trap metadata: consecutive trap
/// sites are generally close together (pc deltas almost always fit in a
/// single-byte leb) and most entries share one trap code (typically
/// `MemoryOutOfBounds` for gc-less wasm), making explicit code bytes rare. This
/// is all in service of shrinking the minimum 5 bytes per entry (u32 offset, u8
/// code), to a bit more than one byte per entry in practice.
///
/// Note that at this time this section has an alignment of 1. Additionally
/// due to the 32-bit offsets in the block index this doesn't support images
/// >= 4GB.
#[derive(Default)]
pub struct TrapEncodingBuilder {
    entries: usize,
    block_index: Vec<[U32<LittleEndian>; 2]>,
    block_bodies: Vec<u8>,
    pending: Vec<(u32, u8)>,
    last_offset: u32,
}

impl TrapEncodingBuilder {
    /// Appends trap information about a function into this section.
    ///
    /// This function is called to describe traps for the `func` range
    /// specified. The `func` offsets are specified relative to the text section
    /// itself, and the `traps` offsets are specified relative to the start of
    /// `func`.
    ///
    /// This is required to be called in-order for increasing ranges of `func`
    /// to ensure the final array is properly sorted. Additionally `traps` must
    /// be sorted.
    pub fn push(&mut self, func: Range<u64>, traps: &[TrapInformation]) {
        // NB: for now this only supports <=4GB text sections in object files.
        // Alternative schemes will need to be created for >32-bit offsets to
        // avoid making this section overly large.
        let func_start = u32::try_from(func.start).unwrap();
        let func_end = u32::try_from(func.end).unwrap();

        // Sanity-check to ensure that functions are pushed in-order, otherwise
        // the encoded blocks won't be sorted which is our goal.
        assert!(func_start >= self.last_offset);

        for info in traps {
            let pos = func_start + info.code_offset;
            assert!(pos >= self.last_offset);
            self.pending.push((pos, info.trap_code.as_u8()));
            self.entries += 1;
            self.last_offset = pos;
            if self.pending.len() == TRAP_BLOCK_SIZE {
                self.seal_block();
            }
        }

        self.last_offset = func_end;
    }

    /// Flushes `self.pending` into one encoded block, appending to the index
    /// and data arrays.
    fn seal_block(&mut self) {
        let first_offset = match self.pending.first() {
            Some((offset, _)) => *offset,
            None => return,
        };
        let body_pos = u32::try_from(self.block_bodies.len()).unwrap();
        self.block_index.push([
            U32::new(LittleEndian, first_offset),
            U32::new(LittleEndian, body_pos),
        ]);

        // The block's default code is its most common one, making the common
        // case of a run of identical codes free to encode.
        let default_code = most_common_code(&self.pending);
        self.block_bodies.push(default_code);

        let mut prev = first_offset;
        for (pc, code) in self.pending.drain(..) {
            let delta = pc - prev;
            prev = pc;
            let differs = code != default_code;
            write_uleb(
                &mut self.block_bodies,
                (u64::from(delta) << 1) | u64::from(differs),
            );
            if differs {
                self.block_bodies.push(code);
            }
        }
    }

    /// Encodes this section into the object provided.
    pub fn append_to(self, obj: &mut Object) {
        let section = obj.add_section(
            obj.segment_name(StandardSegment::Data).to_vec(),
            ELF_WASMTIME_TRAPS.as_bytes().to_vec(),
            SectionKind::ReadOnlyData,
        );

        obj.append_section_data(section, &self.finish(), 1);
    }

    /// Finishes encoding and returns the raw section contents, as decoded by
    /// `lookup_trap_code` and `iterate_traps`.
    fn finish(mut self) -> Vec<u8> {
        self.seal_block();
        let entries = u32::try_from(self.entries).unwrap();
        let num_blocks = u32::try_from(self.block_index.len()).unwrap();
        let mut ret = Vec::with_capacity(8 + self.block_index.len() * 8 + self.block_bodies.len());
        ret.extend_from_slice(&entries.to_le_bytes());
        ret.extend_from_slice(&num_blocks.to_le_bytes());
        ret.extend_from_slice(object::bytes_of_slice(&self.block_index));
        ret.extend_from_slice(&self.block_bodies);
        ret
    }
}

fn most_common_code(entries: &[(u32, u8)]) -> u8 {
    let mut counts = [0u16; 256];
    let mut best = entries[0].1;
    for (_, code) in entries {
        let count = &mut counts[usize::from(*code)];
        *count += 1;
        if *count > counts[usize::from(best)] {
            best = *code;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Trap, iterate_traps, lookup_trap_code};

    fn encode(funcs: &[(Range<u64>, &[TrapInformation])]) -> Vec<u8> {
        let mut builder = TrapEncodingBuilder::default();
        for (func, traps) in funcs {
            builder.push(func.clone(), traps);
        }
        builder.finish()
    }

    fn info(code_offset: u32, trap: Trap) -> TrapInformation {
        TrapInformation {
            code_offset,
            trap_code: trap.into(),
        }
    }

    #[test]
    fn smoke() {
        let section = encode(&[]);
        assert_eq!(lookup_trap_code(&section, 0), None);
        assert_eq!(iterate_traps(&section).unwrap().count(), 0);

        let section = encode(&[(0..0x100, &[])]);
        assert_eq!(lookup_trap_code(&section, 0x50), None);
        assert_eq!(iterate_traps(&section).unwrap().count(), 0);

        let section = encode(&[(
            0..0x100,
            &[
                info(10, Trap::MemoryOutOfBounds),
                info(20, Trap::StackOverflow),
            ],
        )]);
        assert_eq!(lookup_trap_code(&section, 0x50), None);
        assert_eq!(
            lookup_trap_code(&section, 10),
            Some(Trap::MemoryOutOfBounds.into())
        );
        assert_eq!(
            lookup_trap_code(&section, 20),
            Some(Trap::StackOverflow.into())
        );
    }
}
