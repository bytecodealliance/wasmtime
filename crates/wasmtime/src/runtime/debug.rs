//! Debugging API.

use crate::{
    AnyRef, ExnRef, ExternRef, Func, Instance, Module, Val, ValType,
    store::{AutoAssertNoGc, StoreOpaque},
    vm::{Backtrace, VMContext},
};
use alloc::vec::Vec;
use core::{ffi::c_void, ops::ControlFlow, ptr::NonNull};
use wasmtime_environ::{
    DefinedFuncIndex, FrameInstPos, FrameStackShape, FrameStateSlot, FrameStateSlotOffset,
    FrameTableDescriptorIndex, FrameValType, FuncKey,
};
use wasmtime_unwinder::Frame;

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
        if !self.engine().tunables().debug_instrumentation {
            return None;
        }

        let mut frames = vec![];
        Backtrace::trace(self, |frame| {
            // `is_trapping_frame == false`: for now, we do not yet
            // support capturing stack values after a trap, so the PC
            // we use to look up metadata is always a "post-position"
            // PC, i.e., a call's return address.
            frames.extend(VirtualFrame::decode(self, frame, false));
            ControlFlow::Continue(())
        });
        Some(StackView {
            store: self,
            frames,
        })
    }
}

/// A view of values in active Wasm stack frames.
///
/// See the documentation on `Store::stack_value` for more information
/// about which frames this view will show.
pub struct StackView<'a> {
    /// Mutable borrow held to the store.
    ///
    /// This both ensures that the stack does not mutate while we're
    /// observing it (any borrow would do), and lets us create
    /// host-API GC references as values that are references are read
    /// off of the stack (a mutable borrow is needed for this).
    store: &'a mut StoreOpaque,

    /// Pre-enumerated frames. We precompute this rather than walking
    /// a true iterator finger up the stack (e.g., current FP and
    /// current `CallThreadState`) because our existing unwinder logic
    /// is written in a visit-with-closure style; and users of this
    /// API are likely to visit every frame anyway, so
    /// sparseness/efficiency is not a main concern here.
    frames: Vec<VirtualFrame>,
}

/// Internal data pre-computed for one stack frame.
///
/// This combines physical frame info (pc, fp) with the module this PC
/// maps to (yielding a frame table) and one frame as produced by the
/// progpoint lookup (Wasm PC, frame descriptor index, stack shape).
struct VirtualFrame {
    /// The frame pointer.
    fp: usize,
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

/// A view of a frame that can decode values in that frame.
pub struct FrameView<'a> {
    frame_state_slot: FrameStateSlot<'a>,
    store: &'a mut StoreOpaque,
    slot_addr: usize,
    wasm_pc: u32,
    stack: Vec<(FrameStateSlotOffset, FrameValType)>,
}

impl<'a> StackView<'a> {
    /// Get a handle to a specific frame.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of range.
    pub fn frame(&mut self, index: usize) -> FrameView<'_> {
        FrameView::new(self.store, &self.frames[index])
    }

    /// Get the number of frames viewable on this stack.
    pub fn len(&self) -> usize {
        self.frames.len()
    }
}

impl VirtualFrame {
    fn decode(store: &StoreOpaque, frame: Frame, is_trapping_frame: bool) -> Vec<VirtualFrame> {
        let module = store
            .modules()
            .lookup_module_by_pc(frame.pc())
            .expect("Wasm frame PC does not correspond to a module");
        let base = module.code_object().code_memory().text().as_ptr() as usize;
        let pc = frame.pc().wrapping_sub(base);
        let table = module.frame_table();
        let pc = u32::try_from(pc).expect("PC offset too large");
        let pos = if is_trapping_frame {
            FrameInstPos::Pre
        } else {
            FrameInstPos::Post
        };
        let Some(program_points) = table.find_program_point(pc, pos) else {
            return vec![];
        };

        let mut frames: Vec<_> = program_points
            .map(|(wasm_pc, frame_descriptor, stack_shape)| VirtualFrame {
                fp: frame.fp(),
                module: module.clone(),
                wasm_pc,
                frame_descriptor,
                stack_shape,
            })
            .collect();

        // Reverse the frames so we return them inside-out, matching
        // the bottom-up stack traversal order.
        frames.reverse();
        frames
    }
}

impl<'a> FrameView<'a> {
    fn new(store: &'a mut StoreOpaque, frame: &'a VirtualFrame) -> Self {
        let frame_table = frame.module.frame_table();
        // Parse the frame descriptor.
        let (data, slot_to_fp_offset) = frame_table
            .frame_descriptor(frame.frame_descriptor)
            .unwrap();
        let frame_state_slot = FrameStateSlot::parse(data).unwrap();
        let slot_addr = frame
            .fp
            .wrapping_sub(usize::try_from(slot_to_fp_offset).unwrap());
        // Materialize the stack shape so we have O(1) access to its elements.
        let mut stack = frame_state_slot
            .stack(frame.stack_shape)
            .collect::<Vec<_>>();
        stack.reverse(); // Put top-of-stack last.
        FrameView {
            store,
            frame_state_slot,
            slot_addr,
            wasm_pc: frame.wasm_pc,
            stack,
        }
    }

    fn raw_instance(&mut self) -> &'a crate::vm::Instance {
        // Read out the vmctx slot.
        // SAFETY: vmctx is always at offset 0 in the slot.
        let vmctx: *mut VMContext = unsafe { *(self.slot_addr as *mut _) };
        let vmctx = NonNull::new(vmctx).expect("null vmctx in debug state slot");
        // SAFETY: the stored vmctx value is a valid instance in this
        // store; we only visit frames from this store in teh backtrace.
        let instance = unsafe { crate::vm::Instance::from_vmctx(vmctx) };
        // SAFETY: the instance pointer read above is valid.
        unsafe { instance.as_ref() }
    }

    /// Get the instance associated with this frame.
    pub fn instance(&mut self) -> Instance {
        let instance = self.raw_instance();
        Instance::from_wasmtime(instance.id(), self.store)
    }

    /// Get the module associated with this frame, if any (i.e., not a
    /// container instance for a host-created entity).
    pub fn module(&mut self) -> Option<&Module> {
        let instance = self.raw_instance();
        instance.runtime_module()
    }

    /// Get the raw function index associated with this frame, and the
    /// PC as an offset within its code section, if it is a Wasm
    /// function directly from the given `Module` (rather than a
    /// trampoline).
    pub fn wasm_function_index_and_pc(&mut self) -> Option<(DefinedFuncIndex, u32)> {
        let FuncKey::DefinedWasmFunction(module, func) = self.frame_state_slot.func_key() else {
            return None;
        };
        debug_assert_eq!(
            module,
            self.module()
                .expect("module should be defined if this is a defined function")
                .env_module()
                .module_index
        );
        Some((func, self.wasm_pc))
    }

    /// Get the number of locals in this frame.
    pub fn num_locals(&self) -> usize {
        self.frame_state_slot.num_locals()
    }

    /// Get the depth of the operand stack in this frame.
    pub fn num_stacks(&self) -> usize {
        self.stack.len()
    }

    /// Get the type and value of the given local in this frame.
    ///
    /// # Panics
    ///
    /// Panics if the index is out-of-range (greater than
    /// `num_locals()`).
    pub fn local(&mut self, index: usize) -> (ValType, Val) {
        let (offset, ty) = self.frame_state_slot.local(index).unwrap();
        // SAFETY: compiler produced metadata to describe this local
        // slot and stored a value of the correct type into it.
        unsafe { read_value(self.store, self.slot_addr, offset, ty) }
    }

    /// Get the type and value of the given operand-stack value in
    /// this frame.
    ///
    /// Index 0 corresponds to the bottom-of-stack, and higher indices
    /// from there are more recently pushed values.  In other words,
    /// index order reads the Wasm virtual machine's abstract stack
    /// state left-to-right.
    pub fn stack(&mut self, index: usize) -> (ValType, Val) {
        let (offset, ty) = self.stack[index];
        // SAFETY: compiler produced metadata to describe this
        // operand-stack slot and stored a value of the correct type
        // into it.
        unsafe { read_value(self.store, self.slot_addr, offset, ty) }
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
    store: &mut StoreOpaque,
    slot_base: usize,
    offset: FrameStateSlotOffset,
    ty: FrameValType,
) -> (ValType, Val) {
    let address = slot_base.wrapping_add(usize::try_from(offset.offset()).unwrap());

    // SAFETY: each case reads a value from memory that should be
    // valid according to our safety condition.
    match ty {
        FrameValType::I32 => {
            let value = unsafe { *(address as *const i32) };
            (ValType::I32, Val::I32(value))
        }
        FrameValType::I64 => {
            let value = unsafe { *(address as *const i64) };
            (ValType::I64, Val::I64(value))
        }
        FrameValType::F32 => {
            let value = unsafe { *(address as *const u32) };
            (ValType::F32, Val::F32(value))
        }
        FrameValType::F64 => {
            let value = unsafe { *(address as *const u64) };
            (ValType::F64, Val::F64(value))
        }
        FrameValType::V128 => {
            let value = unsafe { *(address as *const u128) };
            (ValType::V128, Val::V128(value.into()))
        }
        FrameValType::AnyRef => {
            let mut nogc = AutoAssertNoGc::new(store);
            let value = unsafe { *(address as *const u32) };
            let value = AnyRef::_from_raw(&mut nogc, value);
            (ValType::ANYREF, Val::AnyRef(value))
        }
        FrameValType::ExnRef => {
            let mut nogc = AutoAssertNoGc::new(store);
            let value = unsafe { *(address as *const u32) };
            let value = ExnRef::_from_raw(&mut nogc, value);
            (ValType::EXNREF, Val::ExnRef(value))
        }
        FrameValType::ExternRef => {
            let mut nogc = AutoAssertNoGc::new(store);
            let value = unsafe { *(address as *const u32) };
            let value = ExternRef::_from_raw(&mut nogc, value);
            (ValType::EXTERNREF, Val::ExternRef(value))
        }
        FrameValType::FuncRef => {
            let value = unsafe { *(address as *const *mut c_void) };
            let value = unsafe { Func::_from_raw(store, value) };
            (ValType::EXTERNREF, Val::FuncRef(value))
        }
        FrameValType::ContRef => {
            unimplemented!("contref values are not implemented in the host API yet")
        }
    }
}
