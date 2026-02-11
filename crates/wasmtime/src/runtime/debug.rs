//! Debugging API.

use super::store::AsStoreOpaque;
use crate::store::StoreId;
use crate::vm::{Activation, Backtrace};
use crate::{
    AnyRef, AsContextMut, CodeMemory, ExnRef, Extern, ExternRef, Func, Instance, Module,
    OwnedRooted, StoreContext, StoreContextMut, Val,
    code::StoreCodePC,
    module::ModuleRegistry,
    store::{AutoAssertNoGc, StoreOpaque},
    vm::{CompiledModuleId, VMContext},
};
use crate::{Caller, Result, Store};
use alloc::collections::{BTreeMap, BTreeSet, btree_map::Entry};
use alloc::vec;
use alloc::vec::Vec;
use core::{ffi::c_void, ptr::NonNull};
#[cfg(feature = "gc")]
use wasmtime_environ::FrameTable;
use wasmtime_environ::{
    DefinedFuncIndex, EntityIndex, FrameInstPos, FrameStackShape, FrameStateSlot,
    FrameStateSlotOffset, FrameTableBreakpointData, FrameTableDescriptorIndex, FrameValType,
    FuncIndex, FuncKey, GlobalIndex, MemoryIndex, TableIndex, TagIndex, Trap,
};
use wasmtime_unwinder::{Frame, FrameCursor, frame_cursor};

impl<T> Store<T> {
    /// Provide a frame handle for all activations, in order from
    /// innermost (most recently called) to outermost on the stack.
    ///
    /// An activation is a contiguous sequence of Wasm frames (called
    /// functions) that were called from host code and called back out
    /// to host code. If there are activations from multiple stores on
    /// the stack, for example if Wasm code in one store calls out to
    /// host code which invokes another Wasm function in another
    /// store, then the other stores are "opaque" to our view here in
    /// the same way that host code is.
    ///
    /// Returns an empty list if debug instrumentation is not enabled
    /// for the engine containing this store.
    pub fn debug_exit_frames(&mut self) -> impl Iterator<Item = FrameHandle> {
        self.as_store_opaque().debug_exit_frames()
    }

    /// Start an edit session to update breakpoints.
    pub fn edit_breakpoints<'a>(&'a mut self) -> Option<BreakpointEdit<'a>> {
        self.as_store_opaque().edit_breakpoints()
    }
}

impl StoreOpaque {
    fn debug_exit_frames(&mut self) -> impl Iterator<Item = FrameHandle> {
        let activations = if self.engine().tunables().debug_guest {
            Backtrace::activations(self)
        } else {
            vec![]
        };

        activations
            .into_iter()
            // SAFETY: each activation is currently active and will
            // remain so (we have a mutable borrow of the store).
            .filter_map(|act| unsafe { FrameHandle::exit_frame(self, act) })
    }

    fn edit_breakpoints<'a>(&'a mut self) -> Option<BreakpointEdit<'a>> {
        if !self.engine().tunables().debug_guest {
            return None;
        }

        let (breakpoints, registry) = self.breakpoints_and_registry_mut();
        Some(breakpoints.edit(registry))
    }
}

impl<'a, T> StoreContextMut<'a, T> {
    /// Provide a frame handle for all activations, in order from
    /// innermost (most recently called) to outermost on the stack.
    ///
    /// See [`Store::debug_exit_frames`] for more details.
    pub fn debug_exit_frames(&mut self) -> impl Iterator<Item = FrameHandle> {
        self.0.as_store_opaque().debug_exit_frames()
    }

    /// Start an edit session to update breakpoints.
    pub fn edit_breakpoints(self) -> Option<BreakpointEdit<'a>> {
        self.0.as_store_opaque().edit_breakpoints()
    }
}

impl<'a, T> Caller<'a, T> {
    /// Provide a frame handle for all activations, in order from
    /// innermost (most recently called) to outermost on the stack.
    ///
    /// See [`Store::debug_exit_frames`] for more details.
    pub fn debug_exit_frames(&mut self) -> impl Iterator<Item = FrameHandle> {
        self.store.0.as_store_opaque().debug_exit_frames()
    }
}

impl Instance {
    /// Get access to a global within this instance's globals index
    /// space.
    ///
    /// This permits accessing globals whether they are exported or
    /// not. However, it is only available for purposes of debugging,
    /// and so is only permitted when `guest_debug` is enabled in the
    /// Engine's configuration. The intent of the Wasmtime API is to
    /// enforce the Wasm type system's encapsulation even in the host
    /// API, except where necessary for developer tooling.
    ///
    /// `None` is returned for any global index that is out-of-bounds.
    ///
    /// `None` is returned if guest-debugging is not enabled in the
    /// engine configuration for this Store.
    pub fn debug_global(
        &self,
        mut store: impl AsContextMut,
        global_index: u32,
    ) -> Option<crate::Global> {
        self.debug_export(
            store.as_context_mut().0,
            GlobalIndex::from_bits(global_index).into(),
        )
        .and_then(|s| s.into_global())
    }

    /// Get access to a memory (unshared only) within this instance's
    /// memory index space.
    ///
    /// This permits accessing memories whether they are exported or
    /// not. However, it is only available for purposes of debugging,
    /// and so is only permitted when `guest_debug` is enabled in the
    /// Engine's configuration. The intent of the Wasmtime API is to
    /// enforce the Wasm type system's encapsulation even in the host
    /// API, except where necessary for developer tooling.
    ///
    /// `None` is returned for any memory index that is out-of-bounds.
    ///
    /// `None` is returned for any shared memory (use
    /// `debug_shared_memory` instead).
    ///
    /// `None` is returned if guest-debugging is not enabled in the
    /// engine configuration for this Store.
    pub fn debug_memory(
        &self,
        mut store: impl AsContextMut,
        memory_index: u32,
    ) -> Option<crate::Memory> {
        self.debug_export(
            store.as_context_mut().0,
            MemoryIndex::from_bits(memory_index).into(),
        )
        .and_then(|s| s.into_memory())
    }

    /// Get access to a shared memory within this instance's memory
    /// index space.
    ///
    /// This permits accessing memories whether they are exported or
    /// not. However, it is only available for purposes of debugging,
    /// and so is only permitted when `guest_debug` is enabled in the
    /// Engine's configuration. The intent of the Wasmtime API is to
    /// enforce the Wasm type system's encapsulation even in the host
    /// API, except where necessary for developer tooling.
    ///
    /// `None` is returned for any memory index that is out-of-bounds.
    ///
    /// `None` is returned for any unshared memory (use `debug_memory`
    /// instead).
    ///
    /// `None` is returned if guest-debugging is not enabled in the
    /// engine configuration for this Store.
    pub fn debug_shared_memory(
        &self,
        mut store: impl AsContextMut,
        memory_index: u32,
    ) -> Option<crate::SharedMemory> {
        self.debug_export(
            store.as_context_mut().0,
            MemoryIndex::from_bits(memory_index).into(),
        )
        .and_then(|s| s.into_shared_memory())
    }

    /// Get access to a table within this instance's table index
    /// space.
    ///
    /// This permits accessing tables whether they are exported or
    /// not. However, it is only available for purposes of debugging,
    /// and so is only permitted when `guest_debug` is enabled in the
    /// Engine's configuration. The intent of the Wasmtime API is to
    /// enforce the Wasm type system's encapsulation even in the host
    /// API, except where necessary for developer tooling.
    ///
    /// `None` is returned for any table index that is out-of-bounds.
    ///
    /// `None` is returned if guest-debugging is not enabled in the
    /// engine configuration for this Store.
    pub fn debug_table(
        &self,
        mut store: impl AsContextMut,
        table_index: u32,
    ) -> Option<crate::Table> {
        self.debug_export(
            store.as_context_mut().0,
            TableIndex::from_bits(table_index).into(),
        )
        .and_then(|s| s.into_table())
    }

    /// Get access to a function within this instance's function index
    /// space.
    ///
    /// This permits accessing functions whether they are exported or
    /// not. However, it is only available for purposes of debugging,
    /// and so is only permitted when `guest_debug` is enabled in the
    /// Engine's configuration. The intent of the Wasmtime API is to
    /// enforce the Wasm type system's encapsulation even in the host
    /// API, except where necessary for developer tooling.
    ///
    /// `None` is returned for any function index that is
    /// out-of-bounds.
    ///
    /// `None` is returned if guest-debugging is not enabled in the
    /// engine configuration for this Store.
    pub fn debug_function(
        &self,
        mut store: impl AsContextMut,
        function_index: u32,
    ) -> Option<crate::Func> {
        self.debug_export(
            store.as_context_mut().0,
            FuncIndex::from_bits(function_index).into(),
        )
        .and_then(|s| s.into_func())
    }

    /// Get access to a tag within this instance's tag index space.
    ///
    /// This permits accessing tags whether they are exported or
    /// not. However, it is only available for purposes of debugging,
    /// and so is only permitted when `guest_debug` is enabled in the
    /// Engine's configuration. The intent of the Wasmtime API is to
    /// enforce the Wasm type system's encapsulation even in the host
    /// API, except where necessary for developer tooling.
    ///
    /// `None` is returned for any tag index that is out-of-bounds.
    ///
    /// `None` is returned if guest-debugging is not enabled in the
    /// engine configuration for this Store.
    pub fn debug_tag(&self, mut store: impl AsContextMut, tag_index: u32) -> Option<crate::Tag> {
        self.debug_export(
            store.as_context_mut().0,
            TagIndex::from_bits(tag_index).into(),
        )
        .and_then(|s| s.into_tag())
    }

    fn debug_export(&self, store: &mut StoreOpaque, index: EntityIndex) -> Option<Extern> {
        if !store.engine().tunables().debug_guest {
            return None;
        }

        let env_module = self._module(store).env_module();
        if !env_module.is_valid(index) {
            return None;
        }
        let store_id = store.id();
        let (instance, registry) = store.instance_and_module_registry_mut(self.id());
        // SAFETY: the `store` and `registry` are associated with
        // this instance as we fetched the instance directly from
        // the store above.
        let export = unsafe { instance.get_export_by_index_mut(registry, store_id, index) };
        Some(Extern::from_wasmtime_export(export, store))
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

/// A handle to a stack frame, valid as long as execution is not
/// resumed in the associated `Store`.
///
/// This handle can be held and cloned and used to refer to a frame
/// within a paused store. It is cheap: it internally consists of a
/// pointer to the actual frame, together with some metadata to
/// determine when that pointer has gone stale.
///
/// At the API level, any usage of this frame handle requires a
/// mutable borrow of the `Store`, because the `Store` logically owns
/// the stack(s) for any execution within it. However, the existence
/// of the handle itself does not hold a borrow on the `Store`; hence,
/// the `Store` can continue to be used and queried, and some state
/// (e.g. memories, tables, GC objects) can even be mutated, as long
/// as execution is not resumed. The intent of this API is to allow a
/// wide variety of debugger implementation strategies that expose
/// stack frames and also allow other commands/actions at the same
/// time.
///
/// The user can use [`FrameHandle::is_valid`] to determine if the
/// handle is still valid and usable.
#[derive(Clone)]
pub struct FrameHandle {
    /// The unwinder cursor at this frame.
    cursor: FrameCursor,

    /// The index of the virtual frame within the physical frame.
    virtual_frame_idx: usize,

    /// The unique Store this frame came from, to ensure the handle is
    /// used with the correct Store.
    store_id: StoreId,

    /// Store `execution_version`.
    store_version: u64,
}

impl FrameHandle {
    /// Create a new FrameHandle at the exit frame of an activation.
    ///
    /// # Safety
    ///
    /// The provided activation must be valid currently.
    unsafe fn exit_frame(store: &mut StoreOpaque, activation: Activation) -> Option<FrameHandle> {
        // SAFETY: activation is valid as per our safety condition.
        let mut cursor = unsafe {
            frame_cursor(
                activation.exit_pc,
                activation.exit_fp,
                activation.entry_trampoline_fp,
            )
        };

        // Find the first virtual frame. Each physical frame may have
        // zero or more virtual frames.
        while !cursor.done() {
            let (cache, registry) = store.frame_data_cache_mut_and_registry();
            let frames = cache.lookup_or_compute(registry, cursor.frame());
            if frames.len() > 0 {
                return Some(FrameHandle {
                    cursor,
                    virtual_frame_idx: 0,
                    store_id: store.id(),
                    store_version: store.vm_store_context().execution_version,
                });
            }
            // SAFETY: activation is still valid (we have not returned
            // control since above).
            unsafe {
                cursor.advance(store.unwinder());
            }
        }

        None
    }

    /// Determine whether this handle can still be used to refer to a
    /// frame.
    pub fn is_valid(&self, mut store: impl AsContextMut) -> bool {
        let store = store.as_context_mut();
        self.is_valid_impl(store.0.as_store_opaque())
    }

    fn is_valid_impl(&self, store: &StoreOpaque) -> bool {
        let id = store.id();
        let version = store.vm_store_context().execution_version;
        self.store_id == id && self.store_version == version
    }

    /// Get a handle to the next frame up the activation (the one that
    /// called this frame), if any.
    pub fn parent(&self, mut store: impl AsContextMut) -> Result<Option<FrameHandle>> {
        let mut store = store.as_context_mut();
        if !self.is_valid(&mut store) {
            crate::error::bail!("Frame handle is no longer valid.");
        }

        let mut parent = self.clone();
        parent.virtual_frame_idx += 1;

        while !parent.cursor.done() {
            let (cache, registry) = store
                .0
                .as_store_opaque()
                .frame_data_cache_mut_and_registry();
            let frames = cache.lookup_or_compute(registry, parent.cursor.frame());
            if parent.virtual_frame_idx < frames.len() {
                return Ok(Some(parent));
            }
            parent.virtual_frame_idx = 0;
            // SAFETY: activation is valid because we checked validity
            // wrt execution version at the top of this function, and
            // we have not returned since.
            unsafe {
                parent.cursor.advance(store.0.as_store_opaque().unwinder());
            }
        }

        Ok(None)
    }

    fn frame_data<'a>(&self, store: &'a mut StoreOpaque) -> Result<&'a FrameData> {
        if !self.is_valid_impl(store) {
            crate::error::bail!("Frame handle is no longer valid.");
        }
        let (cache, registry) = store.frame_data_cache_mut_and_registry();
        let frames = cache.lookup_or_compute(registry, self.cursor.frame());
        // `virtual_frame_idx` counts up for ease of iteration
        // behavior, while the frames are stored in outer-to-inner
        // (i.e., caller to callee) order, so we need to reverse here.
        Ok(&frames[frames.len() - 1 - self.virtual_frame_idx])
    }

    fn raw_instance<'a>(&self, store: &mut StoreOpaque) -> Result<&'a crate::vm::Instance> {
        let frame_data = self.frame_data(store)?;

        // Read out the vmctx slot.

        // SAFETY: vmctx is always at offset 0 in the slot.  (See
        // crates/cranelift/src/func_environ.rs in
        // `update_stack_slot_vmctx()`.)  The frame/activation is
        // still valid because we verified this in `frame_data` above.
        let vmctx: *mut VMContext =
            unsafe { *(frame_data.slot_addr(self.cursor.frame().fp()) as *mut _) };
        let vmctx = NonNull::new(vmctx).expect("null vmctx in debug state slot");
        // SAFETY: the stored vmctx value is a valid instance in this
        // store; we only visit frames from this store in the
        // backtrace.
        let instance = unsafe { crate::vm::Instance::from_vmctx(vmctx) };
        // SAFETY: the instance pointer read above is valid.
        Ok(unsafe { instance.as_ref() })
    }

    /// Get the instance associated with the current frame.
    pub fn instance(&self, mut store: impl AsContextMut) -> Result<Instance> {
        let store = store.as_context_mut();
        let instance = self.raw_instance(store.0.as_store_opaque())?;
        let id = instance.id();
        Ok(Instance::from_wasmtime(id, store.0.as_store_opaque()))
    }

    /// Get the module associated with the current frame, if any
    /// (i.e., not a container instance for a host-created entity).
    pub fn module<'a, T: 'static>(
        &self,
        store: impl Into<StoreContextMut<'a, T>>,
    ) -> Result<Option<&'a Module>> {
        let store = store.into();
        let instance = self.raw_instance(store.0.as_store_opaque())?;
        Ok(instance.runtime_module())
    }

    /// Get the raw function index associated with the current frame, and the
    /// PC as an offset within its code section, if it is a Wasm
    /// function directly from the given `Module` (rather than a
    /// trampoline).
    pub fn wasm_function_index_and_pc(
        &self,
        mut store: impl AsContextMut,
    ) -> Result<Option<(DefinedFuncIndex, u32)>> {
        let mut store = store.as_context_mut();
        let frame_data = self.frame_data(store.0.as_store_opaque())?;
        let FuncKey::DefinedWasmFunction(module, func) = frame_data.func_key else {
            return Ok(None);
        };
        let wasm_pc = frame_data.wasm_pc;
        debug_assert_eq!(
            module,
            self.module(&mut store)?
                .expect("module should be defined if this is a defined function")
                .env_module()
                .module_index
        );
        Ok(Some((func, wasm_pc)))
    }

    /// Get the number of locals in this frame.
    pub fn num_locals(&self, mut store: impl AsContextMut) -> Result<u32> {
        let store = store.as_context_mut();
        let frame_data = self.frame_data(store.0.as_store_opaque())?;
        Ok(u32::try_from(frame_data.locals.len()).unwrap())
    }

    /// Get the depth of the operand stack in this frame.
    pub fn num_stacks(&self, mut store: impl AsContextMut) -> Result<u32> {
        let store = store.as_context_mut();
        let frame_data = self.frame_data(store.0.as_store_opaque())?;
        Ok(u32::try_from(frame_data.stack.len()).unwrap())
    }

    /// Get the type and value of the given local in this frame.
    ///
    /// # Panics
    ///
    /// Panics if the index is out-of-range (greater than
    /// `num_locals()`).
    pub fn local(&self, mut store: impl AsContextMut, index: u32) -> Result<Val> {
        let store = store.as_context_mut();
        let frame_data = self.frame_data(store.0.as_store_opaque())?;
        let (offset, ty) = frame_data.locals[usize::try_from(index).unwrap()];
        let slot_addr = frame_data.slot_addr(self.cursor.frame().fp());
        // SAFETY: compiler produced metadata to describe this local
        // slot and stored a value of the correct type into it. Slot
        // address is valid because we checked liveness of the
        // activation/frame via `frame_data` above.
        Ok(unsafe { read_value(store.0.as_store_opaque(), slot_addr, offset, ty) })
    }

    /// Get the type and value of the given operand-stack value in
    /// this frame.
    ///
    /// Index 0 corresponds to the bottom-of-stack, and higher indices
    /// from there are more recently pushed values.  In other words,
    /// index order reads the Wasm virtual machine's abstract stack
    /// state left-to-right.
    pub fn stack(&self, mut store: impl AsContextMut, index: u32) -> Result<Val> {
        let store = store.as_context_mut();
        let frame_data = self.frame_data(store.0.as_store_opaque())?;
        let (offset, ty) = frame_data.stack[usize::try_from(index).unwrap()];
        let slot_addr = frame_data.slot_addr(self.cursor.frame().fp());
        // SAFETY: compiler produced metadata to describe this
        // operand-stack slot and stored a value of the correct type
        // into it. Slot address is valid because we checked liveness
        // of the activation/frame via `frame_data` above.
        Ok(unsafe { read_value(store.0.as_store_opaque(), slot_addr, offset, ty) })
    }
}

/// A cache from `StoreCodePC`s for modules' private code within a
/// store to pre-computed layout data for the virtual stack frame(s)
/// present at that physical PC.
pub(crate) struct FrameDataCache {
    /// For a given physical PC, the list of virtual frames, from
    /// inner (most recently called/inlined) to outer.
    by_pc: BTreeMap<StoreCodePC, Vec<FrameData>>,
}

impl FrameDataCache {
    pub(crate) fn new() -> FrameDataCache {
        FrameDataCache {
            by_pc: BTreeMap::new(),
        }
    }

    /// Look up (or compute) the list of `FrameData`s from a physical
    /// `Frame`.
    fn lookup_or_compute<'a>(
        &'a mut self,
        registry: &ModuleRegistry,
        frame: Frame,
    ) -> &'a [FrameData] {
        let pc = StoreCodePC::from_raw(frame.pc());
        match self.by_pc.entry(pc) {
            Entry::Occupied(frames) => frames.into_mut(),
            Entry::Vacant(v) => {
                // Although inlining can mix modules, `module` is the
                // module that actually contains the physical PC
                // (i.e., the outermost function that inlined the
                // others).
                let (module, frames) = VirtualFrame::decode(registry, frame.pc());
                let frames = frames
                    .into_iter()
                    .map(|frame| FrameData::compute(frame, &module))
                    .collect::<Vec<_>>();
                v.insert(frames)
            }
        }
    }
}

/// Internal data pre-computed for one stack frame.
///
/// This represents one frame as produced by the progpoint lookup
/// (Wasm PC, frame descriptor index, stack shape).
struct VirtualFrame {
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
    fn decode(registry: &ModuleRegistry, pc: usize) -> (Module, Vec<VirtualFrame>) {
        let (module_with_code, pc) = registry
            .module_and_code_by_pc(pc)
            .expect("Wasm frame PC does not correspond to a module");
        let module = module_with_code.module();
        let table = module.frame_table().unwrap();
        let pc = u32::try_from(pc).expect("PC offset too large");
        let program_points = table.find_program_point(pc, FrameInstPos::Post)
            .expect("There must be a program point record in every frame when debug instrumentation is enabled");

        (
            module.clone(),
            program_points
                .map(|(wasm_pc, frame_descriptor, stack_shape)| VirtualFrame {
                    wasm_pc,
                    frame_descriptor,
                    stack_shape,
                })
                .collect(),
        )
    }
}

/// Data computed when we visit a given frame.
struct FrameData {
    slot_to_fp_offset: usize,
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
    fn compute(frame: VirtualFrame, module: &Module) -> Self {
        let frame_table = module.frame_table().unwrap();
        // Parse the frame descriptor.
        let (data, slot_to_fp_offset) = frame_table
            .frame_descriptor(frame.frame_descriptor)
            .unwrap();
        let frame_state_slot = FrameStateSlot::parse(data).unwrap();
        let slot_to_fp_offset = usize::try_from(slot_to_fp_offset).unwrap();

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
            slot_to_fp_offset,
            func_key: frame_state_slot.func_key(),
            wasm_pc: frame.wasm_pc,
            stack,
            locals,
        }
    }

    fn slot_addr(&self, fp: usize) -> *mut u8 {
        let fp: *mut u8 = core::ptr::with_exposed_provenance_mut(fp);
        fp.wrapping_sub(self.slot_to_fp_offset)
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

/// One debug event that occurs when running Wasm code on a store with
/// a debug handler attached.
#[derive(Debug)]
pub enum DebugEvent<'a> {
    /// A [`wasmtime::Error`](crate::Error) was raised by a hostcall.
    HostcallError(&'a crate::Error),
    /// An exception is thrown and caught by Wasm. The current state
    /// is at the throw-point.
    CaughtExceptionThrown(OwnedRooted<ExnRef>),
    /// An exception was not caught and is escaping to the host.
    UncaughtExceptionThrown(OwnedRooted<ExnRef>),
    /// A Wasm trap occurred.
    Trap(Trap),
    /// A breakpoint was reached.
    Breakpoint,
    /// An epoch yield occurred.
    EpochYield,
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
///
/// [`Store::run_concurrent`]: crate::Store::run_concurrent
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
            code_memory.unpublish()?;
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
            if let Err(e) = store_code
                .code_memory_mut()
                .expect("Must have unique ownership of StoreCode in guest-debug mode")
                .publish()
            {
                abort_on_republish_error(e);
            }
        }
    }
}

/// Abort when we cannot re-publish executable code.
///
/// Note that this puts us in quite a conundrum. Typically we will
/// have been editing breakpoints from within a hostcall context
/// (e.g. inside a debugger hook while execution is paused) with JIT
/// code on the stack. Wasmtime's usual path to return errors is back
/// through that JIT code: we do not panic-unwind across the JIT code,
/// we return into the exit trampoline and that then re-enters the
/// raise libcall to use a Cranelift exception-throw to cross most of
/// the JIT frames to the entry trampoline. When even trampolines are
/// no longer executable, we have no way out. Even an ordinary
/// `panic!` cannot work, because we catch panics and carry them
/// across JIT code using that trampoline-based error path. Our only
/// way out is to directly abort the whole process.
///
/// This is not without precedent: other engines have similar failure
/// paths. For example, SpiderMonkey directly aborts the process when
/// failing to re-apply executable permissions (see [1]).
///
/// Note that we don't really expect to ever hit this case in
/// practice: it's unlikely that `mprotect` applying `PROT_EXEC` would
/// fail due to, e.g., resource exhaustion in the kernel, because we
/// will have the same net number of virtual memory areas before and
/// after the permissions change. Nevertheless, we have to account for
/// the possibility of error.
///
/// [1]: https://searchfox.org/firefox-main/rev/7496c8515212669451d7e775a00c2be07da38ca5/js/src/jit/AutoWritableJitCode.h#26-56
#[cfg(feature = "std")]
fn abort_on_republish_error(e: crate::Error) -> ! {
    log::error!(
        "Failed to re-publish executable code: {e:?}. Wasmtime cannot return through JIT code on the stack and cannot even panic; aborting the process."
    );
    std::process::abort();
}

/// In the `no_std` case, we don't have a concept of a "process
/// abort", so rely on `panic!`. Typically an embedded scenario that
/// uses `no_std` will build with `panic=abort` so the effect is the
/// same. If it doesn't, there is truly nothing we can do here so
/// let's panic anyway; the panic propagation through the trampolines
/// will at least deterministically crash.
#[cfg(not(feature = "std"))]
fn abort_on_republish_error(e: crate::Error) -> ! {
    panic!("Failed to re-publish executable code: {e:?}");
}
