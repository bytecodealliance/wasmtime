//! Exception-throw logic for Wasm exceptions.

use core::ptr::NonNull;

use wasmtime_environ::TagIndex;
use wasmtime_unwinder::{Frame, ThrowAction};

use super::{VMContext, VMStore};
use crate::store::AutoAssertNoGc;

/// Compute the target of the pending exception on the store.
///
/// # Safety
///
/// Must be invoked when Wasm is in the stack and control has
/// re-entered the runtime.
pub unsafe fn compute_throw_action(store: &mut dyn VMStore) -> ThrowAction {
    let mut nogc = AutoAssertNoGc::new(store.store_opaque_mut());

    // Get the tag identity relative to the store.
    let exnref = nogc.take_pending_exception(); // Temporarily take, to avoid borrowing issues.
    let (throwing_tag_instance_id, throwing_tag_defined_tag_index) =
        exnref.tag(&mut nogc).expect("cannot read tag");
    nogc.set_pending_exception(exnref);
    log::trace!(
        "throwing: tag defined in instance {throwing_tag_instance_id:?} defined-tag {throwing_tag_defined_tag_index:?}"
    );

    // Get the state needed for a stack walk.
    let (exit_pc, exit_trampoline_fp, entry_fp) = unsafe {
        (
            *nogc.vm_store_context().last_wasm_exit_pc.get(),
            *nogc.vm_store_context().last_wasm_exit_trampoline_fp.get(),
            *nogc.vm_store_context().last_wasm_entry_fp.get(),
        )
    };

    // Walk the stack, looking up the module with each PC, and using
    // that module to resolve local tag indices into (instance, tag)
    // tuples.
    let handler_lookup = |frame: &Frame| -> Option<usize> {
        log::trace!(
            "exception-throw stack walk: frame at FP={:x} SP={:x} PC={:x}",
            frame.fp(),
            frame.sp().unwrap(),
            frame.pc()
        );
        let module = nogc.modules().lookup_module_by_pc(frame.pc())?;
        let base = module.code_object().code_memory().text().as_ptr() as usize;
        let rel_pc = u32::try_from(frame.pc().wrapping_sub(base)).expect("Module larger than 4GiB");
        let et = module.exception_table();
        for handler in et.lookup_pc(rel_pc) {
            log::trace!("-> checking handler: {handler:?}");
            let is_match = match handler.tag {
                // Catch-all/default handler. Always come last in sequence.
                None => true,
                Some(module_local_tag_index) => {
                    let frame_vmctx = unsafe {
                        frame
                            .read_slot(
                                usize::try_from(
                                    handler
                                        .context_sp_offset
                                        .expect("dynamic context not present for handler record"),
                                )
                                .unwrap(),
                            )
                            .unwrap()
                    };
                    log::trace!("-> read vmctx from frame: {frame_vmctx:x}");
                    let frame_vmctx =
                        NonNull::new(frame_vmctx as *mut VMContext).expect("null vmctx in frame");

                    // SAFETY: we use `instance_from_vmctx` to get an
                    // `&Instance` from a raw vmctx we read off the
                    // stack frame. That method's safety requirements
                    // are that the `vmctx` is a valid vmctx belonging
                    // to an instance in the store (`nogc`). This is
                    // satisfied because every Wasm frame in this
                    // activation must belong to an instance in this
                    // store: we do not permit cross-store calls
                    // without exiting through host code.
                    let (handler_tag_instance, handler_tag_index) = unsafe {
                        let store_id = nogc.id();
                        let instance = nogc.instance_from_vmctx(frame_vmctx);
                        let tag = instance
                            .get_exported_tag(store_id, TagIndex::from_u32(module_local_tag_index));
                        tag.to_raw_indices()
                    };
                    log::trace!(
                        "-> handler's tag {module_local_tag_index:?} resolves to instance {handler_tag_instance:?} defined-tag {handler_tag_index:?}"
                    );

                    handler_tag_instance == throwing_tag_instance_id
                        && handler_tag_index == throwing_tag_defined_tag_index
                }
            };
            if is_match {
                return Some(base.wrapping_add(
                    usize::try_from(handler.handler_offset).expect("Module larger than usize"),
                ));
            }
        }
        None
    };
    let action = unsafe {
        wasmtime_unwinder::compute_throw_action(
            &wasmtime_unwinder::UnwindHost,
            handler_lookup,
            exit_pc,
            exit_trampoline_fp,
            entry_fp,
        )
    };

    log::trace!("throw action: {action:?}");
    action
}
