//! This module contains basic type definitions used by the implementation of
//! the stack switching proposal.

/// FIXME(frank-emrich) Will remove in the final upstreamed version
#[allow(dead_code, reason = "Only accessed in debug builds")]
pub const ENABLE_DEBUG_PRINTING: bool = false;

/// FIXME(frank-emrich) Will remove in the final upstreamed version
#[macro_export]
macro_rules! debug_println {
    ($( $args:expr ),+ ) => {
        #[cfg(debug_assertions)]
        if ENABLE_DEBUG_PRINTING {
            #[cfg(feature = "std")]
            println!($($args),*);
        }
    }
}

/// Runtime configuration options for stack switching that can be set
/// via the command line.
///
/// Part of wasmtime::config::Config type (which is not in scope in this crate).
#[derive(Debug, Clone)]
pub struct StackSwitchingConfig {
    /// The (fixed) size of a continuation stack.
    pub stack_size: usize,
}

impl Default for StackSwitchingConfig {
    fn default() -> Self {
        /// Default size for continuation stacks
        const DEFAULT_FIBER_SIZE: usize = 2097152; // 2MB = 512 pages of 4k

        Self {
            stack_size: DEFAULT_FIBER_SIZE,
        }
    }
}

/// This type is used to save (and subsequently restore) a subset of the data in
/// `VMRuntimeLimits`. See documentation of `VMStackChain` for the exact uses.
#[repr(C)]
#[derive(Debug, Default, Clone)]
pub struct VMStackLimits {
    /// Saved version of `stack_limit` field of `VMRuntimeLimits`
    pub stack_limit: usize,
    /// Saved version of `last_wasm_entry_fp` field of `VMRuntimeLimits`
    pub last_wasm_entry_fp: usize,
}

#[test]
fn check_vm_stack_limits_offsets() {
    use crate::{HostPtr, Module, PtrSize, VMOffsets};
    use core::mem::offset_of;

    let module = Module::new();
    let offsets = VMOffsets::new(HostPtr, &module);
    assert_eq!(
        offset_of!(VMStackLimits, stack_limit),
        usize::from(offsets.ptr.vmstack_limits_stack_limit())
    );
    assert_eq!(
        offset_of!(VMStackLimits, last_wasm_entry_fp),
        usize::from(offsets.ptr.vmstack_limits_last_wasm_entry_fp())
    );
}

/// This type represents "common" information that we need to save both for the
/// initial stack and each continuation.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct VMCommonStackInformation {
    /// Saves subset of `VMRuntimeLimits` for this stack. See documentation of
    /// `VMStackChain` for the exact uses.
    pub limits: VMStackLimits,
    /// For the initial stack, this field must only have one of the following values:
    /// - Running
    /// - Parent
    pub state: VMStackState,

    /// Only in use when state is `Parent`. Otherwise, the list must be empty.
    ///
    /// Represents the handlers that this stack installed when resume-ing a
    /// continuation.
    ///
    /// Note that for any resume instruction, we can re-order the handler
    /// clauses without changing behavior such that all the suspend handlers
    /// come first, followed by all the switch handler (while maintaining the
    /// original ordering within the two groups).
    /// Thus, we assume that the given resume instruction has the following
    /// shape:
    ///
    /// (resume $ct
    ///   (on $tag_0 $block_0) ... (on $tag_{n-1} $block_{n-1})
    ///   (on $tag_n switch) ... (on $tag_m switch)
    /// )
    ///
    /// On resume, the handler list is then filled with m + 1 (i.e., one per
    /// handler clause) entries such that the i-th entry, using 0-based
    /// indexing, is the identifier of $tag_i (represented as *mut
    /// VMTagDefinition).
    /// Further, `first_switch_handler_index` (see below) is set to n (i.e., the
    /// 0-based index of the first switch handler).
    ///
    /// Note that the actual data buffer (i.e., the one `handler.data` points
    /// to) is always allocated on the stack that this `CommonStackInformation`
    /// struct describes.
    pub handlers: VMHandlerList,

    /// Only used when state is `Parent`. See documentation of `handlers` above.
    pub first_switch_handler_index: u32,
}

impl VMCommonStackInformation {
    /// Default value with state set to `Running`
    pub fn running_default() -> Self {
        Self {
            limits: VMStackLimits::default(),
            state: VMStackState::Running,
            handlers: VMHandlerList::empty(),
            first_switch_handler_index: 0,
        }
    }
}

#[test]
fn check_vm_common_stack_information_offsets() {
    use crate::{HostPtr, Module, PtrSize, VMOffsets};
    use core::mem::offset_of;
    use std::mem::size_of;

    let module = Module::new();
    let offsets = VMOffsets::new(HostPtr, &module);
    assert_eq!(
        size_of::<VMCommonStackInformation>(),
        usize::from(offsets.ptr.size_of_vmcommon_stack_information())
    );
    assert_eq!(
        offset_of!(VMCommonStackInformation, limits),
        usize::from(offsets.ptr.vmcommon_stack_information_limits())
    );
    assert_eq!(
        offset_of!(VMCommonStackInformation, state),
        usize::from(offsets.ptr.vmcommon_stack_information_state())
    );
    assert_eq!(
        offset_of!(VMCommonStackInformation, handlers),
        usize::from(offsets.ptr.vmcommon_stack_information_handlers())
    );
    assert_eq!(
        offset_of!(VMCommonStackInformation, first_switch_handler_index),
        usize::from(
            offsets
                .ptr
                .vmcommon_stack_information_first_switch_handler_index()
        )
    );
}

impl VMStackLimits {
    /// Default value, but uses the given value for `stack_limit`.
    pub fn with_stack_limit(stack_limit: usize) -> Self {
        Self {
            stack_limit,
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
/// Reference to a stack-allocated buffer ("array"), storing data of some type
/// `T`.
pub struct VMArray<T> {
    /// Number of currently occupied slots.
    pub length: u32,
    /// Number of slots in the data buffer. Note that this is *not* the size of
    /// the buffer in bytes!
    pub capacity: u32,
    /// The actual data buffer
    pub data: *mut T,
}

impl<T> VMArray<T> {
    /// Creates empty `Array`
    pub fn empty() -> Self {
        Self {
            length: 0,
            capacity: 0,
            data: core::ptr::null_mut(),
        }
    }

    /// Makes `Array` empty.
    pub fn clear(&mut self) {
        *self = Self::empty();
    }
}

#[test]
fn check_vm_array_offsets() {
    use crate::{HostPtr, Module, PtrSize, VMOffsets};
    use core::mem::offset_of;
    use std::mem::size_of;

    // Note that the type parameter has no influence on the size and offsets.

    let module = Module::new();
    let offsets = VMOffsets::new(HostPtr, &module);
    assert_eq!(
        size_of::<VMArray<()>>(),
        usize::from(offsets.ptr.size_of_vmarray())
    );
    assert_eq!(
        offset_of!(VMArray<()>, length),
        usize::from(offsets.ptr.vmarray_length())
    );
    assert_eq!(
        offset_of!(VMArray<()>, capacity),
        usize::from(offsets.ptr.vmarray_capacity())
    );
    assert_eq!(
        offset_of!(VMArray<()>, data),
        usize::from(offsets.ptr.vmarray_data())
    );
}

/// Type used for passing payloads to and from continuations. The actual type
/// argument should be wasmtime::runtime::vm::vmcontext::ValRaw, but we don't
/// have access to that here.
pub type VMPayloads = VMArray<u128>;

/// Type for a list of handlers, represented by the handled tag. Thus, the
/// stored data is actually `*mut VMTagDefinition`, but we don't havr access to
/// that here.
pub type VMHandlerList = VMArray<*mut u8>;

/// Discriminant of variant `Absent` in
/// `wasmtime_runtime::continuation::VMStackChain`.
pub const STACK_CHAIN_ABSENT_DISCRIMINANT: usize = 0;
/// Discriminant of variant `InitialStack` in
/// `wasmtime_runtime::continuation::VMStackChain`.
pub const STACK_CHAIN_INITIAL_STACK_DISCRIMINANT: usize = 1;
/// Discriminant of variant `Continiation` in
/// `wasmtime_runtime::continuation::VMStackChain`.
pub const STACK_CHAIN_CONTINUATION_DISCRIMINANT: usize = 2;

/// Encodes the life cycle of a `VMContRef`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(i32)]
pub enum VMStackState {
    /// The `VMContRef` has been created, but neither `resume` or `switch` has ever been
    /// called on it. During this stage, we may add arguments using `cont.bind`.
    Fresh,
    /// The continuation is running, meaning that it is the one currently
    /// executing code.
    Running,
    /// The continuation is suspended because it executed a resume instruction
    /// that has not finished yet. In other words, it became the parent of
    /// another continuation (which may itself be `Running`, a `Parent`, or
    /// `Suspended`).
    Parent,
    /// The continuation was suspended by a `suspend` or `switch` instruction.
    Suspended,
    /// The function originally passed to `cont.new` has returned normally.
    /// Note that there is no guarantee that a VMContRef will ever
    /// reach this status, as it may stay suspended until being dropped.
    Returned,
}

impl VMStackState {
    /// Returns i32 discriminant of this variant.
    pub fn discriminant(&self) -> i32 {
        // This is well-defined for an enum with repr(i32).
        unsafe { *(self as *const Self as *const i32) }
    }
}

impl From<VMStackState> for i32 {
    fn from(st: VMStackState) -> Self {
        st.discriminant()
    }
}

/// Discriminant of variant `Return` in
/// `ControlEffect`.
pub const CONTROL_EFFECT_RETURN_DISCRIMINANT: u32 = 0;
/// Discriminant of variant `Resume` in
/// `ControlEffect`.
pub const CONTROL_EFFECT_RESUME_DISCRIMINANT: u32 = 1;
/// Discriminant of variant `Suspend` in
/// `ControlEffect`.
pub const CONTROL_EFFECT_SUSPEND_DISCRIMINANT: u32 = 2;
/// Discriminant of variant `Switch` in
/// `ControlEffect`.
pub const CONTROL_EFFECT_SWITCH_DISCRIMINANT: u32 = 3;

/// Universal control effect. This structure encodes return signal, resume
/// signal, suspension signal, and the handler to suspend to in a single variant
/// type. This instance is used at runtime. There is a codegen counterpart in
/// `cranelift/src/stack-switching/control_effect.rs`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ControlEffect {
    /// Used to signal that a continuation has returned and control switches
    /// back to the parent.
    Return = CONTROL_EFFECT_RETURN_DISCRIMINANT,
    /// Used to signal to a continuation that it is being resumed.
    Resume = CONTROL_EFFECT_RESUME_DISCRIMINANT,
    /// Used to signal that a continuation has invoked a `suspend` instruction.
    Suspend {
        /// The index of the handler to be used in the parent continuation to
        /// switch back to.
        handler_index: u32,
    } = CONTROL_EFFECT_SUSPEND_DISCRIMINANT,
    /// Used to signal that a continuation has invoked a `suspend` instruction.
    Switch = CONTROL_EFFECT_SWITCH_DISCRIMINANT,
}
