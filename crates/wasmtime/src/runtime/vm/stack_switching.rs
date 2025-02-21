//! This module contains the runtime components of the implementation of the
//! stack switching proposal.

use core::{cell::UnsafeCell, marker::PhantomPinned, ptr::NonNull};

use stack::ContinuationStack;
use wasmtime_environ::stack_switching::{Array, HandlerList, Payloads, StackLimits, State};
#[allow(unused)]
use wasmtime_environ::{
    debug_println,
    stack_switching::{CommonStackInformation, ENABLE_DEBUG_PRINTING},
};

use crate::vm::{Instance, TrapReason, VMFuncRef, VMStore};
use crate::ValRaw;

pub mod stack;

/// A continuation object is a handle to a continuation reference
/// (i.e. an actual stack). A continuation object only be consumed
/// once. The linearity is checked dynamically in the generated code
/// by comparing the revision witness embedded in the pointer to the
/// actual revision counter on the continuation reference.
///
/// In the optimized implementation, the continuation logically
/// represented by a VMContObj not only encompasses the pointed-to
/// VMContRef, but also all of its parents:
///
/// ```text
///
///                     +----------------+
///                 +-->|   VMContRef    |
///                 |   +----------------+
///                 |            ^
///                 |            | parent
///                 |            |
///                 |   +----------------+
///                 |   |   VMContRef    |
///                 |   +----------------+
///                 |            ^
///                 |            | parent
///  last ancestor  |            |
///                 |   +----------------+
///                 +---|   VMContRef    |    <--  VMContObj
///                     +----------------+
/// ```
///
/// For performance reasons, the VMContRef at the bottom of this chain
/// (i.e., the one pointed to by the VMContObj) has a pointer to the
/// other end of the chain (i.e., its last ancestor).
// FIXME(frank-emrich) Does this actually need to be 16-byte aligned any
// more? Now that we use I128 on the Cranelift side (see
// [wasmtime_cranelift::stack_switching::fatpointer::pointer_type]), it
// should be fine to use the natural alignment of the type.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct VMContObj {
    pub revision: u64,
    pub contref: NonNull<VMContRef>,
}

impl VMContObj {
    pub fn new(contref: NonNull<VMContRef>, revision: u64) -> Self {
        Self { contref, revision }
    }
}

unsafe impl Send for VMContObj {}
unsafe impl Sync for VMContObj {}

#[test]
fn null_pointer_optimization() {
    // The Rust spec does not technically guarantee that the null pointer
    // optimization applies to a struct containing a `NonNull`.
    assert_eq!(
        core::mem::size_of::<Option<VMContObj>>(),
        core::mem::size_of::<VMContObj>()
    );
}

/// The main type representing a continuation.
#[repr(C)]
pub struct VMContRef {
    /// The `CommonStackInformation` of this continuation's stack.
    pub common_stack_information: CommonStackInformation,

    /// The parent of this continuation, which may be another continuation, the
    /// initial stack, or absent (in case of a suspended continuation).
    pub parent_chain: StackChain,

    /// Only used if `common_stack_information.state` is `Suspended` or `Fresh`. In
    /// that case, this points to the end of the stack chain (i.e., the
    /// continuation in the parent chain whose own `parent_chain` field is
    /// `StackChain::Absent`).
    /// Note that this may be a pointer to iself (if the state is `Fresh`, this is always the case).
    pub last_ancestor: *mut VMContRef,

    /// The underlying stack.
    pub stack: ContinuationStack,

    /// Used to store only
    /// 1. The arguments to the function passed to cont.new
    /// 2. The return values of that function
    ///
    /// Note that the actual data buffer (i.e., the one `args.data` points
    /// to) is always allocated on this continuation's stack.
    pub args: Payloads,

    /// Once a continuation has been suspended (using suspend or switch),
    /// this buffer is used to pass payloads to and from the continuation.
    /// More concretely, it is used to
    /// - Pass payloads from a suspend instruction to the corresponding handler.
    /// - Pass payloads to a continuation using cont.bind or resume
    /// - Pass payloads to the continuation being switched to when using switch.
    ///
    /// Note that the actual data buffer (i.e., the one `values.data` points
    /// to) is always allocated on this continuation's stack.
    pub values: Payloads,

    /// Revision counter.
    pub revision: u64,

    /// Tell the compiler that this structure has potential self-references
    /// through the `last_ancestor` pointer.
    _marker: core::marker::PhantomPinned,
}

impl VMContRef {
    pub fn fiber_stack(&self) -> &ContinuationStack {
        &self.stack
    }

    pub fn detach_stack(&mut self) -> ContinuationStack {
        core::mem::replace(&mut self.stack, ContinuationStack::unallocated())
    }

    /// This is effectively a `Default` implementation, without calling it
    /// so. Used to create `VMContRef`s when initializing pooling allocator.
    #[allow(clippy::cast_possible_truncation)]
    pub fn empty() -> Self {
        let limits = StackLimits::with_stack_limit(Default::default());
        let state = State::Fresh;
        let handlers = HandlerList::empty();
        let common_stack_information = CommonStackInformation {
            limits,
            state,
            handlers,
            first_switch_handler_index: 0,
        };
        let parent_chain = StackChain::Absent;
        let last_ancestor = core::ptr::null_mut();
        let stack = ContinuationStack::unallocated();
        let args = Payloads::empty();
        let values = Payloads::empty();
        let revision = 0;
        let _marker = PhantomPinned;

        Self {
            common_stack_information,
            parent_chain,
            last_ancestor,
            stack,
            args,
            values,
            revision,
            _marker,
        }
    }
}

impl Drop for VMContRef {
    fn drop(&mut self) {
        // Note that continuation references do not own their parents, and we
        // don't drop them here.

        // We would like to enforce the invariant that any continuation that
        // was created for a cont.new (rather than, say, just living in a
        // pool and never being touched), either ran to completion or was
        // cancelled. But failing to do so should yield a custom error,
        // instead of panicking here.
    }
}

// These are required so the WasmFX pooling allocator can store a Vec of
// `VMContRef`s.
unsafe impl Send for VMContRef {}
unsafe impl Sync for VMContRef {}

/// Implements `cont.new` instructions (i.e., creation of continuations).
#[inline(always)]
pub fn cont_new(
    store: &mut dyn VMStore,
    instance: &mut Instance,
    func: *mut u8,
    param_count: u32,
    result_count: u32,
) -> Result<*mut VMContRef, TrapReason> {
    let caller_vmctx = instance.vmctx();

    let stack_size = store.engine().config().stack_switching_config.stack_size;

    let contref = store.allocate_continuation()?;
    let contref = unsafe { contref.as_mut().unwrap() };

    let tsp = contref.stack.top().unwrap();
    contref.parent_chain = StackChain::Absent;
    // The continuation is fresh, which is a special case of being suspended.
    // Thus we need to set the correct end of the continuation chain: itself.
    contref.last_ancestor = contref;

    // The initialization function will allocate the actual args/return value buffer and
    // update this object (if needed).
    let contref_args_ptr = &mut contref.args as *mut _ as *mut Array<ValRaw>;

    contref.stack.initialize(
        func.cast::<VMFuncRef>(),
        caller_vmctx.as_ptr(),
        contref_args_ptr,
        param_count,
        result_count,
    );

    // Now that the initial stack pointer was set by the initialization
    // function, use it to determine stack limit.
    let stack_pointer = contref.stack.control_context_stack_pointer();
    // Same caveat regarding stack_limit here as descibed in
    // `wasmtime::runtime::func::RuntimeEntryState::enter_wasm`.
    let wasm_stack_limit = core::cmp::max(
        stack_pointer - store.engine().config().max_wasm_stack,
        tsp as usize - stack_size,
    );
    let limits = StackLimits::with_stack_limit(wasm_stack_limit);
    let csi = &mut contref.common_stack_information;
    csi.state = State::Fresh;
    csi.limits = limits;

    debug_println!("Created contref @ {:p}", contref);
    Ok(contref)
}

// Tests
#[test]
fn offset_and_size_constants() {
    use core::mem::offset_of;
    use wasmtime_environ::stack_switching::offsets::*;

    assert_eq!(
        offset_of!(VMContRef, common_stack_information),
        vm_cont_ref::COMMON_STACK_INFORMATION
    );
    assert_eq!(
        offset_of!(VMContRef, parent_chain),
        vm_cont_ref::PARENT_CHAIN
    );
    assert_eq!(
        offset_of!(VMContRef, last_ancestor),
        vm_cont_ref::LAST_ANCESTOR
    );
    assert_eq!(offset_of!(VMContRef, stack), vm_cont_ref::STACK);
    assert_eq!(offset_of!(VMContRef, args), vm_cont_ref::ARGS);
    assert_eq!(offset_of!(VMContRef, values), vm_cont_ref::VALUES);

    assert_eq!(offset_of!(VMContRef, revision), vm_cont_ref::REVISION);

    assert_eq!(core::mem::size_of::<ContinuationStack>(), FIBER_STACK_SIZE);
    assert_eq!(core::mem::size_of::<StackChain>(), STACK_CHAIN_SIZE);

    // `CommonStackInformation` and `StackLimits` offsets don't need tests because
    // they are defined diretly with `offset_of!`
}

/// This type represents a linked lists ("chain") of stacks, where the a
/// node's successor denotes its parent.
/// A additionally, a `CommonStackInformation` object is associated with
/// each stack in the list.
/// Here, a "stack" is one of the following:
/// - A continuation (i.e., created with cont.new).
/// - The initial stack. This is the stack that we were on when entering
///   Wasm (i.e., when executing
///   `crate::runtime::func::invoke_wasm_and_catch_traps`).
///   This stack never has a parent.
///   In terms of the memory allocation that this stack resides on, it will
///   usually be the main stack, but doesn't have to: If we are running
///   inside a continuation while executing a host call, which in turn
///   re-renters Wasm, the initial stack is actually the stack of that
///   continuation.
///
/// Note that the linked list character of `StackChain` arises from the fact
/// that `StackChain::Continuation` variants have a pointer to a
/// `VMContRef`, which in turn has a `parent_chain` value of type
/// `StackChain`. This is how the stack chain reflects the parent-child
/// relationships between continuations/stacks. This also shows how the
/// initial stack (mentioned above) cannot have a parent.
///
/// There are generally two uses of `StackChain`:
///
/// 1. The `stack_chain` field in the `StoreOpaque` contains such a
/// chain of stacks, where the head of the list denotes the stack that is
/// currently executing (either a continuation or the initial stack). Note
/// that in this case, the linked list must contain 0 or more `Continuation`
/// elements, followed by a final `InitialStack` element. In particular,
/// this list always ends with `InitialStack` and never contains an `Absent`
/// variant.
///
/// 2. When a continuation is suspended, its chain of parents eventually
/// ends with an `Absent` variant in its `parent_chain` field. Note that a
/// suspended continuation never appears in the stack chain in the
/// VMContext!
///
///
/// As mentioned before, each stack in a `StackChain` has a corresponding
/// `CommonStackInformation` object. For continuations, this is stored in
/// the `common_stack_information` field of the corresponding `VMContRef`.
/// For the initial stack, the `InitialStack` variant contains a pointer to
/// a `CommonStackInformation`. The latter will be allocated allocated on
/// the stack frame that executed by `invoke_wasm_and_catch_traps`.
///
/// The following invariants hold for these `StackLimits` objects,
/// and the data in `VMRuntimeLimits`.
///
/// Currently executing stack: For the currently executing stack (i.e., the
/// stack that is at the head of the store's `stack_chain` list), the
/// associated `StackLimits` object contains stale/undefined data. Instead,
/// the live data describing the limits for the currently executing stack is
/// always maintained in `VMRuntimeLimits`. Note that as a general rule
/// independently from any execution of continuations, the `last_wasm_exit*`
/// fields in the `VMRuntimeLimits` contain undefined values while executing
/// wasm.
///
/// Parents of currently executing stack: For stacks that appear in the tail
/// of the store's `stack_chain` list (i.e., stacks that are not currently
/// executing themselves, but are an ancestor of the currently executing
/// stack), we have the following: All the fields in the stack's
/// `StackLimits` are valid, describing the stack's stack limit, and
/// pointers where executing for that stack entered and exited WASM.
///
/// Suspended continuations: For suspended continuations (including their
/// ancestors), we have the following. Note that the initial stack can never
/// be in this state. The `stack_limit` and `last_enter_wasm_sp` fields of
/// the corresponding `StackLimits` object contain valid data, while the
/// `last_exit_wasm_*` fields contain arbitrary values. There is only one
/// exception to this: Note that a continuation that has been created with
/// cont.new, but never been resumed so far, is considered "suspended".
/// However, its `last_enter_wasm_sp` field contains undefined data. This is
/// justified, because when resume-ing a continuation for the first time, a
/// native-to-wasm trampoline is called, which sets up the
/// `last_wasm_entry_sp` in the `VMRuntimeLimits` with the correct value,
/// thus restoring the necessary invariant.
#[derive(Debug, Clone, PartialEq)]
#[repr(usize, C)]
pub enum StackChain {
    /// For suspended continuations, denotes the end of their chain of
    /// ancestors.
    Absent = wasmtime_environ::stack_switching::STACK_CHAIN_ABSENT_DISCRIMINANT,
    /// Represents the initial stack (i.e., where we entered Wasm from the
    /// host by executing
    /// `crate::runtime::func::invoke_wasm_and_catch_traps`). Therefore, it
    /// does not have a parent. The `CommonStackInformation` that this
    /// variant points to is stored in the stack frame of
    /// `invoke_wasm_and_catch_traps`.
    InitialStack(*mut CommonStackInformation) =
        wasmtime_environ::stack_switching::STACK_CHAIN_INITIAL_STACK_DISCRIMINANT,
    /// Represents a continuation's stack.
    Continuation(*mut VMContRef) =
        wasmtime_environ::stack_switching::STACK_CHAIN_CONTINUATION_DISCRIMINANT,
}

impl StackChain {
    /// Indicates if `self` is a `InitialStack` variant.
    pub fn is_initial_stack(&self) -> bool {
        matches!(self, StackChain::InitialStack(_))
    }

    /// Returns an iterator over the continuations in this chain.
    /// We don't implement `IntoIterator` because our iterator is unsafe, so at
    /// least this gives us some way of indicating this, even though the actual
    /// unsafety lies in the `next` function.
    ///
    /// # Safety
    ///
    /// This function is not unsafe per see, but it returns an object
    /// whose usage is unsafe.
    pub unsafe fn into_continuation_iter(self) -> ContinuationIterator {
        ContinuationIterator(self)
    }

    /// Returns an iterator over the stack limits in this chain.
    /// We don't implement `IntoIterator` because our iterator is unsafe, so at
    /// least this gives us some way of indicating this, even though the actual
    /// unsafety lies in the `next` function.
    ///
    /// # Safety
    ///
    /// This function is not unsafe per see, but it returns an object
    /// whose usage is unsafe.
    pub unsafe fn into_stack_limits_iter(self) -> StackLimitsIterator {
        StackLimitsIterator(self)
    }
}

/// Iterator for Continuations in a stack chain.
pub struct ContinuationIterator(StackChain);

/// Iterator for StackLimits in a stack chain.
pub struct StackLimitsIterator(StackChain);

impl Iterator for ContinuationIterator {
    type Item = *mut VMContRef;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            StackChain::Absent | StackChain::InitialStack(_) => None,
            StackChain::Continuation(ptr) => {
                let continuation = unsafe { ptr.as_mut().unwrap() };
                self.0 = continuation.parent_chain.clone();
                Some(ptr)
            }
        }
    }
}

impl Iterator for StackLimitsIterator {
    type Item = *mut StackLimits;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            StackChain::Absent => None,
            StackChain::InitialStack(csi) => {
                let stack_limits = unsafe { &mut (*csi).limits } as *mut StackLimits;
                self.0 = StackChain::Absent;
                Some(stack_limits)
            }
            StackChain::Continuation(ptr) => {
                let continuation = unsafe { ptr.as_mut().unwrap() };
                let stack_limits =
                    (&mut continuation.common_stack_information.limits) as *mut StackLimits;
                self.0 = continuation.parent_chain.clone();
                Some(stack_limits)
            }
        }
    }
}

#[repr(transparent)]
/// Wraps a `StackChain` in an `UnsafeCell`, in order to store it in a
/// `StoreOpaque`.
pub struct StackChainCell(pub UnsafeCell<StackChain>);

impl StackChainCell {
    /// Indicates if the underlying `StackChain` object has value `Absent`.
    pub fn absent() -> Self {
        StackChainCell(UnsafeCell::new(StackChain::Absent))
    }
}

// Since `StackChainCell` objects appear in the `StoreOpaque`,
// they need to be `Send` and `Sync`.
// This is safe for the same reason it is for `VMRuntimeLimits` (see comment
// there): Both types are pod-type with no destructor, and we don't access any
// of their fields from other threads.
unsafe impl Send for StackChainCell {}
unsafe impl Sync for StackChainCell {}

/// FIXME(frank-emrich) Justify why this is safe
unsafe impl crate::vm::VmSafe for StackChainCell {}
