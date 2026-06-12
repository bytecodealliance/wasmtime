//! Data structures to provide transformation of the source

use crate::bytes::{read_sleb, read_uleb};
use core::fmt;
use object::{Bytes, LittleEndian, U32};
use serde_derive::{Deserialize, Serialize};

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

    /// Get the null file position.
    pub fn none() -> FilePos {
        FilePos(u32::MAX)
    }

    /// Is this the null file position?
    #[inline]
    pub fn is_none(&self) -> bool {
        *self == FilePos::none()
    }

    /// Returns the offset that this offset was created with.
    ///
    /// Note that positions created with `FilePos::none` and the `Default`
    /// implementation will return `None` here, whereas positions created with
    /// `FilePos::new` will return `Some`.
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
        FilePos::none()
    }
}

/// A Wasm bytecode offset relative to the start of a component (or
/// top-level module) binary.
///
/// When compiling a component, the Wasm parser returns source
/// positions relative to the entire component binary. This type
/// captures that convention. Use
/// [`ComponentPC::to_module_pc`] to convert to a
/// [`ModulePC`] given the byte offset of the module within the
/// component.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentPC(u32);

impl ComponentPC {
    /// Create a new component-relative PC from a raw offset.
    pub fn new(offset: u32) -> Self {
        Self(offset)
    }

    /// Get the raw u32 offset.
    pub fn raw(self) -> u32 {
        self.0
    }

    /// Convert to a module-relative PC by subtracting the byte offset
    /// of the module within the component binary.
    pub fn to_module_pc(self, wasm_module_offset: u64) -> ModulePC {
        let offset = u32::try_from(wasm_module_offset).unwrap();
        ModulePC(self.0 - offset)
    }
}

impl fmt::Debug for ComponentPC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ComponentPC({:#x})", self.0)
    }
}

impl fmt::Display for ComponentPC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

/// A Wasm bytecode offset relative to the start of a core Wasm
/// module binary.
///
/// In the guest-debug system, PCs are always module-relative because
/// the debugger presents a core-Wasm view of the world where
/// components are deconstructed into individual core Wasm modules.
///
/// For standalone (non-component) modules, `ModulePC` and
/// [`ComponentPC`] values are numerically identical.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModulePC(u32);

impl ModulePC {
    /// Create a new module-relative PC from a raw offset.
    pub fn new(offset: u32) -> Self {
        Self(offset)
    }

    /// Get the raw u32 offset.
    pub fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Debug for ModulePC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ModulePC({:#x})", self.0)
    }
}

impl fmt::Display for ModulePC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

/// Number of address-mapping entries packed into one block of the address map
/// section.
///
/// See `AddressMapSection` in `crate::compile` for the full section format.
/// Chosen as a balance between the fixed-width index overhead per block (8
/// bytes, amortized across entries) and the amount of linear decoding required
/// to look up a single pc within a block.
pub(crate) const ADDRMAP_BLOCK_SIZE: usize = 128;

/// A parsed view of the address map section.
///
/// The fields here correspond to the pieces of the section layout described
/// on `AddressMapSection` in `crate::compile`.
#[derive(Clone, Copy)]
struct AddressMap<'a> {
    /// Total number of address-mapping entries in this section.
    entries: usize,
    /// One `(first_offset, block_pos)` pair per block.
    block_index: &'a [[U32<LittleEndian>; 2]],
    /// Variable-length block bodies, indexed by `block_pos` in the
    /// `block_index` table above.
    block_bodies: &'a [u8],
}

impl<'a> AddressMap<'a> {
    /// Returns an iterator of `(text_offset, FilePos)` for all entries in
    /// `block`, or `None` if the section is malformed.
    fn block_entries(&self, block_index: usize) -> Option<BlockEntries<'a>> {
        let [first_offset, block_pos] = self.block_index.get(block_index)?;
        let first_offset = first_offset.get(LittleEndian);
        let block_pos = block_pos.get(LittleEndian);
        let block = self.block_bodies.get(usize::try_from(block_pos).ok()?..)?;
        let remaining = core::cmp::min(
            ADDRMAP_BLOCK_SIZE,
            self.entries.checked_sub(block_index * ADDRMAP_BLOCK_SIZE)?,
        );
        Some(BlockEntries {
            block,
            prev_offset: first_offset,
            prev_pos: None,
            remaining,
        })
    }
}

/// Iterator over the entries of a single block, decoding the delta-and-flag
/// varints described in the "block body" portion of the section format on
/// `AddressMapSection` in `crate::compile`.
struct BlockEntries<'a> {
    block: &'a [u8],
    prev_offset: u32,
    prev_pos: Option<u32>,
    remaining: usize,
}

impl Iterator for BlockEntries<'_> {
    type Item = (u32, FilePos);

    fn next(&mut self) -> Option<(u32, FilePos)> {
        self.remaining = self.remaining.checked_sub(1)?;
        let token = read_uleb(&mut self.block)?;
        let delta = u32::try_from(token >> 1).ok()?;
        let cur_offset = self.prev_offset.checked_add(delta)?;
        self.prev_offset = cur_offset;
        if token & 1 != 0 {
            return Some((cur_offset, FilePos::none()));
        }
        let pos = match self.prev_pos {
            // The first non-none position of a block is encoded absolutely...
            None => u32::try_from(read_uleb(&mut self.block)?).ok()?,
            // ... and subsequent positions are sleb deltas from the previous
            // non-none position.
            Some(prev) => {
                let delta = read_sleb(&mut self.block)?;
                prev.checked_add_signed(i32::try_from(delta).ok()?)?
            }
        };
        self.prev_pos = Some(pos);
        Some((cur_offset, FilePos(pos)))
    }
}

/// Parse an `ELF_WASMTIME_ADDRMAP` section into its header, block index, and
/// block bodies.
fn parse(section: &[u8]) -> Option<AddressMap<'_>> {
    let mut section = Bytes(section);
    // NB: this matches the encoding written by `AddressMapSection` in the
    // `compile::address_map` module.
    let entries = section.read::<U32<LittleEndian>>().ok()?;
    let entries = usize::try_from(entries.get(LittleEndian)).ok()?;
    let num_blocks = section.read::<U32<LittleEndian>>().ok()?;
    let num_blocks = usize::try_from(num_blocks.get(LittleEndian)).ok()?;
    let (block_index, block_bodies) =
        object::slice_from_bytes::<[U32<LittleEndian>; 2]>(section.0, num_blocks).ok()?;
    Some(AddressMap {
        entries,
        block_index,
        block_bodies,
    })
}

/// Lookup an `offset` within an encoded address map section, returning the
/// original `FilePos` that corresponds to the offset, if found.
///
/// This function takes a `section` as its first argument which must have been
/// created with `AddressMapSection` in `crate::compile`, whose documentation
/// describes the format decoded here. This is intended to be the raw
/// `ELF_WASMTIME_ADDRMAP` section from the compilation artifact.
///
/// The `offset` provided is a relative offset from the start of the text
/// section of the pc that is being looked up. If `offset` is out of range or
/// doesn't correspond to anything in this file then `None` is returned.
pub fn lookup_file_pos(section: &[u8], offset: usize) -> Option<FilePos> {
    let section = parse(section)?;
    let offset = u32::try_from(offset).ok()?;

    // Find the last block whose first pc is `<= offset`. Note that, unlike the
    // trap section, this is a bucket-style search: each entry covers addresses
    // from its own `text_offset` until the next entry's, so `offset` need not
    // match an entry exactly. The covering entry is wholly contained in this
    // block since the next block only takes over at its own `first_offset`.
    let block = section
        .block_index
        .partition_point(|[first_offset, _]| first_offset.get(LittleEndian) <= offset)
        .checked_sub(1)?;

    // Find the last entry within this block whose offset is `<= offset`; that
    // entry's bucket covers `offset`. At least the block's first entry always
    // qualifies due to the index search above.
    let mut pos = None;
    for (entry_offset, entry_pos) in section.block_entries(block)? {
        if entry_offset > offset {
            break;
        }
        pos = Some(entry_pos);
    }
    pos
}

/// Iterate over the address map contained in the given address map section.
///
/// This function takes a `section` as its first argument which must have been
/// created with `AddressMapSection` in `crate::compile`. This is intended to
/// be the raw `ELF_WASMTIME_ADDRMAP` section from the compilation artifact.
///
/// The yielded offsets are relative to the start of the text section for this
/// map's code object.
pub fn iterate_address_map<'a>(
    section: &'a [u8],
) -> Option<impl Iterator<Item = (u32, FilePos)> + 'a> {
    let section = parse(section)?;

    Some(
        (0..section.block_index.len())
            .flat_map(move |block| section.block_entries(block).into_iter().flatten()),
    )
}
