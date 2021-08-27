//! Data structures to provide transformation of the source
// addresses of a WebAssembly module into the native code.

use object::write::{Object, StandardSegment};
use object::{Bytes, LittleEndian, SectionKind, U32Bytes};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::ops::Range;

/// Single source location to generated address mapping.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InstructionAddressMap {
    /// Where in the source wasm binary this instruction comes from, specified
    /// in an offset of bytes from the front of the file.
    pub srcloc: FilePos,

    /// Offset from the start of the function's compiled code to where this
    /// instruction is located, or the region where it starts.
    pub code_offset: u32,
}

/// A position within an original source file,
///
/// This structure is used as a newtype wrapper around a 32-bit integer which
/// represents an offset within a file where a wasm instruction or function is
/// to be originally found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilePos(u32);

impl FilePos {
    /// Create a new file position with the given offset.
    pub fn new(pos: u32) -> FilePos {
        assert!(pos != u32::MAX);
        FilePos(pos)
    }

    /// Returns the offset that this offset was created with.
    ///
    /// Note that the `Default` implementation will return `None` here, whereas
    /// positions created with `FilePos::new` will return `Some`.
    pub fn file_offset(self) -> Option<u32> {
        if self.0 == u32::MAX {
            None
        } else {
            Some(self.0)
        }
    }
}

impl Default for FilePos {
    fn default() -> FilePos {
        FilePos(u32::MAX)
    }
}

/// Builder for the address map section of a wasmtime compilation image.
///
/// This builder is used to conveniently built the `ELF_WASMTIME_ADDRMAP`
/// section by compilers, and provides utilities to directly insert the results
/// into an `Object`.
#[derive(Default)]
pub struct AddressMapSection {
    offsets: Vec<U32Bytes<LittleEndian>>,
    positions: Vec<U32Bytes<LittleEndian>>,
    last_offset: u32,
}

/// A custom Wasmtime-specific section of our compilation image which stores
/// mapping data from offsets in the image to offset in the original wasm
/// binary.
///
/// This section has a custom binary encoding. Currently its encoding is:
///
/// * The section starts with a 32-bit little-endian integer. This integer is
///   how many entries are in the following two arrays.
/// * Next is an array with the previous count number of 32-bit little-endian
///   integers. This array is a sorted list of relative offsets within the text
///   section. This is intended to be a lookup array to perform a binary search
///   on an offset within the text section on this array.
/// * Finally there is another array, with the same count as before, also of
///   32-bit little-endian integers. These integers map 1:1 with the previous
///   array of offsets, and correspond to what the original offset was in the
///   wasm file.
///
/// Decoding this section is intentionally simple, it only requires loading a
/// 32-bit little-endian integer plus some bounds checks. Reading this section
/// is done with the `lookup_file_pos` function below. Reading involves
/// performing a binary search on the first array using the index found for the
/// native code offset to index into the second array and find the wasm code
/// offset.
///
/// At this time this section has an alignment of 1, which means all reads of it
/// are unaligned. Additionally at this time the 32-bit encodings chosen here
/// mean that >=4gb text sections are not supported.
pub const ELF_WASMTIME_ADDRMAP: &str = ".wasmtime.addrmap";

impl AddressMapSection {
    /// Pushes a new set of instruction mapping information for a function added
    /// in the exectuable.
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

        self.offsets.reserve(instrs.len());
        self.positions.reserve(instrs.len());
        for map in instrs {
            // Sanity-check to ensure that functions are pushed in-order, otherwise
            // the `offsets` array won't be sorted which is our goal.
            let pos = func_start + map.code_offset;
            assert!(pos >= self.last_offset);
            self.offsets.push(U32Bytes::new(LittleEndian, pos));
            self.positions
                .push(U32Bytes::new(LittleEndian, map.srcloc.0));
            self.last_offset = pos;
        }
        self.last_offset = func_end;
    }

    /// Finishes encoding this section into the `Object` provided.
    pub fn append_to(self, obj: &mut Object) {
        let section = obj.add_section(
            obj.segment_name(StandardSegment::Data).to_vec(),
            ELF_WASMTIME_ADDRMAP.as_bytes().to_vec(),
            SectionKind::ReadOnlyData,
        );

        // NB: this matches the encoding expected by `lookup` below.
        let amt = u32::try_from(self.offsets.len()).unwrap();
        obj.append_section_data(section, &amt.to_le_bytes(), 1);
        obj.append_section_data(section, object::bytes_of_slice(&self.offsets), 1);
        obj.append_section_data(section, object::bytes_of_slice(&self.positions), 1);
    }
}

/// Lookup an `offset` within an encoded address map section, returning the
/// original `FilePos` that corresponds to the offset, if found.
///
/// This function takes a `section` as its first argument which must have been
/// created with `AddressMapSection` above. This is intended to be the raw
/// `ELF_WASMTIME_ADDRMAP` section from the compilation artifact.
///
/// The `offset` provided is a relative offset from the start of the text
/// section of the pc that is being looked up. If `offset` is out of range or
/// doesn't correspond to anything in this file then `None` is returned.
pub fn lookup_file_pos(section: &[u8], offset: usize) -> Option<FilePos> {
    let mut section = Bytes(section);
    // NB: this matches the encoding written by `append_to` above.
    let count = section.read::<U32Bytes<LittleEndian>>().ok()?;
    let count = usize::try_from(count.get(LittleEndian)).ok()?;
    let (offsets, section) =
        object::slice_from_bytes::<U32Bytes<LittleEndian>>(section.0, count).ok()?;
    let (positions, section) =
        object::slice_from_bytes::<U32Bytes<LittleEndian>>(section, count).ok()?;
    debug_assert!(section.is_empty());

    // First perform a binary search on the `offsets` array. This is a sorted
    // array of offsets within the text section, which is conveniently what our
    // `offset` also is. Note that we are somewhat unlikely to find a precise
    // match on the element in the array, so we're largely interested in which
    // "bucket" the `offset` falls into.
    let offset = u32::try_from(offset).ok()?;
    let index = match offsets.binary_search_by_key(&offset, |v| v.get(LittleEndian)) {
        // Exact hit!
        Ok(i) => i,

        // This *would* be at the first slot in the array, so no
        // instructions cover `pc`.
        Err(0) => return None,

        // This would be at the `nth` slot, so we're at the `n-1`th slot.
        Err(n) => n - 1,
    };

    // Using the `index` we found of which bucket `offset` corresponds to we can
    // lookup the actual `FilePos` value in the `positions` array.
    let pos = positions.get(index)?;
    Some(FilePos(pos.get(LittleEndian)))
}
