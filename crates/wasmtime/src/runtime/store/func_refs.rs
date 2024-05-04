//! Lifetime management of `VMFuncRef`s inside of stores, and filling in their
//! trampolines.

use crate::module::ModuleRegistry;
use crate::prelude::*;
use crate::runtime::vm::{SendSyncPtr, VMFuncRef, VMNativeCallHostFuncContext};
use alloc::sync::Arc;
use core::ptr::NonNull;

/// An arena of `VMFuncRef`s.
///
/// Allows a store to pin and own funcrefs so that it can patch in trampolines
/// for `VMFuncRef`s that are missing a `wasm_call` trampoline and
/// need Wasm to supply it.
#[derive(Default)]
pub struct FuncRefs {
    /// A bump allocation arena where we allocate `VMFuncRef`s such
    /// that they are pinned and owned.
    bump: SendSyncBump,

    /// Pointers into `self.bump` for entries that need `wasm_call` field filled
    /// in.
    with_holes: Vec<SendSyncPtr<VMFuncRef>>,

    /// Pinned `VMFuncRef`s that had their `wasm_call` field
    /// pre-patched when constructing an `InstancePre`, and which we need to
    /// keep alive for our owning store's lifetime.
    instance_pre_func_refs: Vec<Arc<[VMFuncRef]>>,
}

use send_sync_bump::SendSyncBump;
mod send_sync_bump {
    #[derive(Default)]
    pub struct SendSyncBump(bumpalo::Bump);

    impl SendSyncBump {
        pub fn alloc<T>(&mut self, val: T) -> &mut T {
            self.0.alloc(val)
        }
    }

    // Safety: We require `&mut self` on the only public method, which means it
    // is safe to send `&SendSyncBump` references across threads because they
    // can't actually do anything with it.
    unsafe impl Sync for SendSyncBump {}
}

impl FuncRefs {
    /// Push the given `VMFuncRef` into this arena, returning a
    /// pinned pointer to it.
    ///
    /// # Safety
    ///
    /// You may only access the return value on the same thread as this
    /// `FuncRefs` and only while the store holding this `FuncRefs` exists.
    pub unsafe fn push(&mut self, func_ref: VMFuncRef) -> NonNull<VMFuncRef> {
        debug_assert!(func_ref.wasm_call.is_none());
        // Debug assert that the vmctx is a `VMNativeCallHostFuncContext` as
        // that is the only kind that can have holes.
        let _ = unsafe { VMNativeCallHostFuncContext::from_opaque(func_ref.vmctx) };

        let func_ref = self.bump.alloc(func_ref);
        let unpatched = SendSyncPtr::from(func_ref);
        let ret = unpatched.as_non_null();
        self.with_holes.push(unpatched);
        ret
    }

    /// Patch any `VMFuncRef::wasm_call`s that need filling in.
    pub fn fill(&mut self, modules: &ModuleRegistry) {
        self.with_holes.retain_mut(|f| {
            unsafe {
                let func_ref = f.as_mut();
                debug_assert!(func_ref.wasm_call.is_none());

                // Debug assert that the vmctx is a `VMNativeCallHostFuncContext` as
                // that is the only kind that can have holes.
                let _ = VMNativeCallHostFuncContext::from_opaque(func_ref.vmctx);

                func_ref.wasm_call = modules.wasm_to_native_trampoline(func_ref.type_index);
                func_ref.wasm_call.is_none()
            }
        });
    }

    /// Push pre-patched `VMFuncRef`s from an `InstancePre`.
    pub fn push_instance_pre_func_refs(&mut self, func_refs: Arc<[VMFuncRef]>) {
        self.instance_pre_func_refs.push(func_refs);
    }
}
