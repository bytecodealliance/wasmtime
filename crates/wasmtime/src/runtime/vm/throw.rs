//! Exception-throw logic for Wasm exceptions.

use super::{VMContext, VMStore};
use crate::{store::AutoAssertNoGc, vm::Instance};
use core::ptr::NonNull;
use wasmtime_environ::TagIndex;
use wasmtime_unwinder::{Frame, Handler};

/// Compute the target of the pending exception on the store.
///
/// # Safety
///
/// The stored last-exit state in `store` either must be valid, or
/// must have a zeroed exit FP if no Wasm is on the stack.
pub unsafe fn compute_handler(store: &mut dyn VMStore) -> Option<Handler> {
    let mut nogc = AutoAssertNoGc::new(store.store_opaque_mut());

    // Get the tag identity relative to the store.

    // Temporarily take, to avoid borrowing issues.
    let exnref = nogc
        .take_pending_exception()
        .expect("Only invoked when an exception is pending");
    let (throwing_tag_instance_id, throwing_tag_defined_tag_index) =
        exnref.tag(&mut nogc).expect("cannot read tag");
    nogc.set_pending_exception(exnref);
    log::trace!(
        "throwing: tag defined in instance {throwing_tag_instance_id:?} defined-tag {throwing_tag_defined_tag_index:?}"
    );

    // Get the state needed for a stack walk.
    let (exit_pc, exit_fp, entry_fp) = unsafe {
        (
            *nogc.vm_store_context().last_wasm_exit_pc.get(),
            nogc.vm_store_context().last_wasm_exit_fp(),
            *nogc.vm_store_context().last_wasm_entry_fp.get(),
        )
    };

    // Early out: if there is no exit FP -- which can happen if a host
    // func, wrapped up as a `Func`, is called directly via
    // `Func::call` -- then the only possible action we can take is
    // `None` (i.e., no handler, unwind to entry from host).
    if exit_fp == 0 {
        return None;
    }

    // Walk the stack, looking up the module with each PC, and using
    // that module to resolve local tag indices into (instance, tag)
    // tuples.
    let handler_lookup = |frame: &Frame| -> Option<(usize, usize)> {
        log::trace!(
            "exception-throw stack walk: frame at FP={:x} PC={:x}",
            frame.fp(),
            frame.pc()
        );
        let module = nogc.modules().lookup_module_by_pc(frame.pc())?;
        let base = module.code_object().code_memory().text().as_ptr() as usize;
        let rel_pc = u32::try_from(frame.pc().wrapping_sub(base)).expect("Module larger than 4GiB");
        let et = module.exception_table();
        let (frame_offset, handlers) = et.lookup_pc(rel_pc);
        let fp_to_sp = frame_offset.map(|frame_offset| -isize::try_from(frame_offset).unwrap());
        for handler in handlers {
            log::trace!("-> checking handler: {handler:?}");
            let is_match = match handler.tag {
                // Catch-all/default handler. Always come last in sequence.
                None => true,
                Some(module_local_tag_index) => {
                    let fp_to_sp =
                        fp_to_sp.expect("frame offset is necessary for exception unwind");
                    let fp_offset = fp_to_sp
                        + isize::try_from(
                            handler
                                .context_sp_offset
                                .expect("dynamic context not present for handler record"),
                        )
                        .unwrap();
                    let frame_vmctx = unsafe { frame.read_slot_from_fp(fp_offset) };
                    log::trace!("-> read vmctx from frame: {frame_vmctx:x}");
                    let frame_vmctx =
                        NonNull::new(frame_vmctx as *mut VMContext).expect("null vmctx in frame");

                    // SAFETY: we use `Instance::from_vmctx` to get an
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
                        let instance = Instance::from_vmctx(&nogc, frame_vmctx);
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
                let fp_to_sp = fp_to_sp.expect("frame offset must be known if we found a handler");
                return Some((
                    base.wrapping_add(
                        usize::try_from(handler.handler_offset).expect("Module larger than usize"),
                    ),
                    frame.fp().wrapping_add_signed(fp_to_sp),
                ));
            }
        }
        None
    };
    let unwinder = nogc.unwinder();
    let action = unsafe { Handler::find(unwinder, handler_lookup, exit_pc, exit_fp, entry_fp) };
    log::trace!("throw action: {action:?}");
    action
}
