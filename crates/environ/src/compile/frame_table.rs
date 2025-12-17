//! Builder for the `ELF_WASMTIME_FRAME_TABLE` ("frame table") section
//! in compiled executables.
//!
//! This section is present only if debug instrumentation is
//! enabled. It describes functions, stackslots that carry Wasm state,
//! and allows looking up active Wasm frames (including multiple
//! frames in one function due to inlining), Wasm local types and Wasm
//! operand stack depth in each frame by PC, with offsets to read
//! those values off of the state in the stack frame.

use crate::{
    FrameInstPos, FrameStackShape, FrameStateSlotOffset, FrameTableDescriptorIndex, FrameValType,
    FuncKey, WasmHeapTopType, WasmValType, prelude::*,
};
use object::{LittleEndian, U32Bytes};
use std::collections::{HashMap, hash_map::Entry};

/// Builder for a stackslot descriptor.
pub struct FrameStateSlotBuilder {
    /// Function identifier for this state slot.
    func_key: FuncKey,

    /// Pointer size for target.
    pointer_size: u32,

    /// Local types and offsets.
    locals: Vec<(FrameValType, FrameStateSlotOffset)>,

    /// Stack nodes: (parent, type, offset) tuples.
    stacks: Vec<(Option<FrameStackShape>, FrameValType, FrameStateSlotOffset)>,

    /// Hashconsing for stack-type nodes.
    stacks_dedup:
        HashMap<(Option<FrameStackShape>, FrameValType, FrameStateSlotOffset), FrameStackShape>,

    /// Size of vmctx (one pointer).
    vmctx_size: u32,

    /// Size of all locals.
    locals_size: u32,

    /// Maximum size of whole state slot.
    slot_size: u32,
}

impl From<WasmValType> for FrameValType {
    fn from(ty: WasmValType) -> FrameValType {
        match ty {
            WasmValType::I32 => FrameValType::I32,
            WasmValType::I64 => FrameValType::I64,
            WasmValType::F32 => FrameValType::F32,
            WasmValType::F64 => FrameValType::F64,
            WasmValType::V128 => FrameValType::V128,
            WasmValType::Ref(r) => match r.heap_type.top() {
                WasmHeapTopType::Any => FrameValType::AnyRef,
                WasmHeapTopType::Extern => FrameValType::ExternRef,
                WasmHeapTopType::Func => FrameValType::FuncRef,
                WasmHeapTopType::Exn => FrameValType::ExnRef,
                WasmHeapTopType::Cont => FrameValType::ContRef,
            },
        }
    }
}

impl FrameStateSlotBuilder {
    /// Create a new state-slot builder.
    pub fn new(func_key: FuncKey, pointer_size: u32) -> FrameStateSlotBuilder {
        FrameStateSlotBuilder {
            func_key,
            pointer_size,
            locals: vec![],
            stacks: vec![],
            stacks_dedup: HashMap::new(),
            vmctx_size: pointer_size,
            locals_size: 0,
            slot_size: pointer_size,
        }
    }

    /// Add a local to the state-slot.
    ///
    /// Locals must be added in local index order, and must be added
    /// before any stack shapes are defined. The offset in the state
    /// slot is returned.
    pub fn add_local(&mut self, ty: FrameValType) -> FrameStateSlotOffset {
        // N.B.: the vmctx pointer is always at offset 0, so we add
        // its size here.
        let offset = FrameStateSlotOffset(self.vmctx_size + self.locals_size);
        let size = ty.storage_size(self.pointer_size);
        self.locals_size += size;
        self.slot_size += size;
        self.locals.push((ty, offset));
        offset
    }

    /// Get a local's offset in the state-slot.
    pub fn local_offset(&self, local: u32) -> FrameStateSlotOffset {
        let index = usize::try_from(local).unwrap();
        self.locals[index].1
    }

    /// Push a stack entry. Returns the stack-shape descriptor and the
    /// offset at which to write the pushed value.
    pub fn push_stack(
        &mut self,
        parent: Option<FrameStackShape>,
        ty: FrameValType,
    ) -> (FrameStackShape, FrameStateSlotOffset) {
        let offset = parent
            .map(|parent| {
                let (_, ty, offset) = self.stacks[parent.index()];
                offset.add(ty.storage_size(self.pointer_size))
            })
            // N.B.: the stack starts at vmctx_size + locals_size,
            // because the layout puts vmctx first, then locals, then
            // stack.
            .unwrap_or(FrameStateSlotOffset(self.vmctx_size + self.locals_size));

        self.slot_size = core::cmp::max(
            self.slot_size,
            offset.0 + ty.storage_size(self.pointer_size),
        );

        let shape = match self.stacks_dedup.entry((parent, ty, offset)) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                let shape = FrameStackShape(u32::try_from(self.stacks.len()).unwrap());
                self.stacks.push((parent, ty, offset));
                *v.insert(shape)
            }
        };

        (shape, offset)
    }

    /// Get the offset for the top slot in a given stack shape.
    pub fn stack_last_offset(&self, shape: FrameStackShape) -> FrameStateSlotOffset {
        self.stacks[shape.index()].2
    }

    /// Serialize the frame-slot descriptor so it can be included as
    /// metadata.
    pub fn serialize(&self) -> Vec<u8> {
        // Format (all little-endian):
        // - func_key: (u32, u32)
        // - num_locals: u32
        // - num_stack_shapes: u32
        // - local_offsets: num_locals times:
        //   - offset: u32 (offset from start of state slot)
        // - stack_shape_parents: num_stack_shapes times:
        //   - parent_shape: u32 (or u32::MAX for none)
        // - stack_shape_offsets: num_stack_shapes times:
        //   - offset: u32 (offset from start of state slot for top-of-stack value)
        // - local_types: num_locals times:
        //   - type: u8
        // - stack_shape_types: num_stack_shapes times:
        //   - type: u8 (type of top-of-stack value)

        let mut buffer = vec![];
        let (func_key_namespace, func_key_index) = self.func_key.into_parts();
        buffer.extend_from_slice(&u32::to_le_bytes(func_key_namespace.into_raw()));
        buffer.extend_from_slice(&u32::to_le_bytes(func_key_index.into_raw()));

        buffer.extend_from_slice(&u32::to_le_bytes(u32::try_from(self.locals.len()).unwrap()));
        buffer.extend_from_slice(&u32::to_le_bytes(u32::try_from(self.stacks.len()).unwrap()));

        for (_, offset) in &self.locals {
            buffer.extend_from_slice(&u32::to_le_bytes(offset.0));
        }
        for (parent, _, _) in &self.stacks {
            let parent = parent.map(|p| p.0).unwrap_or(u32::MAX);
            buffer.extend_from_slice(&u32::to_le_bytes(parent));
        }
        for (_, _, offset) in &self.stacks {
            buffer.extend_from_slice(&u32::to_le_bytes(offset.0));
        }
        for (ty, _) in &self.locals {
            buffer.push(*ty as u8);
        }
        for (_, ty, _) in &self.stacks {
            buffer.push(*ty as u8);
        }

        buffer
    }

    /// The total size required for all locals/stack storage.
    pub fn size(&self) -> u32 {
        self.slot_size
    }
}

/// Builder for the Frame Table.
///
/// Format:
///
/// - `num_slot_descriptors`: u32
/// - `num_progpoints`: u32
/// - `num_breakpoints`: u32
/// - `frame_descriptor_pool_length`: u32
/// - `progpoint_descriptor_pool_length`: u32
/// - `breakpoint_patch_pool_length`: u32
/// - `num_slot_descriptors` times:
///   - frame descriptor offset: u32
///   - length: u32
/// - `num_slot_descriptors` times:
///   - offset from frame up to FP: u32
/// - `num_progpoints` times:
///   - PC, from start of text section, position (post/pre): u32
///     - encoded as (pc << 1) | post_pre_bit
/// - `num_progpoints` times:
///   - progpoint descriptor offset: u32
/// - `num_breakpoints` times:
///    - Wasm PC: u32 (sorted order; may repeat)
/// - `num_breakpoints` times:
///    - patch offset in text: u32
/// - `num_breakpoints` times:
///    - end of breakpoint patch data in pool: u32
///      (find the start by end of previous; patches are in the
///      pool in order and this saves storing redundant start/end values)
/// - frame descriptors (format described above; `frame_descriptor_pool_length` bytes)
/// - progpoint descriptors (`progpoint_descriptor_pool_length` bytes)
///   - each descriptor: sequence of frames
///     - Wasm PC: u32 (high bit set to indicate a parent frame)
///     - slot descriptor index: u32
///     - stack shape index: u32 (or u32::MAX for none)
/// - breakpoint patch pool (`breakpoint_patch_pool_length` bytes)
///   - freeform slices of machine-code bytes to patch in
#[derive(Default)]
pub struct FrameTableBuilder {
    /// (offset, length) pairs into `frame_descriptor_data`, indexed
    /// by frame descriptor number.
    frame_descriptor_ranges: Vec<U32Bytes<LittleEndian>>,
    frame_descriptor_data: Vec<u8>,

    /// Offset from frame slot up to FP for each frame descriptor.
    frame_descriptor_fp_offsets: Vec<U32Bytes<LittleEndian>>,

    progpoint_pcs: Vec<U32Bytes<LittleEndian>>,
    progpoint_descriptor_offsets: Vec<U32Bytes<LittleEndian>>,
    progpoint_descriptor_data: Vec<U32Bytes<LittleEndian>>,

    breakpoint_pcs: Vec<U32Bytes<LittleEndian>>,
    breakpoint_patch_offsets: Vec<U32Bytes<LittleEndian>>,
    breakpoint_patch_data_ends: Vec<U32Bytes<LittleEndian>>,

    breakpoint_patch_data: Vec<u8>,
}

impl FrameTableBuilder {
    /// Add one frame descriptor.
    ///
    /// Returns the frame descriptor index.
    pub fn add_frame_descriptor(
        &mut self,
        slot_to_fp_offset: u32,
        data: &[u8],
    ) -> FrameTableDescriptorIndex {
        let start = u32::try_from(self.frame_descriptor_data.len()).unwrap();
        self.frame_descriptor_data.extend(data.iter().cloned());
        let end = u32::try_from(self.frame_descriptor_data.len()).unwrap();

        let index = FrameTableDescriptorIndex(
            u32::try_from(self.frame_descriptor_fp_offsets.len()).unwrap(),
        );
        self.frame_descriptor_fp_offsets
            .push(U32Bytes::new(LittleEndian, slot_to_fp_offset));
        self.frame_descriptor_ranges
            .push(U32Bytes::new(LittleEndian, start));
        self.frame_descriptor_ranges
            .push(U32Bytes::new(LittleEndian, end));

        index
    }

    /// Add one program point.
    pub fn add_program_point(
        &mut self,
        native_pc: u32,
        pos: FrameInstPos,
        // For each frame: Wasm PC, frame descriptor, stack shape
        // within the frame descriptor.
        frames: &[(u32, FrameTableDescriptorIndex, FrameStackShape)],
    ) {
        let pc_and_pos = FrameInstPos::encode(native_pc, pos);
        // If we already have a program point record at this PC,
        // overwrite it.
        while let Some(last) = self.progpoint_pcs.last()
            && last.get(LittleEndian) == pc_and_pos
        {
            self.progpoint_pcs.pop();
            self.progpoint_descriptor_offsets.pop();
            self.progpoint_descriptor_data
                .truncate(self.progpoint_descriptor_data.len() - 3);
        }

        let start = u32::try_from(self.progpoint_descriptor_data.len()).unwrap();
        self.progpoint_pcs
            .push(U32Bytes::new(LittleEndian, pc_and_pos));
        self.progpoint_descriptor_offsets
            .push(U32Bytes::new(LittleEndian, start));

        for (i, &(wasm_pc, frame_descriptor, stack_shape)) in frames.iter().enumerate() {
            debug_assert!(wasm_pc < 0x8000_0000);
            let not_last = i < (frames.len() - 1);
            let wasm_pc = wasm_pc | if not_last { 0x8000_0000 } else { 0 };
            self.progpoint_descriptor_data
                .push(U32Bytes::new(LittleEndian, wasm_pc));
            self.progpoint_descriptor_data
                .push(U32Bytes::new(LittleEndian, frame_descriptor.0));
            self.progpoint_descriptor_data
                .push(U32Bytes::new(LittleEndian, stack_shape.0));
        }
    }

    /// Add one breakpoint patch.
    pub fn add_breakpoint_patch(&mut self, wasm_pc: u32, patch_start_native_pc: u32, patch: &[u8]) {
        self.breakpoint_pcs
            .push(U32Bytes::new(LittleEndian, wasm_pc));
        self.breakpoint_patch_offsets
            .push(U32Bytes::new(LittleEndian, patch_start_native_pc));
        self.breakpoint_patch_data.extend(patch.iter().cloned());
        let end = u32::try_from(self.breakpoint_patch_data.len()).unwrap();
        self.breakpoint_patch_data_ends
            .push(U32Bytes::new(LittleEndian, end));
    }

    /// Serialize the framd-table data section, taking a closure to
    /// consume slices.
    pub fn serialize<F: FnMut(&[u8])>(&mut self, mut f: F) {
        // Pad `frame_descriptor_data` to a multiple of 4 bytes so
        // `progpoint_descriptor_data` is aligned as well.
        while self.frame_descriptor_data.len() & 3 != 0 {
            self.frame_descriptor_data.push(0);
        }

        let num_frame_descriptors = u32::try_from(self.frame_descriptor_fp_offsets.len()).unwrap();
        f(&num_frame_descriptors.to_le_bytes());
        let num_prog_points = u32::try_from(self.progpoint_pcs.len()).unwrap();
        f(&num_prog_points.to_le_bytes());
        let num_breakpoints = u32::try_from(self.breakpoint_pcs.len()).unwrap();
        f(&num_breakpoints.to_le_bytes());

        let frame_descriptor_pool_length = u32::try_from(self.frame_descriptor_data.len()).unwrap();
        f(&frame_descriptor_pool_length.to_le_bytes());
        let progpoint_descriptor_pool_length =
            u32::try_from(self.progpoint_descriptor_data.len()).unwrap();
        f(&progpoint_descriptor_pool_length.to_le_bytes());
        let breakpoint_patch_pool_length = u32::try_from(self.breakpoint_patch_data.len()).unwrap();
        f(&breakpoint_patch_pool_length.to_le_bytes());

        f(object::bytes_of_slice(&self.frame_descriptor_ranges));
        f(object::bytes_of_slice(&self.frame_descriptor_fp_offsets));
        f(object::bytes_of_slice(&self.progpoint_pcs));
        f(object::bytes_of_slice(&self.progpoint_descriptor_offsets));
        f(object::bytes_of_slice(&self.breakpoint_pcs));
        f(object::bytes_of_slice(&self.breakpoint_patch_offsets));
        f(object::bytes_of_slice(&self.breakpoint_patch_data_ends));
        f(&self.frame_descriptor_data);
        f(object::bytes_of_slice(&self.progpoint_descriptor_data));
        f(&self.breakpoint_patch_data);
    }
}
