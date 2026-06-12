//! Data structures to provide transformation of the source

use crate::InstructionAddressMap;
use crate::address_map::ADDRMAP_BLOCK_SIZE;
use crate::bytes::{write_sleb, write_uleb};
use crate::obj::ELF_WASMTIME_ADDRMAP;
use crate::prelude::*;
use object::write::{Object, StandardSegment};
use object::{LittleEndian, SectionKind, U32};
use std::ops::Range;

/// Builder for the address map section of a wasmtime compilation image.
///
/// This builder is used to conveniently build the `ELF_WASMTIME_ADDRMAP`
/// section by compilers, and provides utilities to directly insert the results
/// into an `Object`.
///
/// # Section format
///
/// The section encodes a sequence of `(text_offset, file_pos)` entries, sorted
/// by `text_offset`, where `text_offset` is the location of an instruction
/// relative to the start of the text section and `file_pos` is the offset
/// within the original wasm file of the instruction it was compiled from, or
/// the `FilePos::none()` sentinel for generated code with no wasm-level
/// source. Unlike the trap section each entry here describes a range of pcs:
/// an entry covers addresses from its own `text_offset` up to the next
/// entry's. This format is optimized to enable cheap (O(log n)) lookup given
/// an offset to find a source location while also being relatively compact as
/// this is included in all modules by default and is, uncompressed, the
/// largest of Wasmtime's metadata sections. To satisfy this the section is
/// encoded as two major pieces: an index and a sequence of blocks.
///
/// The index is used to perform a binary search given a particular
/// `text_offset` to find a particular block. The index stores text offsets as
/// well as byte offsets in the "block bodies" section. Once a block is found
/// each block contains up to `ADDRMAP_BLOCK_SIZE` entries encoded next to each
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
/// │ │ block_pos: u32                │ │  `first_offset`; `block_pos` is
/// │ ├───────────────────────────────┤ │  relative to the start of the
/// │ │ ...                           │ │  block bodies area below
/// │ └───────────────────────────────┘ │
/// ├───────────────────────────────────┤
/// │ block bodies                      │
/// │ ┌───────────────────────────────┐ │
/// │ │ entry: uleb token             │ │  one entry per instruction
/// │ │ [file_pos: uleb]              │ │  mapping in the block,
/// │ ├───────────────────────────────┤ │  `ADDRMAP_BLOCK_SIZE` max
/// │ │ ...                           │ │
/// │ └───────────────────────────────┘ │
/// │ ┌───────────────────────────────┐ │
/// │ │ ...                           │ │
/// │ └───────────────────────────────┘ │
/// └───────────────────────────────────┘
/// ```
///
/// * `entry_count` is the total number of entries (pc/srcloc combos) in the
///   section and `block_count` is the number of blocks, `ceil(entry_count /
///   ADDRMAP_BLOCK_SIZE)`.
/// * In the block index, `first_offset` is the `text_offset` of the block's
///   first entry and `block_pos` is the position of the block's body,
///   relative to the start of the bodies area (i.e. the end of the index).
/// * Each entry is a uleb-encoded token `(pc_delta << 1) | pos_is_none`.
///   Here `pc_delta` is this entry's `text_offset` minus the previous
///   entry's (the first entry's delta is relative to the block's
///   `first_offset` and is therefore 0). If `pos_is_none` is set this entry's
///   file position is `FilePos::none()` and nothing else follows the token.
///   Otherwise the token is followed by the entry's file position: the first
///   non-none position in a block is uleb-encoded absolutely and subsequent
///   positions are sleb-encoded deltas from the previous non-none position.
///   Delta chains restart at each block so blocks can be decoded
///   independently.
///
/// Lookup (`lookup_file_pos`) binary searches the fixed-width block index for
/// the last block whose `first_offset` is `<=` the pc in question, then
/// linearly decodes at most `ADDRMAP_BLOCK_SIZE` entries of that block's body
/// looking for the last entry whose `text_offset` is `<=` the pc.
///
/// This encoding leans on a few properties of address map metadata:
/// consecutive instructions are close together (pc deltas almost always fit in
/// a single-byte leb), consecutive source locations are close together and
/// mostly increasing (position deltas almost always fit in a single-byte
/// sleb), and entries with no source location are common (a quarter of all
/// entries) and cost only the token's flag bit. This is all in service of shrinking the
/// previous 8 bytes per entry (u32 offset, u32 file position) to roughly 2
/// bytes per entry in practice.
///
/// Note that at this time this section has an alignment of 1. Additionally
/// due to the 32-bit offsets in the block index this doesn't support images
/// >= 4GB.
#[derive(Default)]
pub struct AddressMapSection {
    entries: usize,
    block_index: Vec<[U32<LittleEndian>; 2]>,
    block_bodies: Vec<u8>,
    pending: Vec<(u32, u32)>,
    last_offset: u32,
}

impl AddressMapSection {
    /// Pushes a new set of instruction mapping information for a function added
    /// in the executable.
    ///
    /// The `func` argument here is the range of the function, relative to the
    /// start of the text section in the executable. The `instrs` provided are
    /// the descriptors for instructions in the function and their various
    /// mappings back to original source positions.
    ///
    /// This is required to be called for `func` values that are strictly
    /// increasing in addresses (e.g. as the object is built). Additionally the
    /// `instrs` map must be sorted based on code offset in the native text
    /// section.
    pub fn push(&mut self, func: Range<u64>, instrs: &[InstructionAddressMap]) {
        // NB: for now this only supports <=4GB text sections in object files.
        // Alternative schemes will need to be created for >32-bit offsets to
        // avoid making this section overly large.
        let func_start = u32::try_from(func.start).unwrap();
        let func_end = u32::try_from(func.end).unwrap();

        let mut last_srcloc = None;
        for map in instrs {
            // Sanity-check to ensure that functions are pushed in-order, otherwise
            // the encoded blocks won't be sorted which is our goal.
            let pos = func_start + map.code_offset;
            assert!(pos >= self.last_offset);
            self.last_offset = pos;

            // Drop duplicate instruction mappings that match what was
            // previously pushed into the array since the representation used
            // here will naturally cover `pos` with the previous entry.
            let srcloc = map.srcloc.file_offset().unwrap_or(u32::MAX);
            if Some(srcloc) == last_srcloc {
                continue;
            }
            last_srcloc = Some(srcloc);

            self.pending.push((pos, srcloc));
            self.entries += 1;
            if self.pending.len() == ADDRMAP_BLOCK_SIZE {
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
        let block_pos = u32::try_from(self.block_bodies.len()).unwrap();
        self.block_index.push([
            U32::new(LittleEndian, first_offset),
            U32::new(LittleEndian, block_pos),
        ]);

        let mut prev_offset = first_offset;
        let mut prev_pos = None;
        for (offset, pos) in self.pending.drain(..) {
            let delta = offset - prev_offset;
            prev_offset = offset;
            let is_none = pos == u32::MAX;
            write_uleb(
                &mut self.block_bodies,
                (u64::from(delta) << 1) | u64::from(is_none),
            );
            if is_none {
                continue;
            }
            match prev_pos {
                // The first non-none position of a block is encoded absolutely
                // and subsequent positions are deltas from the previous one,
                // ensuring each block can be decoded independently.
                None => write_uleb(&mut self.block_bodies, u64::from(pos)),
                Some(prev) => write_sleb(&mut self.block_bodies, i64::from(pos) - i64::from(prev)),
            }
            prev_pos = Some(pos);
        }
    }

    /// Finishes encoding this section into the `Object` provided.
    pub fn append_to(self, obj: &mut Object) {
        let section = obj.add_section(
            obj.segment_name(StandardSegment::Data).to_vec(),
            ELF_WASMTIME_ADDRMAP.as_bytes().to_vec(),
            SectionKind::ReadOnlyData,
        );

        obj.append_section_data(section, &self.finish(), 1);
    }

    /// Finishes encoding and returns the raw section contents, as decoded by
    /// `lookup_file_pos` and `iterate_address_map`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FilePos, iterate_address_map, lookup_file_pos};

    fn encode(funcs: &[(Range<u64>, &[InstructionAddressMap])]) -> Vec<u8> {
        let mut builder = AddressMapSection::default();
        for (func, instrs) in funcs {
            builder.push(func.clone(), instrs);
        }
        builder.finish()
    }

    fn map(code_offset: u32, srcloc: FilePos) -> InstructionAddressMap {
        InstructionAddressMap {
            srcloc,
            code_offset,
        }
    }

    #[test]
    fn smoke() {
        let section = encode(&[]);
        assert_eq!(lookup_file_pos(&section, 0), None);
        assert_eq!(iterate_address_map(&section).unwrap().count(), 0);

        let section = encode(&[(0..0x100, &[])]);
        assert_eq!(lookup_file_pos(&section, 0x50), None);
        assert_eq!(iterate_address_map(&section).unwrap().count(), 0);

        let section = encode(&[(
            0..0x100,
            &[
                map(10, FilePos::new(100)),
                map(20, FilePos::none()),
                map(30, FilePos::new(90)),
            ],
        )]);
        // pcs before the first entry have no mapping
        assert_eq!(lookup_file_pos(&section, 9), None);
        // each entry covers pcs from its own offset until the next entry
        assert_eq!(lookup_file_pos(&section, 10), Some(FilePos::new(100)));
        assert_eq!(lookup_file_pos(&section, 19), Some(FilePos::new(100)));
        assert_eq!(lookup_file_pos(&section, 20), Some(FilePos::none()));
        assert_eq!(lookup_file_pos(&section, 29), Some(FilePos::none()));
        assert_eq!(lookup_file_pos(&section, 30), Some(FilePos::new(90)));
        // ... with the last entry covering everything afterwards
        assert_eq!(lookup_file_pos(&section, 0x1000), Some(FilePos::new(90)));
    }

    #[test]
    fn many_blocks() {
        // Enough entries to span multiple blocks, mixing forward and backward
        // source-position movement with `FilePos::none()` entries, including
        // at block boundaries.
        let maps = (0..1000)
            .map(|i| {
                let srcloc = match i % 3 {
                    0 => FilePos::none(),
                    1 => FilePos::new(20_000 + i),
                    _ => FilePos::new(20_000 - i),
                };
                map(i * 3, srcloc)
            })
            .collect::<Vec<_>>();
        let section = encode(&[(0..0x10000, &maps)]);

        let decoded = iterate_address_map(&section).unwrap().collect::<Vec<_>>();
        assert_eq!(decoded.len(), maps.len());
        for (map, (offset, pos)) in maps.iter().zip(&decoded) {
            assert_eq!(*offset, map.code_offset);
            assert_eq!(*pos, map.srcloc);
        }

        // Both an entry's exact pc and a pc inside its bucket resolve to it.
        for map in &maps {
            let offset = usize::try_from(map.code_offset).unwrap();
            assert_eq!(lookup_file_pos(&section, offset), Some(map.srcloc));
            assert_eq!(lookup_file_pos(&section, offset + 1), Some(map.srcloc));
        }
    }
}
