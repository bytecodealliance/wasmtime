//! Shared translation of the component model's `{enter,exit}-sync-call`
//! intrinsics.
//!
//! Two different places wrap a synchronous call into another component
//! instance with these intrinsics: fused adapters, whose calls are translated
//! in `func_environ.rs`, and the `resource.drop` trampoline's call to a
//! resource destructor, translated in `compiler/component.rs`.
//!
//! Rather than call out to the host to do the task bookkeeping that entering a
//! sync call entails, both push a `VMDeferredThread` into their own stack frame
//! and publish it as the store's current thread. The host then only promotes it
//! into a real `GuestTask` if it ever actually needs one; see
//! `StoreOpaque::force_current_thread` and
//! `StoreOpaque::force_deferred_current_thread`.
//!
//! The two callers differ only in where the arguments recorded by `enter` come
//! from and in how `exit`'s out-of-line slow path is called. Everything else
//! lives here: the `VMDeferredThread` layout and, in particular, the
//! `context.{get,set}` slot save/zero/restore, which has to match what the host
//! does for a non-deferred thread in `StoreOpaque::set_thread`.

use crate::alias_region::AliasRegions;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_frontend::FunctionBuilder;
use wasmtime_environ::{GetPtrSize, NUM_COMPONENT_CONTEXT_SLOTS, PtrSize};

/// The `enter_sync_call` arguments recorded into the pushed
/// `VMDeferredThread`, to be replayed by the host if it ever has to promote the
/// deferred thread into a real one.
pub struct EnterArgs {
    /// The component instance performing the call.
    pub caller_instance: ir::Value,
    /// Whether the callee is async-lifted, as an `i32` boolean.
    pub callee_async: ir::Value,
    /// The component instance being called into.
    pub callee_instance: ir::Value,
}

/// Translates `enter-sync-call`: allocates a `VMDeferredThread` in the current
/// function's frame, records `args` into it along with the caller's context
/// slots, zeroes the live context slots for the callee, and publishes the frame
/// as the store's current thread.
///
/// `vmctx` must be a core wasm `*mut VMContext` for this store. The returned
/// stack slot must be handed to [`exit`] once the callee returns.
pub fn enter<O>(
    builder: &mut FunctionBuilder<'_>,
    alias_regions: &mut AliasRegions<O>,
    vmctx: ir::Value,
    args: EnterArgs,
) -> ir::StackSlot
where
    O: GetPtrSize,
{
    let pointer_type = alias_regions.pointer_type();
    let ptr = alias_regions.ptr_size();

    // Allocate the on-stack `VMDeferredThread`.
    let slot = builder.func.create_sized_stack_slot(ir::StackSlotData::new(
        ir::StackSlotKind::ExplicitSlot,
        u32::from(ptr.vm_deferred_thread().size()),
        u8::try_from(ptr.size().trailing_zeros()).unwrap(),
    ));
    let slot_addr = builder.ins().stack_addr(pointer_type, slot, 0);
    let vmstore = alias_regions
        .vmctx()
        .store_context()
        .load(&mut builder.cursor(), vmctx);

    // Link the previous current thread in as this frame's parent.
    let parent = alias_regions
        .vm_store_context()
        .current_thread()
        .load(&mut builder.cursor(), vmstore);
    alias_regions
        .vm_deferred_thread()
        .parent()
        .store(&mut builder.cursor(), slot_addr, parent);

    // Record the deferred `enter_sync_call` arguments.
    alias_regions.vm_deferred_thread().caller_instance().store(
        &mut builder.cursor(),
        slot_addr,
        args.caller_instance,
    );
    alias_regions.vm_deferred_thread().callee_async().store(
        &mut builder.cursor(),
        slot_addr,
        args.callee_async,
    );
    alias_regions.vm_deferred_thread().callee_instance().store(
        &mut builder.cursor(),
        slot_addr,
        args.callee_instance,
    );

    // Save the caller's context slots into the frame and reset the live values
    // to 0 for the freshly-entered (deferred) thread.
    for i in 0..u8::try_from(NUM_COMPONENT_CONTEXT_SLOTS).unwrap() {
        let saved = alias_regions
            .vm_store_context()
            .component_context(i)
            .load(&mut builder.cursor(), vmstore);
        alias_regions.vm_deferred_thread().saved_context(i).store(
            &mut builder.cursor(),
            slot_addr,
            saved,
        );
        let zero = builder.ins().iconst(ir::types::I32, 0);
        alias_regions.vm_store_context().component_context(i).store(
            &mut builder.cursor(),
            vmstore,
            zero,
        );
    }

    // Publish the deferred thread as the store's current thread.
    alias_regions.vm_store_context().current_thread().store(
        &mut builder.cursor(),
        vmstore,
        slot_addr,
    );

    slot
}

/// The unfinished slow path of an [`exit`], which the caller must fill in with
/// an out-of-line call to the `exit_sync_call` intrinsic before finishing it
/// with [`SlowExit::finish`].
#[must_use = "the slow path of an exit-sync-call must be emitted and finished"]
pub struct SlowExit {
    cont_block: ir::Block,
}

impl SlowExit {
    pub fn finish(self, builder: &mut FunctionBuilder<'_>) {
        builder.ins().jump(self.cont_block, &[]);
        builder.seal_block(self.cont_block);
        builder.switch_to_block(self.cont_block);
    }
}

/// Translates `exit-sync-call`, the counterpart to [`enter`], where `slot` is
/// the stack slot that `enter` returned.
///
/// If the deferred thread pushed by `enter` is still the store's current
/// thread, meaning nothing ever forced it into a real one, then it is popped
/// and the caller's context slots restored inline. Otherwise the host has state
/// to tear down, which only it can do, so the caller must emit an out-of-line
/// call to the `exit_sync_call` intrinsic: on return the builder is positioned
/// in that slow path, which is finished with [`SlowExit::finish`].
pub fn exit<O>(
    builder: &mut FunctionBuilder<'_>,
    alias_regions: &mut AliasRegions<O>,
    vmctx: ir::Value,
    slot: ir::StackSlot,
) -> SlowExit
where
    O: GetPtrSize,
{
    let pointer_type = alias_regions.pointer_type();
    let slot_addr = builder.ins().stack_addr(pointer_type, slot, 0);
    let vmstore = alias_regions
        .vmctx()
        .store_context()
        .load(&mut builder.cursor(), vmctx);
    let cur = alias_regions
        .vm_store_context()
        .current_thread()
        .load(&mut builder.cursor(), vmstore);
    let is_fast = builder.ins().icmp(IntCC::Equal, cur, slot_addr);

    let fast_block = builder.create_block();
    let slow_block = builder.create_block();
    let cont_block = builder.create_block();
    builder
        .ins()
        .brif(is_fast, fast_block, &[], slow_block, &[]);
    builder.seal_block(fast_block);
    builder.seal_block(slow_block);

    // Fast path: pop the deferred thread and restore the caller's context.
    builder.switch_to_block(fast_block);
    let parent = alias_regions
        .vm_deferred_thread()
        .parent()
        .load(&mut builder.cursor(), slot_addr);
    alias_regions
        .vm_store_context()
        .current_thread()
        .store(&mut builder.cursor(), vmstore, parent);
    for i in 0..u8::try_from(NUM_COMPONENT_CONTEXT_SLOTS).unwrap() {
        let saved = alias_regions
            .vm_deferred_thread()
            .saved_context(i)
            .load(&mut builder.cursor(), slot_addr);
        alias_regions.vm_store_context().component_context(i).store(
            &mut builder.cursor(),
            vmstore,
            saved,
        );
    }
    builder.ins().jump(cont_block, &[]);

    // Slow path: the thread was promoted to a real one, so the caller does the
    // equivalent teardown out-of-line.
    builder.switch_to_block(slow_block);
    SlowExit { cont_block }
}
