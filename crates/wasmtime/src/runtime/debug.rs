//! Debugging API.

use crate::{
    AnyRef, ExnRef, ExternRef, Func, Instance, Module, Val,
    store::{AutoAssertNoGc, StoreOpaque},
    vm::{CurrentActivationBacktrace, VMContext},
};
use alloc::vec::Vec;
use core::{ffi::c_void, ptr::NonNull};
use wasmtime_environ::{
    DefinedFuncIndex, FrameInstPos, FrameStackShape, FrameStateSlot, FrameStateSlotOffset,
    FrameTable, FrameTableDescriptorIndex, FrameValType, FuncKey,
};
use wasmtime_unwinder::Frame;

use super::store::AsStoreOpaque;

impl StoreOpaque {
    /// Provide an object that captures Wasm stack state, including
    /// Wasm VM-level values (locals and operand stack).
    ///
    /// This object views all activations for the current store that
    /// are on the stack. An activation is a contiguous sequence of
    /// Wasm frames (called functions) that were called from host code
    /// and called back out to host code. If there are activations
    /// from multiple stores on the stack, for example if Wasm code in
    /// one store calls out to host code which invokes another Wasm
    /// function in another store, then the other stores are "opaque"
    /// to our view here in the same way that host code is.
    ///
    /// Returns `None` if debug instrumentation is not enabled for
    /// the engine containing this store.
    pub fn stack_values(&mut self) -> Option<StackView<'_>> {
        if !self.engine().tunables().debug_guest {
            return None;
        }

        // SAFETY: This takes a mutable borrow of `self` (the
        // `StoreOpaque`), which owns all active stacks in the
        // store. We do not provide any API that could mutate the
        // frames that we are walking on the `StackView`.
        let iter = unsafe { CurrentActivationBacktrace::new(self) };
        let mut view = StackView {
            iter,
            is_trapping_frame: false,
            frames: vec![],
            current: None,
        };
        view.move_up(); // Load the first frame.
        Some(view)
    }
}

/// A view of values in active Wasm stack frames.
///
/// See the documentation on `Store::stack_value` for more information
/// about which frames this view will show.
pub struct StackView<'a> {
    /// Iterator over frames.
    ///
    /// This iterator owns the store while the view exists (accessible
    /// as `iter.store`).
    iter: CurrentActivationBacktrace<'a>,

    /// Is the next frame to be visited by the iterator a trapping
    /// frame?
    ///
    /// This alters how we interpret `pc`: for a trap, we look at the
    /// instruction that *starts* at `pc`, while for all frames
    /// further up the stack (i.e., at a callsite), we look at the
    /// instruction that *ends* at `pc`.
    is_trapping_frame: bool,

    /// Virtual frame queue: decoded from `iter`, not yet
    /// yielded. Innermost frame on top (last).
    ///
    /// This is only non-empty when there is more than one virtual
    /// frame in a physical frame (i.e., for inlining); thus, its size
    /// is bounded by our inlining depth.
    frames: Vec<VirtualFrame>,

    /// Currently focused virtual frame.
    current: Option<FrameData>,
}

impl<'a> StackView<'a> {
    /// Move up to the next frame in the activation.
    pub fn move_up(&mut self) {
        // If there are no virtual frames to yield, take and decode
        // the next physical frame.
        //
        // Note that `if` rather than `while` here, and the assert
        // that we get some virtual frames back, enforce the invariant
        // that each physical frame decodes to at least one virtual
        // frame (i.e., there are no physical frames for interstitial
        // functions or other things that we completely ignore). If
        // this ever changes, we can remove the assert and convert
        // this to a loop that polls until it finds virtual frames.
        self.current = None;
        if self.frames.is_empty() {
            let Some(next_frame) = self.iter.next() else {
                return;
            };
            self.frames =
                VirtualFrame::decode(&mut self.iter.store, next_frame, self.is_trapping_frame);
            debug_assert!(!self.frames.is_empty());
            self.is_trapping_frame = false;
        }

        // Take a frame and focus it as the current one.
        self.current = self.frames.pop().map(|vf| FrameData::compute(vf));
    }

    /// Has the iterator reached the end of the activation?
    pub fn done(&self) -> bool {
        self.current.is_none()
    }

    fn frame_data(&self) -> &FrameData {
        self.current.as_ref().expect("No current frame")
    }

    fn raw_instance(&self) -> &crate::vm::Instance {
        // Read out the vmctx slot.

        // SAFETY: vmctx is always at offset 0 in the slot.
        // (See crates/cranelift/src/func_environ.rs in `update_stack_slot_vmctx()`.)
        let vmctx: *mut VMContext = unsafe { *(self.frame_data().slot_addr as *mut _) };
        let vmctx = NonNull::new(vmctx).expect("null vmctx in debug state slot");
        // SAFETY: the stored vmctx value is a valid instance in this
        // store; we only visit frames from this store in the
        // backtrace.
        let instance = unsafe { crate::vm::Instance::from_vmctx(vmctx) };
        // SAFETY: the instance pointer read above is valid.
        unsafe { instance.as_ref() }
    }

    /// Get the instance associated with the current frame.
    pub fn instance(&mut self) -> Instance {
        let instance = self.raw_instance();
        Instance::from_wasmtime(instance.id(), self.iter.store.as_store_opaque())
    }

    /// Get the module associated with the current frame, if any
    /// (i.e., not a container instance for a host-created entity).
    pub fn module(&self) -> Option<&Module> {
        let instance = self.raw_instance();
        instance.runtime_module()
    }

    /// Get the raw function index associated with the current frame, and the
    /// PC as an offset within its code section, if it is a Wasm
    /// function directly from the given `Module` (rather than a
    /// trampoline).
    pub fn wasm_function_index_and_pc(&self) -> Option<(DefinedFuncIndex, u32)> {
        let data = self.frame_data();
        let FuncKey::DefinedWasmFunction(module, func) = data.func_key else {
            return None;
        };
        debug_assert_eq!(
            module,
            self.module()
                .expect("module should be defined if this is a defined function")
                .env_module()
                .module_index
        );
        Some((func, data.wasm_pc))
    }

    /// Get the number of locals in this frame.
    pub fn num_locals(&self) -> u32 {
        u32::try_from(self.frame_data().locals.len()).unwrap()
    }

    /// Get the depth of the operand stack in this frame.
    pub fn num_stacks(&self) -> u32 {
        u32::try_from(self.frame_data().stack.len()).unwrap()
    }

    /// Get the type and value of the given local in this frame.
    ///
    /// # Panics
    ///
    /// Panics if the index is out-of-range (greater than
    /// `num_locals()`).
    pub fn local(&mut self, index: u32) -> Val {
        let data = self.frame_data();
        let (offset, ty) = data.locals[usize::try_from(index).unwrap()];
        let slot_addr = data.slot_addr;
        // SAFETY: compiler produced metadata to describe this local
        // slot and stored a value of the correct type into it.
        unsafe { read_value(&mut self.iter.store, slot_addr, offset, ty) }
    }

    /// Get the type and value of the given operand-stack value in
    /// this frame.
    ///
    /// Index 0 corresponds to the bottom-of-stack, and higher indices
    /// from there are more recently pushed values.  In other words,
    /// index order reads the Wasm virtual machine's abstract stack
    /// state left-to-right.
    pub fn stack(&mut self, index: u32) -> Val {
        let data = self.frame_data();
        let (offset, ty) = data.stack[usize::try_from(index).unwrap()];
        let slot_addr = data.slot_addr;
        // SAFETY: compiler produced metadata to describe this
        // operand-stack slot and stored a value of the correct type
        // into it.
        unsafe { read_value(&mut self.iter.store, slot_addr, offset, ty) }
    }
}

/// Internal data pre-computed for one stack frame.
///
/// This combines physical frame info (pc, fp) with the module this PC
/// maps to (yielding a frame table) and one frame as produced by the
/// progpoint lookup (Wasm PC, frame descriptor index, stack shape).
struct VirtualFrame {
    /// The frame pointer.
    fp: *const u8,
    /// The resolved module handle for the physical PC.
    ///
    /// The module for each inlined frame within the physical frame is
    /// resolved from the vmctx reachable for each such frame; this
    /// module isused only for looking up the frame table.
    module: Module,
    /// The Wasm PC for this frame.
    wasm_pc: u32,
    /// The frame descriptor for this frame.
    frame_descriptor: FrameTableDescriptorIndex,
    /// The stack shape for this frame.
    stack_shape: FrameStackShape,
}

impl VirtualFrame {
    /// Return virtual frames corresponding to a physical frame, from
    /// outermost to innermost.
    fn decode(
        store: &mut AutoAssertNoGc<'_>,
        frame: Frame,
        is_trapping_frame: bool,
    ) -> Vec<VirtualFrame> {
        let module = store
            .modules()
            .lookup_module_by_pc(frame.pc())
            .expect("Wasm frame PC does not correspond to a module");
        let base = module.code_object().code_memory().text().as_ptr() as usize;
        let pc = frame.pc().wrapping_sub(base);
        let table = module.frame_table().unwrap();
        let pc = u32::try_from(pc).expect("PC offset too large");
        let pos = if is_trapping_frame {
            FrameInstPos::Pre
        } else {
            FrameInstPos::Post
        };
        let program_points = table.find_program_point(pc, pos).expect("There must be a program point record in every frame when debug instrumentation is enabled");

        program_points
            .map(|(wasm_pc, frame_descriptor, stack_shape)| VirtualFrame {
                fp: core::ptr::with_exposed_provenance(frame.fp()),
                module: module.clone(),
                wasm_pc,
                frame_descriptor,
                stack_shape,
            })
            .collect()
    }
}

/// Data computed when we visit a given frame.
struct FrameData {
    slot_addr: *const u8,
    func_key: FuncKey,
    wasm_pc: u32,
    /// Shape of locals in this frame.
    ///
    /// We need to store this locally because `FrameView` cannot
    /// borrow the store: it needs a mut borrow, and an iterator
    /// cannot yield the same mut borrow multiple times because it
    /// cannot control the lifetime of the values it yields (the
    /// signature of `next()` does not bound the return value to the
    /// `&mut self` arg).
    locals: Vec<(FrameStateSlotOffset, FrameValType)>,
    /// Shape of the stack slots at this program point in this frame.
    ///
    /// In addition to the borrowing-related reason above, we also
    /// materialize this because we want to provide O(1) access to the
    /// stack by depth, and the frame slot descriptor stores info in a
    /// linked-list (actually DAG, with dedup'ing) way.
    stack: Vec<(FrameStateSlotOffset, FrameValType)>,
}

impl FrameData {
    fn compute(frame: VirtualFrame) -> Self {
        let frame_table = frame.module.frame_table().unwrap();
        // Parse the frame descriptor.
        let (data, slot_to_fp_offset) = frame_table
            .frame_descriptor(frame.frame_descriptor)
            .unwrap();
        let frame_state_slot = FrameStateSlot::parse(data).unwrap();
        let slot_addr = frame
            .fp
            .wrapping_sub(usize::try_from(slot_to_fp_offset).unwrap());

        // Materialize the stack shape so we have O(1) access to its
        // elements, and so we don't need to keep the borrow to the
        // module alive.
        let mut stack = frame_state_slot
            .stack(frame.stack_shape)
            .collect::<Vec<_>>();
        stack.reverse(); // Put top-of-stack last.

        // Materialize the local offsets/types so we don't need to
        // keep the borrow to the module alive.
        let locals = frame_state_slot.locals().collect::<Vec<_>>();

        FrameData {
            slot_addr,
            func_key: frame_state_slot.func_key(),
            wasm_pc: frame.wasm_pc,
            stack,
            locals,
        }
    }
}

/// Read the value at the given offset.
///
/// # Safety
///
/// The `offset` and `ty` must correspond to a valid value written
/// to the frame by generated code of the correct type. This will
/// be the case if this information comes from the frame tables
/// (as long as the frontend that generates the tables and
/// instrumentation is correct, and as long as the tables are
/// preserved through serialization).
unsafe fn read_value(
    store: &mut AutoAssertNoGc<'_>,
    slot_base: *const u8,
    offset: FrameStateSlotOffset,
    ty: FrameValType,
) -> Val {
    let address = unsafe { slot_base.offset(isize::try_from(offset.offset()).unwrap()) };

    // SAFETY: each case reads a value from memory that should be
    // valid according to our safety condition.
    match ty {
        FrameValType::I32 => {
            let value = unsafe { *(address as *const i32) };
            Val::I32(value)
        }
        FrameValType::I64 => {
            let value = unsafe { *(address as *const i64) };
            Val::I64(value)
        }
        FrameValType::F32 => {
            let value = unsafe { *(address as *const u32) };
            Val::F32(value)
        }
        FrameValType::F64 => {
            let value = unsafe { *(address as *const u64) };
            Val::F64(value)
        }
        FrameValType::V128 => {
            let value = unsafe { *(address as *const u128) };
            Val::V128(value.into())
        }
        FrameValType::AnyRef => {
            let mut nogc = AutoAssertNoGc::new(store);
            let value = unsafe { *(address as *const u32) };
            let value = AnyRef::_from_raw(&mut nogc, value);
            Val::AnyRef(value)
        }
        FrameValType::ExnRef => {
            let mut nogc = AutoAssertNoGc::new(store);
            let value = unsafe { *(address as *const u32) };
            let value = ExnRef::_from_raw(&mut nogc, value);
            Val::ExnRef(value)
        }
        FrameValType::ExternRef => {
            let mut nogc = AutoAssertNoGc::new(store);
            let value = unsafe { *(address as *const u32) };
            let value = ExternRef::_from_raw(&mut nogc, value);
            Val::ExternRef(value)
        }
        FrameValType::FuncRef => {
            let value = unsafe { *(address as *const *mut c_void) };
            let value = unsafe { Func::_from_raw(store, value) };
            Val::FuncRef(value)
        }
        FrameValType::ContRef => {
            unimplemented!("contref values are not implemented in the host API yet")
        }
    }
}

/// Compute raw pointers to all GC refs in the given frame.
// Note: ideally this would be an impl Iterator, but this is quite
// awkward because of the locally computed data (FrameStateSlot::parse
// structured result) within the closure borrowed by a nested closure.
pub(crate) fn gc_refs_in_frame<'a>(ft: FrameTable<'a>, pc: u32, fp: *mut usize) -> Vec<*mut u32> {
    let fp = fp.cast::<u8>();
    let mut ret = vec![];
    if let Some(frames) = ft.find_program_point(pc, FrameInstPos::Post) {
        for (_wasm_pc, frame_desc, stack_shape) in frames {
            let (frame_desc_data, slot_to_fp_offset) = ft.frame_descriptor(frame_desc).unwrap();
            let frame_base = unsafe { fp.offset(-isize::try_from(slot_to_fp_offset).unwrap()) };
            let frame_desc = FrameStateSlot::parse(frame_desc_data).unwrap();
            for (offset, ty) in frame_desc.stack_and_locals(stack_shape) {
                match ty {
                    FrameValType::AnyRef | FrameValType::ExnRef | FrameValType::ExternRef => {
                        let slot = unsafe {
                            frame_base
                                .offset(isize::try_from(offset.offset()).unwrap())
                                .cast::<u32>()
                        };
                        ret.push(slot);
                    }
                    FrameValType::ContRef | FrameValType::FuncRef => {}
                    FrameValType::I32
                    | FrameValType::I64
                    | FrameValType::F32
                    | FrameValType::F64
                    | FrameValType::V128 => {}
                }
            }
        }
    }
    ret
}
