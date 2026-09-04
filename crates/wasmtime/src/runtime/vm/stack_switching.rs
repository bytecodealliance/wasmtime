//! This module contains the runtime components of the implementation of the
//! stack switching proposal.

mod stack;

use crate::vm::{VMCommonStackInformation, VMContRef, VMHostArray, VMStackLimits};
use core::{marker::PhantomPinned, ptr::NonNull};

pub use stack::*;

// This constant is used to signal that a trap occurred inside a child
// continuation. Morally, it should be part of
// `wasmtime_cranelift::stack_switching::control_effect`, but that
// would require increasing the dependency surface considerably just
// for a constant, thus we define it here so that it can be referenced
// by the assembly stubs.
#[allow(dead_code, reason = "Only referenced by assembly stubs")]
pub const CONTROL_EFFECT_TRAP_ENCODING: u64 =
    (wasmtime_environ::CONTROL_EFFECT_TRAP_DISCRIMINANT as u64) << 32;

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
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VMContObj {
    pub contref: NonNull<VMContRef>,
    pub revision: usize,
}

impl VMContObj {
    pub fn new(contref: NonNull<VMContRef>, revision: usize) -> Self {
        Self { contref, revision }
    }

    /// Construction a VMContinuationObject from a pointer and revision
    ///
    /// The `contref` pointer may be null in which case None will be returned.
    ///
    /// # Safety
    ///
    /// Behavior will be undefined if a pointer to data that is not a
    /// VMContRef is provided.
    pub unsafe fn from_raw_parts(contref: *mut u8, revision: usize) -> Option<Self> {
        NonNull::new(contref.cast::<VMContRef>()).map(|contref| Self::new(contref, revision))
    }
}

unsafe impl Send for VMContObj {}
unsafe impl Sync for VMContObj {}

impl VMCommonStackInformation {
    /// Default value with state set to `Running`
    pub fn running_default() -> Self {
        Self {
            limits: VMStackLimits::default(),
            state: VMStackState::Running,
            handlers: VMHostArray::empty(),
            first_switch_handler_index: 0,
        }
    }
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

impl VMHostArray {
    /// Creates an empty array.
    pub fn empty() -> Self {
        Self {
            length: 0,
            capacity: 0,
            data: core::ptr::null_mut(),
        }
    }
}

impl VMContRef {
    pub fn fiber_stack(&self) -> &VMContinuationStack {
        &self.stack
    }

    pub fn detach_stack(&mut self) -> VMContinuationStack {
        core::mem::replace(&mut self.stack, VMContinuationStack::unallocated())
    }

    /// This is effectively a `Default` implementation, without calling it
    /// so. Used to create `VMContRef`s when initializing pooling allocator.
    pub fn empty() -> Self {
        let limits = VMStackLimits::with_stack_limit(Default::default());
        let state = VMStackState::Fresh;
        let handlers = VMHostArray::empty();
        let common_stack_information = VMCommonStackInformation {
            limits,
            state,
            handlers,
            first_switch_handler_index: 0,
        };
        let parent_chain = VMStackChain::Absent;
        let last_ancestor = core::ptr::null_mut();
        let stack = VMContinuationStack::unallocated();
        let args = VMHostArray::empty();
        let values = VMHostArray::empty();
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
#[cfg(feature = "stack-switching")]
#[inline(always)]
pub fn cont_new(
    store: &mut dyn crate::vm::VMStore,
    instance: crate::store::InstanceId,
    func: *mut u8,
    param_count: u32,
    result_count: u32,
) -> crate::Result<*mut VMContRef> {
    let instance = store.instance_mut(instance);
    let caller_vmctx = instance.vmctx();

    let stack_size = store.engine().config().async_stack_size;

    let contref = store.allocate_continuation()?;
    let contref = unsafe { contref.as_mut().unwrap() };

    let tsp = contref.stack.top().unwrap();
    contref.parent_chain = VMStackChain::Absent;
    // The continuation is fresh, which is a special case of being suspended.
    // Thus we need to set the correct end of the continuation chain: itself.
    contref.last_ancestor = contref;

    // The initialization function will allocate the actual args/return value buffer and
    // update this object (if needed).
    let contref_args_ptr = &mut contref.args as *mut VMHostArray;

    contref.stack.initialize(
        func.cast::<crate::vm::VMFuncRef>(),
        caller_vmctx.as_ptr(),
        contref_args_ptr,
        param_count,
        result_count,
    )?;

    // Now that the initial stack pointer was set by the initialization
    // function, use it to determine stack limit.
    let stack_pointer = contref.stack.control_context_stack_pointer();
    // Same caveat regarding stack_limit here as described in
    // `wasmtime::runtime::func::EntryStoreContext::enter_wasm`.
    let wasm_stack_limit = core::cmp::max(
        stack_pointer - store.engine().config().max_wasm_stack,
        tsp as usize - stack_size,
    );
    let limits = VMStackLimits::with_stack_limit(wasm_stack_limit);
    let csi = &mut contref.common_stack_information;
    csi.state = VMStackState::Fresh;
    csi.limits = limits;

    log::trace!("Created contref @ {contref:p}");
    Ok(contref)
}

/// This type represents a linked lists ("chain") of stacks, where the a
/// node's successor denotes its parent.
/// Additionally, a `CommonStackInformation` object is associated with
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
/// Note that the linked list character of `VMStackChain` arises from the fact
/// that `VMStackChain::Continuation` variants have a pointer to a
/// `VMContRef`, which in turn has a `parent_chain` value of type
/// `VMStackChain`. This is how the stack chain reflects the parent-child
/// relationships between continuations/stacks. This also shows how the
/// initial stack (mentioned above) cannot have a parent.
///
/// There are generally two uses of `VMStackChain`:
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
/// As mentioned before, each stack in a `VMStackChain` has a corresponding
/// `CommonStackInformation` object. For continuations, this is stored in
/// the `common_stack_information` field of the corresponding `VMContRef`.
/// For the initial stack, the `InitialStack` variant contains a pointer to
/// a `CommonStackInformation`. The latter will be allocated allocated on
/// the stack frame that executed by `invoke_wasm_and_catch_traps`.
///
/// The following invariants hold for these `VMStackLimits` objects,
/// and the data in `VMStoreContext`.
///
/// Currently executing stack: For the currently executing stack (i.e., the
/// stack that is at the head of the store's `stack_chain` list), the
/// associated `VMStackLimits` object contains stale/undefined data. Instead,
/// the live data describing the limits for the currently executing stack is
/// always maintained in `VMStoreContext`. Note that as a general rule
/// independently from any execution of continuations, the `last_wasm_exit*`
/// fields in the `VMStoreContext` contain undefined values while executing
/// wasm.
///
/// Parents of currently executing stack: For stacks that appear in the tail
/// of the store's `stack_chain` list (i.e., stacks that are not currently
/// executing themselves, but are an ancestor of the currently executing
/// stack), we have the following: All the fields in the stack's
/// `VMStackLimits` are valid, describing the stack's stack limit, and
/// pointers where executing for that stack entered and exited Wasm.
///
/// Suspended continuations: For suspended continuations (including their
/// ancestors), we have the following. Note that the initial stack can never
/// be in this state. The `stack_limit` and `last_enter_wasm_sp` fields of
/// the corresponding `VMStackLimits` object contain valid data, while the
/// `last_exit_wasm_*` fields contain arbitrary values. There is only one
/// exception to this: Note that a continuation that has been created with
/// cont.new, but never been resumed so far, is considered "suspended".
/// However, its `last_enter_wasm_sp` field contains undefined data. This is
/// justified, because when resume-ing a continuation for the first time, a
/// native-to-wasm trampoline is called, which sets up the
/// `last_wasm_entry_sp` in the `VMStoreContext` with the correct value,
/// thus restoring the necessary invariant.
#[derive(Debug, Clone, PartialEq)]
#[repr(usize, C)]
pub enum VMStackChain {
    /// For suspended continuations, denotes the end of their chain of
    /// ancestors.
    Absent = wasmtime_environ::STACK_CHAIN_ABSENT_DISCRIMINANT,
    /// Represents the initial stack (i.e., where we entered Wasm from the
    /// host by executing
    /// `crate::runtime::func::invoke_wasm_and_catch_traps`). Therefore, it
    /// does not have a parent. The `CommonStackInformation` that this
    /// variant points to is stored in the stack frame of
    /// `invoke_wasm_and_catch_traps`.
    InitialStack(*mut VMCommonStackInformation) =
        wasmtime_environ::STACK_CHAIN_INITIAL_STACK_DISCRIMINANT,
    /// Represents a continuation's stack.
    Continuation(*mut VMContRef) = wasmtime_environ::STACK_CHAIN_CONTINUATION_DISCRIMINANT,
}

impl VMStackChain {
    /// Indicates if `self` is a `InitialStack` variant.
    pub fn is_initial_stack(&self) -> bool {
        matches!(self, VMStackChain::InitialStack(_))
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
pub struct ContinuationIterator(VMStackChain);

/// Iterator for VMStackLimits in a stack chain.
pub struct StackLimitsIterator(VMStackChain);

impl Iterator for ContinuationIterator {
    type Item = *mut VMContRef;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            VMStackChain::Absent | VMStackChain::InitialStack(_) => None,
            VMStackChain::Continuation(ptr) => {
                let continuation = unsafe { ptr.as_mut().unwrap() };
                self.0 = continuation.parent_chain.clone();
                Some(ptr)
            }
        }
    }
}

impl Iterator for StackLimitsIterator {
    type Item = *mut VMStackLimits;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            VMStackChain::Absent => None,
            VMStackChain::InitialStack(csi) => {
                let stack_limits = unsafe { &mut (*csi).limits } as *mut VMStackLimits;
                self.0 = VMStackChain::Absent;
                Some(stack_limits)
            }
            VMStackChain::Continuation(ptr) => {
                let continuation = unsafe { ptr.as_mut().unwrap() };
                let stack_limits =
                    (&mut continuation.common_stack_information.limits) as *mut VMStackLimits;
                self.0 = continuation.parent_chain.clone();
                Some(stack_limits)
            }
        }
    }
}

/// Encodes the life cycle of a `VMContRef`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum VMStackState {
    /// The `VMContRef` has been created, but neither `resume` or `switch` has ever been
    /// called on it. During this stage, we may add arguments using `cont.bind`.
    Fresh = wasmtime_environ::STACK_STATE_FRESH_DISCRIMINANT,
    /// The continuation is running, meaning that it is the one currently
    /// executing code.
    Running = wasmtime_environ::STACK_STATE_RUNNING_DISCRIMINANT,
    /// The continuation is suspended because it executed a resume instruction
    /// that has not finished yet. In other words, it became the parent of
    /// another continuation (which may itself be `Running`, a `Parent`, or
    /// `Suspended`).
    Parent = wasmtime_environ::STACK_STATE_PARENT_DISCRIMINANT,
    /// The continuation was suspended by a `suspend` or `switch` instruction.
    Suspended = wasmtime_environ::STACK_STATE_SUSPENDED_DISCRIMINANT,
    /// The function originally passed to `cont.new` has returned normally.
    /// Note that there is no guarantee that a VMContRef will ever
    /// reach this status, as it may stay suspended until being dropped.
    Returned = wasmtime_environ::STACK_STATE_RETURNED_DISCRIMINANT,
    /// The function originally passed to `cont.new` terminated with a trap.
    /// This is a terminal state and the continuation cannot be resumed.
    Trapped = wasmtime_environ::STACK_STATE_TRAPPED_DISCRIMINANT,
}

#[cfg(test)]
mod tests {
    use core::mem::{offset_of, size_of};

    use wasmtime_environ::{HostPtr, Module, PtrSize, StaticModuleIndex, VMOffsets};

    use super::*;

    #[test]
    fn null_pointer_optimization() {
        // The Rust spec does not technically guarantee that the null pointer
        // optimization applies to a struct containing a `NonNull`.
        assert_eq!(size_of::<Option<VMContObj>>(), size_of::<VMContObj>());
    }

    #[test]
    fn check_vm_contobj_offsets() {
        let module = Module::new(StaticModuleIndex::from_u32(0));
        let offsets = VMOffsets::new(HostPtr, &module);
        assert_eq!(
            offset_of!(VMContObj, contref),
            usize::from(offsets.ptr.vmcontobj_contref())
        );
        assert_eq!(
            offset_of!(VMContObj, revision),
            usize::from(offsets.ptr.vmcontobj_revision())
        );
        assert_eq!(
            size_of::<VMContObj>(),
            usize::from(offsets.ptr.size_of_vmcontobj())
        )
    }

    #[test]
    fn check_vm_stack_chain_offsets() {
        let module = Module::new(StaticModuleIndex::from_u32(0));
        let offsets = VMOffsets::new(HostPtr, &module);
        assert_eq!(
            size_of::<VMStackChain>(),
            usize::from(offsets.ptr.size_of_vmstack_chain())
        );
    }
}
