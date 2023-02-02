use crate::obj::ELF_WASMTIME_TRAPS;
use object::write::{Object, StandardSegment};
use object::{Bytes, LittleEndian, SectionKind, U32Bytes};
use std::convert::TryFrom;
use std::fmt;
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

/// Information about trap.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TrapInformation {
    /// The offset of the trapping instruction in native code.
    ///
    /// This is relative to the beginning of the function.
    pub code_offset: u32,

    /// Code of the trap.
    pub trap_code: Trap,
}

// The code can be accessed from the c-api, where the possible values are
// translated into enum values defined there:
//
// * `wasm_trap_code` in c-api/src/trap.rs, and
// * `wasmtime_trap_code_enum` in c-api/include/wasmtime/trap.h.
//
// These need to be kept in sync.
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[allow(missing_docs)]
pub enum Trap {
    /// The current stack space was exhausted.
    StackOverflow,

    /// An out-of-bounds memory access.
    MemoryOutOfBounds,

    /// A wasm atomic operation was presented with a not-naturally-aligned linear-memory address.
    HeapMisaligned,

    /// An out-of-bounds access to a table.
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
    Interrupt,

    /// When the `component-model` feature is enabled this trap represents a
    /// function that was `canon lift`'d, then `canon lower`'d, then called.
    /// This combination of creation of a function in the component model
    /// generates a function that always traps and, when called, produces this
    /// flavor of trap.
    AlwaysTrapAdapter,

    /// When wasm code is configured to consume fuel and it runs out of fuel
    /// then this trap will be raised.
    OutOfFuel,

    /// Used to indicate that a trap was raised by atomic wait operations on non shared memory.
    AtomicWaitNonSharedMemory,
    // if adding a variant here be sure to update the `check!` macro below
}

impl fmt::Display for Trap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Trap::*;

        let desc = match self {
            StackOverflow => "call stack exhausted",
            MemoryOutOfBounds => "out of bounds memory access",
            HeapMisaligned => "unaligned atomic",
            TableOutOfBounds => "undefined element: out of bounds table access",
            IndirectCallToNull => "uninitialized element",
            BadSignature => "indirect call type mismatch",
            IntegerOverflow => "integer overflow",
            IntegerDivisionByZero => "integer divide by zero",
            BadConversionToInteger => "invalid conversion to integer",
            UnreachableCodeReached => "wasm `unreachable` instruction executed",
            Interrupt => "interrupt",
            AlwaysTrapAdapter => "degenerate component adapter called",
            OutOfFuel => "all fuel consumed by WebAssembly",
            AtomicWaitNonSharedMemory => "atomic wait on non-shared memory",
        };
        write!(f, "wasm trap: {desc}")
    }
}

impl std::error::Error for Trap {}

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
pub fn lookup_trap_code(section: &[u8], offset: usize) -> Option<Trap> {
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
    // This simply converts from the `trap`, a `u8`, to the `Trap` enum.
    macro_rules! check {
        ($($name:ident)*) => ($(if trap == Trap::$name as u8 {
            return Some(Trap::$name);
        })*);
    }

    check! {
        StackOverflow
        MemoryOutOfBounds
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
        OutOfFuel
        AtomicWaitNonSharedMemory
    }

    if cfg!(debug_assertions) {
        panic!("missing mapping for {}", trap);
    } else {
        None
    }
}
