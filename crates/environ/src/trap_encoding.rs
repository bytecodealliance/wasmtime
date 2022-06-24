use object::write::{Object, StandardSegment};
use object::{Bytes, LittleEndian, SectionKind, U32Bytes};
use std::convert::TryFrom;
use std::ops::Range;

/// A helper structure to build the custom-encoded section of a wasmtime
/// compilation image which encodes trap information.
///
/// This structure is incrementally fed the results of compiling individual
/// functions and handles all the encoding internally, allowing usage of
/// `lookup_trap_code` below with the resulting section.
#[derive(Default)]
pub struct TrapEncodingBuilder {
    offsets: Vec<U32Bytes<LittleEndian>>,
    traps: Vec<u8>,
    last_offset: u32,
}

/// A custom binary-encoded section of wasmtime compilation artifacts which
/// encodes the ability to map an offset in the text section to the trap code
/// that it corresponds to.
///
/// This section is used at runtime to determine what flavor fo trap happened to
/// ensure that embedders and debuggers know the reason for the wasm trap. The
/// encoding of this section is custom to Wasmtime and managed with helpers in
/// the `object` crate:
///
/// * First the section has a 32-bit little endian integer indicating how many
///   trap entries are in the section.
/// * Next is an array, of the same length as read before, of 32-bit
///   little-endian integers. These integers are offsets into the text section
///   of the compilation image.
/// * Finally is the same count number of bytes. Each of these bytes corresponds
///   to a trap code.
///
/// This section is decoded by `lookup_trap_code` below which will read the
/// section count, slice some bytes to get the various arrays, and then perform
/// a binary search on the offsets array to find the an index corresponding to
/// the pc being looked up. If found the same index in the trap array (the array
/// of bytes) is the trap code for that offset.
///
/// Note that at this time this section has an alignment of 1. Additionally due
/// to the 32-bit encodings for offsets this doesn't support images >=4gb.
pub const ELF_WASMTIME_TRAPS: &str = ".wasmtime.traps";

/// Information about trap.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TrapInformation {
    /// The offset of the trapping instruction in native code.
    ///
    /// This is relative to the beginning of the function.
    pub code_offset: u32,

    /// Code of the trap.
    pub trap_code: TrapCode,
}

/// A trap code describing the reason for a trap.
///
/// All trap instructions have an explicit trap code.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum TrapCode {
    /// The current stack space was exhausted.
    StackOverflow,

    /// A `heap_addr` instruction detected an out-of-bounds error.
    ///
    /// Note that not all out-of-bounds heap accesses are reported this way;
    /// some are detected by a segmentation fault on the heap unmapped or
    /// offset-guard pages.
    HeapOutOfBounds,

    /// A wasm atomic operation was presented with a not-naturally-aligned linear-memory address.
    HeapMisaligned,

    /// A `table_addr` instruction detected an out-of-bounds error.
    TableOutOfBounds,

    /// Indirect call to a null table entry.
    IndirectCallToNull,

    /// Signature mismatch on indirect call.
    BadSignature,

    /// An integer arithmetic operation caused an overflow.
    IntegerOverflow,

    /// An integer division by zero.
    IntegerDivisionByZero,

    /// Failed float-to-int conversion.
    BadConversionToInteger,

    /// Code that was supposed to have been unreachable was reached.
    UnreachableCodeReached,

    /// Execution has potentially run too long and may be interrupted.
    /// This trap is resumable.
    Interrupt,

    /// Used for the component model when functions are lifted/lowered in a way
    /// that generates a function that always traps.
    AlwaysTrapAdapter,
    // if adding a variant here be sure to update the `check!` macro below
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
        // the `offsets` array won't be sorted which is our goal.
        assert!(func_start >= self.last_offset);

        self.offsets.reserve(traps.len());
        self.traps.reserve(traps.len());
        for info in traps {
            let pos = func_start + info.code_offset;
            assert!(pos >= self.last_offset);
            self.offsets.push(U32Bytes::new(LittleEndian, pos));
            self.traps.push(info.trap_code as u8);
            self.last_offset = pos;
        }

        self.last_offset = func_end;
    }

    /// Encodes this section into the object provided.
    pub fn append_to(self, obj: &mut Object) {
        let section = obj.add_section(
            obj.segment_name(StandardSegment::Data).to_vec(),
            ELF_WASMTIME_TRAPS.as_bytes().to_vec(),
            SectionKind::ReadOnlyData,
        );

        // NB: this matches the encoding expected by `lookup` below.
        let amt = u32::try_from(self.traps.len()).unwrap();
        obj.append_section_data(section, &amt.to_le_bytes(), 1);
        obj.append_section_data(section, object::bytes_of_slice(&self.offsets), 1);
        obj.append_section_data(section, &self.traps, 1);
    }
}

/// Decodes the provided trap information section and attempts to find the trap
/// code corresponding to the `offset` specified.
///
/// The `section` provided is expected to have been built by
/// `TrapEncodingBuilder` above. Additionally the `offset` should be a relative
/// offset within the text section of the compilation image.
pub fn lookup_trap_code(section: &[u8], offset: usize) -> Option<TrapCode> {
    let mut section = Bytes(section);
    // NB: this matches the encoding written by `append_to` above.
    let count = section.read::<U32Bytes<LittleEndian>>().ok()?;
    let count = usize::try_from(count.get(LittleEndian)).ok()?;
    let (offsets, traps) =
        object::slice_from_bytes::<U32Bytes<LittleEndian>>(section.0, count).ok()?;
    debug_assert_eq!(traps.len(), count);

    // The `offsets` table is sorted in the trap section so perform a binary
    // search of the contents of this section to find whether `offset` is an
    // entry in the section. Note that this is a precise search because trap pcs
    // should always be precise as well as our metadata about them, which means
    // we expect an exact match to correspond to a trap opcode.
    //
    // Once an index is found within the `offsets` array then that same index is
    // used to lookup from the `traps` list of bytes to get the trap code byte
    // corresponding to this offset.
    let offset = u32::try_from(offset).ok()?;
    let index = offsets
        .binary_search_by_key(&offset, |val| val.get(LittleEndian))
        .ok()?;
    debug_assert!(index < traps.len());
    let trap = *traps.get(index)?;

    // FIXME: this could use some sort of derive-like thing to avoid having to
    // deduplicate the names here.
    //
    // This simply converts from the `trap`, a `u8`, to the `TrapCode` enum.
    macro_rules! check {
        ($($name:ident)*) => ($(if trap == TrapCode::$name as u8 {
            return Some(TrapCode::$name);
        })*);
    }

    check! {
        StackOverflow
        HeapOutOfBounds
        HeapMisaligned
        TableOutOfBounds
        IndirectCallToNull
        BadSignature
        IntegerOverflow
        IntegerDivisionByZero
        BadConversionToInteger
        UnreachableCodeReached
        Interrupt
        AlwaysTrapAdapter
    }

    if cfg!(debug_assertions) {
        panic!("missing mapping for {}", trap);
    } else {
        None
    }
}
