//! Debugging API.

use crate::{
    AnyRef, AsContext, AsContextMut, CodeMemory, ExnRef, ExternRef, Func, Instance, Module,
    OwnedRooted, StoreContext, StoreContextMut, Val,
    code::StoreCodePC,
    module::ModuleRegistry,
    store::{AutoAssertNoGc, StoreOpaque},
    vm::{CompiledModuleId, CurrentActivationBacktrace, VMContext},
};
use alloc::collections::BTreeSet;
use alloc::vec;
use alloc::vec::Vec;
use anyhow::Result;
use core::{ffi::c_void, ptr::NonNull};
#[cfg(feature = "gc")]
use wasmtime_environ::FrameTable;
use wasmtime_environ::{
    DefinedFuncIndex, FrameInstPos, FrameStackShape, FrameStateSlot, FrameStateSlotOffset,
    FrameTableBreakpointData, FrameTableDescriptorIndex, FrameValType, FuncKey, Trap,
};
use wasmtime_unwinder::Frame;

use super::store::AsStoreOpaque;

impl<'a, T> StoreContextMut<'a, T> {
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
    pub fn debug_frames(self) -> Option<DebugFrameCursor<'a, T>> {
        if !self.engine().tunables().debug_guest {
            return None;
        }

        // SAFETY: This takes a mutable borrow of `self` (the
        // `StoreOpaque`), which owns all active stacks in the
        // store. We do not provide any API that could mutate the
        // frames that we are walking on the `DebugFrameCursor`.
        let iter = unsafe { CurrentActivationBacktrace::new(self) };
        let mut view = DebugFrameCursor {
            iter,
            is_trapping_frame: false,
            frames: vec![],
            current: None,
        };
        view.move_to_parent(); // Load the first frame.
        Some(view)
    }

    /// Start an edit session to update breakpoints.
    pub fn edit_breakpoints(self) -> Option<BreakpointEdit<'a>> {
        if !self.engine().tunables().debug_guest {
            return None;
        }

        let (breakpoints, registry) = self.0.breakpoints_and_registry_mut();
        Some(breakpoints.edit(registry))
    }
}

impl<'a, T> StoreContext<'a, T> {
    /// Return all breakpoints.
    pub fn breakpoints(self) -> Option<impl Iterator<Item = Breakpoint> + 'a> {
        if !self.engine().tunables().debug_guest {
            return None;
        }

        let (breakpoints, registry) = self.0.breakpoints_and_registry();
        Some(breakpoints.breakpoints(registry))
    }

    /// Indicate whether single-step mode is enabled.
    pub fn is_single_step(&self) -> bool {
        let (breakpoints, _) = self.0.breakpoints_and_registry();
        breakpoints.is_single_step()
    }
}

/// A view of an active stack frame, with the ability to move up the
/// stack.
///
/// See the documentation on `Store::stack_value` for more information
/// about which frames this view will show.
pub struct DebugFrameCursor<'a, T: 'static> {
    /// Iterator over frames.
    ///
    /// This iterator owns the store while the view exists (accessible
    /// as `iter.store`).
    iter: CurrentActivationBacktrace<'a, T>,

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

impl<'a, T: 'static> DebugFrameCursor<'a, T> {
    /// Move up to the next frame in the activation.
    pub fn move_to_parent(&mut self) {
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
            self.frames = VirtualFrame::decode(
                self.iter.store.0.as_store_opaque(),
                next_frame,
                self.is_trapping_frame,
            );
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
        Instance::from_wasmtime(instance.id(), self.iter.store.0.as_store_opaque())
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
        unsafe { read_value(&mut self.iter.store.0, slot_addr, offset, ty) }
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
        unsafe { read_value(&mut self.iter.store.0, slot_addr, offset, ty) }
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
    fn decode(store: &mut StoreOpaque, frame: Frame, is_trapping_frame: bool) -> Vec<VirtualFrame> {
        let (module_with_code, pc) = store
            .modules()
            .module_and_code_by_pc(frame.pc())
            .expect("Wasm frame PC does not correspond to a module");
        let module = module_with_code.module();
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
    store: &mut StoreOpaque,
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
#[cfg(feature = "gc")]
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

impl<'a, T: 'static> AsContext for DebugFrameCursor<'a, T> {
    type Data = T;
    fn as_context(&self) -> StoreContext<'_, Self::Data> {
        StoreContext(self.iter.store.0)
    }
}
impl<'a, T: 'static> AsContextMut for DebugFrameCursor<'a, T> {
    fn as_context_mut(&mut self) -> StoreContextMut<'_, Self::Data> {
        StoreContextMut(self.iter.store.0)
    }
}

/// One debug event that occurs when running Wasm code on a store with
/// a debug handler attached.
#[derive(Debug)]
pub enum DebugEvent<'a> {
    /// An `anyhow::Error` was raised by a hostcall.
    HostcallError(&'a anyhow::Error),
    /// An exception is thrown and caught by Wasm. The current state
    /// is at the throw-point.
    CaughtExceptionThrown(OwnedRooted<ExnRef>),
    /// An exception was not caught and is escaping to the host.
    UncaughtExceptionThrown(OwnedRooted<ExnRef>),
    /// A Wasm trap occurred.
    Trap(Trap),
    /// A breakpoint was reached.
    Breakpoint,
}

/// A handler for debug events.
///
/// This is an async callback that is invoked directly within the
/// context of a debug event that occurs, i.e., with the Wasm code
/// still on the stack. The callback can thus observe that stack, up
/// to the most recent entry to Wasm.[^1]
///
/// Because this callback receives a `StoreContextMut`, it has full
/// access to any state that any other hostcall has, including the
/// `T`. In that way, it is like an epoch-deadline callback or a
/// call-hook callback. It also "freezes" the entire store for the
/// duration of the debugger callback future.
///
/// In the future, we expect to provide an "externally async" API on
/// the `Store` that allows receiving a stream of debug events and
/// accessing the store mutably while frozen; that will need to
/// integrate with [`Store::run_concurrent`] to properly timeslice and
/// scope the mutable access to the store, and has not been built
/// yet. In the meantime, it should be possible to build a fully
/// functional debugger with this async-callback API by channeling
/// debug events out, and requests to read the store back in, over
/// message-passing channels between the callback and an external
/// debugger main loop.
///
/// Note that the `handle` hook may use its mutable store access to
/// invoke another Wasm. Debug events will also be caught and will
/// cause further `handle` invocations during this recursive
/// invocation. It is up to the debugger to handle any implications of
/// this reentrancy (e.g., implications on a duplex channel protocol
/// with an event/continue handshake) if it does so.
///
/// Note also that this trait has `Clone` as a supertrait, and the
/// handler is cloned at every invocation as an artifact of the
/// internal ownership structure of Wasmtime: the handler itself is
/// owned by the store, but also receives a mutable borrow to the
/// whole store, so we need to clone it out to invoke it. It is
/// recommended that this trait be implemented by a type that is cheap
/// to clone: for example, a single `Arc` handle to debugger state.
///
/// [^1]: Providing visibility further than the most recent entry to
///       Wasm is not directly possible because it could see into
///       another async stack, and the stack that polls the future
///       running a particular Wasm invocation could change after each
///       suspend point in the handler.
pub trait DebugHandler: Clone + Send + Sync + 'static {
    /// The data expected on the store that this handler is attached
    /// to.
    type Data;

    /// Handle a debug event.
    fn handle(
        &self,
        store: StoreContextMut<'_, Self::Data>,
        event: DebugEvent<'_>,
    ) -> impl Future<Output = ()> + Send;
}

/// Breakpoint state for modules within a store.
#[derive(Default)]
pub(crate) struct BreakpointState {
    /// Single-step mode.
    single_step: bool,
    /// Breakpoints added individually.
    breakpoints: BTreeSet<BreakpointKey>,
}

/// A breakpoint.
pub struct Breakpoint {
    /// Reference to the module in which we are setting the breakpoint.
    pub module: Module,
    /// Wasm PC offset within the module.
    pub pc: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct BreakpointKey(CompiledModuleId, u32);

impl BreakpointKey {
    fn from_raw(module: &Module, pc: u32) -> BreakpointKey {
        BreakpointKey(module.id(), pc)
    }

    fn get(&self, registry: &ModuleRegistry) -> Breakpoint {
        let module = registry
            .module_by_compiled_id(self.0)
            .expect("Module should not have been removed from Store")
            .clone();
        Breakpoint { module, pc: self.1 }
    }
}

/// A breakpoint-editing session.
///
/// This enables updating breakpoint state (setting or unsetting
/// individual breakpoints or the store-global single-step flag) in a
/// batch. It is more efficient to batch these updates because
/// "re-publishing" the newly patched code, with update breakpoint
/// settings, typically requires a syscall to re-enable execute
/// permissions.
pub struct BreakpointEdit<'a> {
    state: &'a mut BreakpointState,
    registry: &'a mut ModuleRegistry,
    /// Modules that have been edited.
    ///
    /// Invariant: each of these modules' CodeMemory objects is
    /// *unpublished* when in the dirty set.
    dirty_modules: BTreeSet<StoreCodePC>,
}

impl BreakpointState {
    pub(crate) fn edit<'a>(&'a mut self, registry: &'a mut ModuleRegistry) -> BreakpointEdit<'a> {
        BreakpointEdit {
            state: self,
            registry,
            dirty_modules: BTreeSet::new(),
        }
    }

    pub(crate) fn breakpoints<'a>(
        &'a self,
        registry: &'a ModuleRegistry,
    ) -> impl Iterator<Item = Breakpoint> + 'a {
        self.breakpoints.iter().map(|key| key.get(registry))
    }

    pub(crate) fn is_single_step(&self) -> bool {
        self.single_step
    }
}

impl<'a> BreakpointEdit<'a> {
    fn get_code_memory<'b>(
        registry: &'b mut ModuleRegistry,
        dirty_modules: &mut BTreeSet<StoreCodePC>,
        module: &Module,
    ) -> Result<&'b mut CodeMemory> {
        let store_code_pc = registry.store_code_base_or_register(module)?;
        let code_memory = registry
            .store_code_mut(store_code_pc)
            .expect("Just checked presence above")
            .code_memory_mut()
            .expect("Must have unique ownership of StoreCode in guest-debug mode");
        if dirty_modules.insert(store_code_pc) {
            // SAFETY: We have a mutable borrow to the `registry`,
            // which is part of the `Store`. Code in this store cannot
            // run while we hold that borrow. We re-publish when we
            // are dropped.
            unsafe {
                code_memory.unpublish()?;
            }
        }
        Ok(code_memory)
    }

    fn patch<'b>(
        patches: impl Iterator<Item = FrameTableBreakpointData<'b>> + 'b,
        mem: &mut CodeMemory,
        enable: bool,
    ) {
        let mem = mem.text_mut();
        for patch in patches {
            let data = if enable { patch.enable } else { patch.disable };
            let mem = &mut mem[patch.offset..patch.offset + data.len()];
            log::trace!(
                "patch: offset 0x{:x} with enable={enable}: data {data:?} replacing {mem:?}",
                patch.offset
            );
            mem.copy_from_slice(data);
        }
    }

    /// Add a breakpoint in the given module at the given PC in that
    /// module.
    ///
    /// No effect if the breakpoint is already set.
    pub fn add_breakpoint(&mut self, module: &Module, pc: u32) -> Result<()> {
        let key = BreakpointKey::from_raw(module, pc);
        self.state.breakpoints.insert(key);
        log::trace!("patching in breakpoint {key:?}");
        let mem = Self::get_code_memory(self.registry, &mut self.dirty_modules, module)?;
        let frame_table = module
            .frame_table()
            .expect("Frame table must be present when guest-debug is enabled");
        let patches = frame_table.lookup_breakpoint_patches_by_pc(pc);
        Self::patch(patches, mem, true);
        Ok(())
    }

    /// Remove a breakpoint in the given module at the given PC in
    /// that module.
    ///
    /// No effect if the breakpoint was not set.
    pub fn remove_breakpoint(&mut self, module: &Module, pc: u32) -> Result<()> {
        let key = BreakpointKey::from_raw(module, pc);
        self.state.breakpoints.remove(&key);
        if !self.state.single_step {
            let mem = Self::get_code_memory(self.registry, &mut self.dirty_modules, module)?;
            let frame_table = module
                .frame_table()
                .expect("Frame table must be present when guest-debug is enabled");
            let patches = frame_table.lookup_breakpoint_patches_by_pc(pc);
            Self::patch(patches, mem, false);
        }
        Ok(())
    }

    /// Turn on or off single-step mode.
    ///
    /// In single-step mode, a breakpoint event is emitted at every
    /// Wasm PC.
    pub fn single_step(&mut self, enabled: bool) -> Result<()> {
        log::trace!(
            "single_step({enabled}) with breakpoint set {:?}",
            self.state.breakpoints
        );
        let modules = self.registry.all_modules().cloned().collect::<Vec<_>>();
        for module in modules {
            let mem = Self::get_code_memory(self.registry, &mut self.dirty_modules, &module)?;
            let table = module
                .frame_table()
                .expect("Frame table must be present when guest-debug is enabled");
            for (wasm_pc, patch) in table.breakpoint_patches() {
                let key = BreakpointKey::from_raw(&module, wasm_pc);
                let this_enabled = enabled || self.state.breakpoints.contains(&key);
                log::trace!(
                    "single_step: enabled {enabled} key {key:?} -> this_enabled {this_enabled}"
                );
                Self::patch(core::iter::once(patch), mem, this_enabled);
            }
        }

        self.state.single_step = enabled;

        Ok(())
    }
}

impl<'a> Drop for BreakpointEdit<'a> {
    fn drop(&mut self) {
        for &store_code_base in &self.dirty_modules {
            let store_code = self.registry.store_code_mut(store_code_base).unwrap();
            store_code
                .code_memory_mut()
                .expect("Must have unique ownership of StoreCode in guest-debug mode")
                .publish()
                .expect("re-publish failed");
        }
    }
}
