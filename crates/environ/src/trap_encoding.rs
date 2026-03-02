use core::fmt;
use object::{Bytes, LittleEndian, U32Bytes};

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

macro_rules! generate_trap_type {
    (pub enum Trap {
        $(
            $(#[$doc:meta])*
            $name:ident = $msg:tt,
        )*
    }) => {
        #[non_exhaustive]
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
        #[expect(missing_docs, reason = "self-describing variants")]
        pub enum Trap {
            $(
                $(#[$doc])*
                $name,
            )*
        }

        impl Trap {
            /// Converts a byte back into a `Trap` if its in-bounds
            pub fn from_u8(byte: u8) -> Option<Trap> {
                $(
                    if byte == Trap::$name as u8 {
                        return Some(Trap::$name);
                    }
                )*
                None
            }
        }

        impl fmt::Display for Trap {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let desc = match self {
                    $(Self::$name => $msg,)*
                };
                write!(f, "wasm trap: {desc}")
            }
        }
    }
}

// The code can be accessed from the c-api, where the possible values are
// translated into enum values defined there:
//
// *  the const assertions in c-api/src/trap.rs, and
// * `wasmtime_trap_code_enum` in c-api/include/wasmtime/trap.h.
//
// These need to be kept in sync.
generate_trap_type! {
    pub enum Trap {
        /// The current stack space was exhausted.
        StackOverflow = "call stack exhausted",

        /// An out-of-bounds memory access.
        MemoryOutOfBounds = "out of bounds memory access",

        /// A wasm atomic operation was presented with a not-naturally-aligned linear-memory address.
        HeapMisaligned = "unaligned atomic",

        /// An out-of-bounds access to a table.
        TableOutOfBounds = "undefined element: out of bounds table access",

        /// Indirect call to a null table entry.
        IndirectCallToNull = "uninitialized element",

        /// Signature mismatch on indirect call.
        BadSignature = "indirect call type mismatch",

        /// An integer arithmetic operation caused an overflow.
        IntegerOverflow = "integer overflow",

        /// An integer division by zero.
        IntegerDivisionByZero = "integer divide by zero",

        /// Failed float-to-int conversion.
        BadConversionToInteger = "invalid conversion to integer",

        /// Code that was supposed to have been unreachable was reached.
        UnreachableCodeReached = "wasm `unreachable` instruction executed",

        /// Execution has potentially run too long and may be interrupted.
        Interrupt = "interrupt",

        /// When wasm code is configured to consume fuel and it runs out of fuel
        /// then this trap will be raised.
        OutOfFuel = "all fuel consumed by WebAssembly",

        /// Used to indicate that a trap was raised by atomic wait operations on non shared memory.
        AtomicWaitNonSharedMemory = "atomic wait on non-shared memory",

        /// Call to a null reference.
        NullReference = "null reference",

        /// Attempt to access beyond the bounds of an array.
        ArrayOutOfBounds = "out of bounds array access",

        /// Attempted an allocation that was too large to succeed.
        AllocationTooLarge = "allocation size too large",

        /// Attempted to cast a reference to a type that it is not an instance of.
        CastFailure = "cast failure",

        /// When the `component-model` feature is enabled this trap represents a
        /// scenario where one component tried to call another component but it
        /// would have violated the reentrance rules of the component model,
        /// triggering a trap instead.
        CannotEnterComponent = "cannot enter component instance",

        /// Async-lifted export failed to produce a result by calling `task.return`
        /// before returning `STATUS_DONE` and/or after all host tasks completed.
        NoAsyncResult = "async-lifted export failed to produce a result",

        /// We are suspending to a tag for which there is no active handler.
        UnhandledTag = "unhandled tag",

        /// Attempt to resume a continuation twice.
        ContinuationAlreadyConsumed = "continuation already consumed",

        /// A Pulley opcode was executed at runtime when the opcode was disabled at
        /// compile time.
        DisabledOpcode = "pulley opcode disabled at compile time was executed",

        /// Async event loop deadlocked; i.e. it cannot make further progress given
        /// that all host tasks have completed and any/all host-owned stream/future
        /// handles have been dropped.
        AsyncDeadlock = "deadlock detected: event loop cannot make further progress",

        /// When the `component-model` feature is enabled this trap represents a
        /// scenario where a component instance tried to call an import or intrinsic
        /// when it wasn't allowed to, e.g. from a post-return function.
        CannotLeaveComponent = "cannot leave component instance",

        /// A synchronous task attempted to make a potentially blocking call prior
        /// to returning.
        CannotBlockSyncTask = "cannot block a synchronous task before returning",

        /// A component tried to lift a `char` with an invalid bit pattern.
        InvalidChar = "invalid `char` bit pattern",

        /// Debug assertion generated for a fused adapter regarding the expected
        /// completion of a string encoding operation.
        DebugAssertStringEncodingFinished = "should have finished string encoding",

        /// Debug assertion generated for a fused adapter regarding a string
        /// encoding operation.
        DebugAssertEqualCodeUnits = "code units should be equal",

        /// Debug assertion generated for a fused adapter regarding the alignment of
        /// a pointer.
        DebugAssertPointerAligned = "pointer should be aligned",

        /// Debug assertion generated for a fused adapter regarding the upper bits
        /// of a 64-bit value.
        DebugAssertUpperBitsUnset = "upper bits should be unset",

        /// A component tried to lift or lower a string past the end of its memory.
        StringOutOfBounds = "string content out-of-bounds",

        /// A component tried to lift or lower a list past the end of its memory.
        ListOutOfBounds = "list content out-of-bounds",

        /// A component used an invalid discriminant when lowering a variant value.
        InvalidDiscriminant = "invalid variant discriminant",

        /// A component passed an unaligned pointer when lifting or lowering a
        /// value.
        UnalignedPointer = "unaligned pointer",

        /// `task.cancel` was called by a task which has not been cancelled.
        TaskCancelNotCancelled = "`task.cancel` called by task which has not been cancelled",

        /// `task.return` or `task.cancel` was called more than once for the
        /// current task.
        TaskCancelOrReturnTwice = "`task.return` or `task.cancel` called more than once for current task",

        /// `subtask.cancel` was called after terminal status was already
        /// delivered.
        SubtaskCancelAfterTerminal = "`subtask.cancel` called after terminal status delivered",

        /// Invalid `task.return` signature and/or options for the current task.
        TaskReturnInvalid = "invalid `task.return` signature and/or options for current task",

        /// Cannot drop waitable set with waiters in it.
        WaitableSetDropHasWaiters = "cannot drop waitable set with waiters",

        /// Cannot drop a subtask which has not yet resolved.
        SubtaskDropNotResolved = "cannot drop a subtask which has not yet resolved",

        /// Start function does not match the expected type.
        ThreadNewIndirectInvalidType = "start function does not match expected type (currently only `(i32) -> ()` is supported)",

        /// The start function index points to an uninitialized function.
        ThreadNewIndirectUninitialized = "the start function index points to an uninitialized function",

        /// Backpressure-related intrinsics overflowed the built-in 16-bit
        /// counter.
        BackpressureOverflow = "backpressure counter overflow",

        /// Invalid code returned from `callback` of `async`-lifted function.
        UnsupportedCallbackCode = "unsupported callback code",

        /// Cannot resume a thread which is not suspended.
        CannotResumeThread = "cannot resume thread which is not suspended",

        // if adding a variant here be sure to update `trap.rs` and `trap.h` as
        // mentioned above
    }
}

impl core::error::Error for Trap {}

/// Decodes the provided trap information section and attempts to find the trap
/// code corresponding to the `offset` specified.
///
/// The `section` provided is expected to have been built by
/// `TrapEncodingBuilder` above. Additionally the `offset` should be a relative
/// offset within the text section of the compilation image.
pub fn lookup_trap_code(section: &[u8], offset: usize) -> Option<Trap> {
    let (offsets, traps) = parse(section)?;

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
    let byte = *traps.get(index)?;

    let trap = Trap::from_u8(byte);
    debug_assert!(trap.is_some(), "missing mapping for {byte}");
    trap
}

fn parse(section: &[u8]) -> Option<(&[U32Bytes<LittleEndian>], &[u8])> {
    let mut section = Bytes(section);
    // NB: this matches the encoding written by `append_to` above.
    let count = section.read::<U32Bytes<LittleEndian>>().ok()?;
    let count = usize::try_from(count.get(LittleEndian)).ok()?;
    let (offsets, traps) =
        object::slice_from_bytes::<U32Bytes<LittleEndian>>(section.0, count).ok()?;
    debug_assert_eq!(traps.len(), count);
    Some((offsets, traps))
}

/// Returns an iterator over all of the traps encoded in `section`, which should
/// have been produced by `TrapEncodingBuilder`.
pub fn iterate_traps(section: &[u8]) -> Option<impl Iterator<Item = (u32, Trap)> + '_> {
    let (offsets, traps) = parse(section)?;
    Some(
        offsets
            .iter()
            .zip(traps)
            .map(|(offset, trap)| (offset.get(LittleEndian), Trap::from_u8(*trap).unwrap())),
    )
}
