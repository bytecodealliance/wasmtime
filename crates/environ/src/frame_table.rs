//! Frame-table parser and lookup logic.
//!
//! This module contains utilities to interpret the `.wasmtime.frame`
//! section in a compiled artifact as produced by
//! [`crate::compile::frame_table::FrameTableBuilder`].

use crate::FuncKey;
use alloc::vec::Vec;
use object::{Bytes, LittleEndian, U32Bytes};

/// An index into the table of stack shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrameStackShape(pub(crate) u32);
impl FrameStackShape {
    pub(crate) fn index(self) -> usize {
        usize::try_from(self.0).unwrap()
    }

    /// Get the raw stack-shape index suitable for serializing into
    /// metadata.
    pub fn raw(self) -> u32 {
        self.0
    }

    /// Wrap a raw stack shape index (e.g. from debug tags) into a FrameStackShape.
    pub fn from_raw(index: u32) -> FrameStackShape {
        FrameStackShape(index)
    }
}

/// An index to a frame descriptor that can be referenced from a
/// program point descriptor.
#[derive(Clone, Copy, Debug)]
pub struct FrameTableDescriptorIndex(pub(crate) u32);
impl FrameTableDescriptorIndex {
    fn index(self) -> usize {
        usize::try_from(self.0).unwrap()
    }
}

/// A parser for a frame-table section.
///
/// This parser holds slices to the in-memory section data, and is
/// cheap to construct: it reads some header fields but does not
/// interpret or validate content data until queried.
pub struct FrameTable<'a> {
    frame_descriptor_ranges: &'a [U32Bytes<LittleEndian>],
    frame_descriptor_data: &'a [u8],

    frame_descriptor_fp_offsets: &'a [U32Bytes<LittleEndian>],

    progpoint_pcs: &'a [U32Bytes<LittleEndian>],
    progpoint_descriptor_offsets: &'a [U32Bytes<LittleEndian>],
    progpoint_descriptor_data: &'a [U32Bytes<LittleEndian>],

    breakpoint_pcs: &'a [U32Bytes<LittleEndian>],
    breakpoint_patch_offsets: &'a [U32Bytes<LittleEndian>],
    breakpoint_patch_data_ends: &'a [U32Bytes<LittleEndian>],
    breakpoint_patch_data: &'a [u8],

    original_text: &'a [u8],
}

impl<'a> FrameTable<'a> {
    /// Parse a frame table section from a byte-slice as produced by
    /// [`crate::compile::frame_table::FrameTableBuilder`].
    pub fn parse(data: &'a [u8], original_text: &'a [u8]) -> anyhow::Result<FrameTable<'a>> {
        let mut data = Bytes(data);
        let num_frame_descriptors = data
            .read::<U32Bytes<LittleEndian>>()
            .map_err(|_| anyhow::anyhow!("Unable to read frame descriptor count prefix"))?;
        let num_frame_descriptors = usize::try_from(num_frame_descriptors.get(LittleEndian))?;
        let num_progpoint_descriptors = data
            .read::<U32Bytes<LittleEndian>>()
            .map_err(|_| anyhow::anyhow!("Unable to read progpoint descriptor count prefix"))?;
        let num_progpoint_descriptors =
            usize::try_from(num_progpoint_descriptors.get(LittleEndian))?;
        let num_breakpoints = data
            .read::<U32Bytes<LittleEndian>>()
            .map_err(|_| anyhow::anyhow!("Unable to read breakpoint count prefix"))?;
        let num_breakpoints = usize::try_from(num_breakpoints.get(LittleEndian))?;

        let frame_descriptor_pool_length = data
            .read::<U32Bytes<LittleEndian>>()
            .map_err(|_| anyhow::anyhow!("Unable to read frame descriptor pool length"))?;
        let frame_descriptor_pool_length =
            usize::try_from(frame_descriptor_pool_length.get(LittleEndian))?;
        let progpoint_descriptor_pool_length = data
            .read::<U32Bytes<LittleEndian>>()
            .map_err(|_| anyhow::anyhow!("Unable to read progpoint descriptor pool length"))?;
        let progpoint_descriptor_pool_length =
            usize::try_from(progpoint_descriptor_pool_length.get(LittleEndian))?;
        let breakpoint_patch_pool_length = data
            .read::<U32Bytes<LittleEndian>>()
            .map_err(|_| anyhow::anyhow!("Unable to read breakpoint patch pool length"))?;
        let breakpoint_patch_pool_length =
            usize::try_from(breakpoint_patch_pool_length.get(LittleEndian))?;

        let (frame_descriptor_ranges, data) =
            object::slice_from_bytes::<U32Bytes<LittleEndian>>(data.0, 2 * num_frame_descriptors)
                .map_err(|_| anyhow::anyhow!("Unable to read frame descriptor ranges slice"))?;
        let (frame_descriptor_fp_offsets, data) =
            object::slice_from_bytes::<U32Bytes<LittleEndian>>(data, num_frame_descriptors)
                .map_err(|_| anyhow::anyhow!("Unable to read frame descriptor FP offset slice"))?;

        let (progpoint_pcs, data) =
            object::slice_from_bytes::<U32Bytes<LittleEndian>>(data, num_progpoint_descriptors)
                .map_err(|_| anyhow::anyhow!("Unable to read progpoint PC slice"))?;
        let (progpoint_descriptor_offsets, data) =
            object::slice_from_bytes::<U32Bytes<LittleEndian>>(data, num_progpoint_descriptors)
                .map_err(|_| anyhow::anyhow!("Unable to read progpoint descriptor offset slice"))?;
        let (breakpoint_pcs, data) =
            object::slice_from_bytes::<U32Bytes<LittleEndian>>(data, num_breakpoints)
                .map_err(|_| anyhow::anyhow!("Unable to read breakpoint PC slice"))?;
        let (breakpoint_patch_offsets, data) =
            object::slice_from_bytes::<U32Bytes<LittleEndian>>(data, num_breakpoints)
                .map_err(|_| anyhow::anyhow!("Unable to read breakpoint patch offsets slice"))?;
        let (breakpoint_patch_data_ends, data) =
            object::slice_from_bytes::<U32Bytes<LittleEndian>>(data, num_breakpoints)
                .map_err(|_| anyhow::anyhow!("Unable to read breakpoint patch data ends slice"))?;

        let (frame_descriptor_data, data) = data
            .split_at_checked(frame_descriptor_pool_length)
            .ok_or_else(|| anyhow::anyhow!("Unable to read frame descriptor pool"))?;

        let (progpoint_descriptor_data, data) = object::slice_from_bytes::<U32Bytes<LittleEndian>>(
            data,
            progpoint_descriptor_pool_length,
        )
        .map_err(|_| anyhow::anyhow!("Unable to read progpoint descriptor pool"))?;

        let (breakpoint_patch_data, _) = data
            .split_at_checked(breakpoint_patch_pool_length)
            .ok_or_else(|| anyhow::anyhow!("Unable to read breakpoint patch pool"))?;

        Ok(FrameTable {
            frame_descriptor_ranges,
            frame_descriptor_data,
            frame_descriptor_fp_offsets,
            progpoint_pcs,
            progpoint_descriptor_offsets,
            progpoint_descriptor_data,
            breakpoint_pcs,
            breakpoint_patch_offsets,
            breakpoint_patch_data_ends,
            breakpoint_patch_data,
            original_text,
        })
    }

    /// Get raw frame descriptor data and slot-to-FP-offset for a
    /// given frame descriptor.
    pub fn frame_descriptor(
        &self,
        frame_descriptor: FrameTableDescriptorIndex,
    ) -> Option<(&'a [u8], u32)> {
        let range_start = self
            .frame_descriptor_ranges
            .get(frame_descriptor.index() * 2)?
            .get(LittleEndian);
        let range_end = self
            .frame_descriptor_ranges
            .get(frame_descriptor.index() * 2 + 1)?
            .get(LittleEndian);
        let range_start = usize::try_from(range_start).unwrap();
        let range_end = usize::try_from(range_end).unwrap();
        if range_end < range_start || range_end > self.frame_descriptor_data.len() {
            return None;
        }
        let descriptor = &self.frame_descriptor_data[range_start..range_end];
        let slot_to_fp_offset = self
            .frame_descriptor_fp_offsets
            .get(frame_descriptor.index())?
            .get(LittleEndian);
        Some((descriptor, slot_to_fp_offset))
    }

    /// Get frames for the program point at the PC upper-bounded by a
    /// given search PC (offset in text section).
    pub fn find_program_point(
        &self,
        search_pc: u32,
        search_pos: FrameInstPos,
    ) -> Option<impl Iterator<Item = (u32, FrameTableDescriptorIndex, FrameStackShape)>> {
        let key = FrameInstPos::encode(search_pc, search_pos);
        let index = match self
            .progpoint_pcs
            .binary_search_by_key(&key, |entry| entry.get(LittleEndian))
        {
            Ok(idx) => idx,
            Err(idx) if idx > 0 => idx - 1,
            Err(_) => return None,
        };

        Some(self.program_point_frame_iter(index))
    }

    /// Get all program point records with iterators over
    /// corresponding frames for each.
    pub fn into_program_points(
        self,
    ) -> impl Iterator<
        Item = (
            u32,
            FrameInstPos,
            Vec<(u32, FrameTableDescriptorIndex, FrameStackShape)>,
        ),
    > + 'a {
        self.progpoint_pcs.iter().enumerate().map(move |(i, pc)| {
            let pc_and_pos = pc.get(LittleEndian);
            let (pc, pos) = FrameInstPos::decode(pc_and_pos);
            (
                pc,
                pos,
                self.program_point_frame_iter(i).collect::<Vec<_>>(),
            )
        })
    }

    fn program_point_frame_iter(
        &self,
        index: usize,
    ) -> impl Iterator<Item = (u32, FrameTableDescriptorIndex, FrameStackShape)> {
        let offset =
            usize::try_from(self.progpoint_descriptor_offsets[index].get(LittleEndian)).unwrap();
        let mut data = &self.progpoint_descriptor_data[offset..];

        core::iter::from_fn(move || {
            if data.len() < 3 {
                return None;
            }
            let wasm_pc = data[0].get(LittleEndian);
            let frame_descriptor = FrameTableDescriptorIndex(data[1].get(LittleEndian));
            let stack_shape = FrameStackShape(data[2].get(LittleEndian));
            data = &data[3..];
            let not_last = wasm_pc & 0x8000_0000 != 0;
            let wasm_pc = wasm_pc & 0x7fff_ffff;
            if !not_last {
                data = &[];
            }
            Some((wasm_pc, frame_descriptor, stack_shape))
        })
    }

    /// For a given breakpoint index, return the patch offset in text,
    /// the patch data, and the original data.
    fn breakpoint_patch(&self, i: usize) -> FrameTableBreakpointData<'_> {
        let patch_pool_start = if i == 0 {
            0
        } else {
            self.breakpoint_patch_data_ends[i - 1].get(LittleEndian)
        };
        let patch_pool_end = self.breakpoint_patch_data_ends[i].get(LittleEndian);
        let patch_pool_start = usize::try_from(patch_pool_start).unwrap();
        let patch_pool_end = usize::try_from(patch_pool_end).unwrap();
        let len = patch_pool_end - patch_pool_start;
        let offset = self.breakpoint_patch_offsets[i].get(LittleEndian);
        let offset = usize::try_from(offset).unwrap();
        let original_data = &self.original_text[offset..offset + len];
        FrameTableBreakpointData {
            offset,
            enable: &self.breakpoint_patch_data[patch_pool_start..patch_pool_end],
            disable: original_data,
        }
    }

    /// Find a list of breakpoint patches for a given Wasm PC.
    pub fn lookup_breakpoint_patches_by_pc(
        &self,
        pc: u32,
    ) -> impl Iterator<Item = FrameTableBreakpointData<'_>> + '_ {
        // Find *some* entry with a matching Wasm PC. Note that there
        // may be multiple entries for one PC.
        let range = match self
            .breakpoint_pcs
            .binary_search_by_key(&pc, |p| p.get(LittleEndian))
        {
            Ok(mut i) => {
                // Scan backward to first index with this PC.
                while i > 0 && self.breakpoint_pcs[i - 1].get(LittleEndian) == pc {
                    i -= 1;
                }

                // Scan forward to find the end of the range.
                let mut end = i;
                while end < self.breakpoint_pcs.len()
                    && self.breakpoint_pcs[end].get(LittleEndian) == pc
                {
                    end += 1;
                }

                i..end
            }
            Err(_) => 0..0,
        };

        range.map(|i| self.breakpoint_patch(i))
    }

    /// Return an iterator over all breakpoint patches.
    ///
    /// Returned tuples are (Wasm PC, breakpoint data).
    pub fn breakpoint_patches(
        &self,
    ) -> impl Iterator<Item = (u32, FrameTableBreakpointData<'_>)> + '_ {
        self.breakpoint_pcs.iter().enumerate().map(|(i, wasm_pc)| {
            let wasm_pc = wasm_pc.get(LittleEndian);
            let data = self.breakpoint_patch(i);
            (wasm_pc, data)
        })
    }
}

/// Data describing how to patch code to enable or disable one
/// breakpoint.
pub struct FrameTableBreakpointData<'a> {
    /// Offset in the code image's text section.
    pub offset: usize,
    /// Code bytes to patch in to enable the breakpoint.
    pub enable: &'a [u8],
    /// Code bytes to patch in to disable the breakpoint.
    pub disable: &'a [u8],
}

/// An instruction position for a program point.
///
/// We attach debug metadata to a *position* on an offset in the text
/// (code) section, either "post" or "pre". The "post" position
/// logically comes first, and is associated with the instruction that
/// ends at this offset (i.e., the previous instruction). The "pre"
/// position comes next, and is associated with the instruction that
/// begins at this offset (i.e., the next instruction).
///
/// We make this distinction because metadata lookups sometimes occur
/// with a PC that is after the instruction (e.g., the return address
/// after a call instruction), and sometimes at the instruction (e.g.,
/// a trapping PC address). The lookup context will know which one to
/// use -- e.g., when walking the stack, "pre" for a trapping PC and
/// "post" for every frame after that -- so we simply encode it as
/// part of the position and allow searching on it.
///
/// The need for this distinction can be understood by way of an
/// example; say we have:
///
/// ```plain
/// call ...
/// trapping_store ...
/// ```
///
/// where both instructions have debug metadata. We might look up the
/// PC of `trapping_store` once as we walk the stack from within the
/// call (we will get this PC because it is the return address) and
/// once when `trapping_store` itself traps; and we want different
/// metadata in each case.
///
/// An alternative is to universally attach tags to the end offset of
/// an instruction, which allows us to handle return addresses
/// naturally but requires traps to adjust their PC. However, this
/// requires trap handlers to know the length of the trapping
/// instruction, which is not always easy -- in the most general case,
/// on variable-length instruction sets, it requires a full
/// instruction decoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrameInstPos {
    /// The "post" position at an offset attaches to the instruction
    /// that ends at this offset, i.e., came previously.
    Post,
    /// The "pre" position at an offset attaches to the instruction
    /// that begins at this offset, i.e., comes next.
    Pre,
}

impl FrameInstPos {
    pub(crate) fn encode(pc: u32, pos: FrameInstPos) -> u32 {
        let lsb = match pos {
            Self::Post => 0,
            Self::Pre => 1,
        };
        debug_assert!(pc < 0x8000_0000);
        (pc << 1) | lsb
    }
    pub(crate) fn decode(bits: u32) -> (u32, FrameInstPos) {
        let pos = match bits & 1 {
            0 => Self::Post,
            1 => Self::Pre,
            _ => unreachable!(),
        };
        let pc = bits >> 1;
        (pc, pos)
    }
}

/// An offset into the state slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrameStateSlotOffset(pub(crate) u32);
impl FrameStateSlotOffset {
    #[cfg(feature = "compile")]
    pub(crate) fn add(self, offset: u32) -> FrameStateSlotOffset {
        FrameStateSlotOffset(self.0 + offset)
    }

    /// Get the offset into the state stackslot, suitable for use in a
    /// `stack_store`/`stack_load` instruction.
    pub fn offset(self) -> i32 {
        i32::try_from(self.0).unwrap()
    }
}

/// A type stored in a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(missing_docs, reason = "self-describing variants")]
pub enum FrameValType {
    I32,
    I64,
    F32,
    F64,
    V128,
    AnyRef,
    FuncRef,
    ExternRef,
    ExnRef,
    ContRef,
}

impl FrameValType {
    #[cfg(feature = "compile")]
    pub(crate) fn storage_size(&self, pointer_size: u32) -> u32 {
        match self {
            FrameValType::I32 => 4,
            FrameValType::I64 => 8,
            FrameValType::F32 => 4,
            FrameValType::F64 => 8,
            FrameValType::V128 => 16,
            FrameValType::AnyRef | FrameValType::ExternRef | FrameValType::ExnRef => 4,
            FrameValType::FuncRef => pointer_size,
            FrameValType::ContRef => 2 * pointer_size,
        }
    }
}

impl From<FrameValType> for u8 {
    fn from(value: FrameValType) -> u8 {
        match value {
            FrameValType::I32 => 0,
            FrameValType::I64 => 1,
            FrameValType::F32 => 2,
            FrameValType::F64 => 3,
            FrameValType::V128 => 4,
            FrameValType::AnyRef => 5,
            FrameValType::FuncRef => 6,
            FrameValType::ExternRef => 7,
            FrameValType::ExnRef => 8,
            FrameValType::ContRef => 9,
        }
    }
}

impl TryFrom<u8> for FrameValType {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> anyhow::Result<Self> {
        match value {
            0 => Ok(Self::I32),
            1 => Ok(Self::I64),
            2 => Ok(Self::F32),
            3 => Ok(Self::F64),
            4 => Ok(Self::V128),
            5 => Ok(Self::AnyRef),
            6 => Ok(Self::FuncRef),
            7 => Ok(Self::ExternRef),
            8 => Ok(Self::ExnRef),
            9 => Ok(Self::ContRef),
            _ => Err(anyhow::anyhow!("Invalid type")),
        }
    }
}

/// Parser for a frame state slot descriptor.
///
/// This provides the ability to extract offsets and types for locals
/// and for the stack given a stack shape.
pub struct FrameStateSlot<'a> {
    func_key: FuncKey,
    local_offsets: &'a [U32Bytes<LittleEndian>],
    stack_shape_parents: &'a [U32Bytes<LittleEndian>],
    stack_shape_offsets: &'a [U32Bytes<LittleEndian>],
    local_types: &'a [u8],
    stack_shape_types: &'a [u8],
}

impl<'a> FrameStateSlot<'a> {
    /// Parse a slot descriptor.
    ///
    /// This parses the descriptor bytes as provided by
    /// [`FrameTable::frame_descriptor`].
    pub fn parse(descriptor: &'a [u8]) -> anyhow::Result<FrameStateSlot<'a>> {
        let mut data = Bytes(descriptor);
        let func_key_namespace = data
            .read::<U32Bytes<LittleEndian>>()
            .map_err(|_| anyhow::anyhow!("Unable to read func key namespace"))?
            .get(LittleEndian);
        let func_key_index = data
            .read::<U32Bytes<LittleEndian>>()
            .map_err(|_| anyhow::anyhow!("Unable to read func key index"))?
            .get(LittleEndian);
        let func_key = FuncKey::from_raw_parts(func_key_namespace, func_key_index);

        let num_locals = data
            .read::<U32Bytes<LittleEndian>>()
            .map_err(|_| anyhow::anyhow!("Unable to read num_locals"))?
            .get(LittleEndian);
        let num_locals = usize::try_from(num_locals)?;
        let num_stack_shapes = data
            .read::<U32Bytes<LittleEndian>>()
            .map_err(|_| anyhow::anyhow!("Unable to read num_stack_shapes"))?
            .get(LittleEndian);
        let num_stack_shapes = usize::try_from(num_stack_shapes)?;

        let (local_offsets, data) =
            object::slice_from_bytes::<U32Bytes<LittleEndian>>(data.0, num_locals)
                .map_err(|_| anyhow::anyhow!("Unable to read local_offsets slice"))?;
        let (stack_shape_parents, data) =
            object::slice_from_bytes::<U32Bytes<LittleEndian>>(data, num_stack_shapes)
                .map_err(|_| anyhow::anyhow!("Unable to read stack_shape_parents slice"))?;
        let (stack_shape_offsets, data) =
            object::slice_from_bytes::<U32Bytes<LittleEndian>>(data, num_stack_shapes)
                .map_err(|_| anyhow::anyhow!("Unable to read stack_shape_offsets slice"))?;
        let (local_types, data) = data
            .split_at_checked(num_locals)
            .ok_or_else(|| anyhow::anyhow!("Unable to read local_types slice"))?;
        let (stack_shape_types, _) = data
            .split_at_checked(num_stack_shapes)
            .ok_or_else(|| anyhow::anyhow!("Unable to read stack_shape_types slice"))?;

        Ok(FrameStateSlot {
            func_key,
            local_offsets,
            stack_shape_parents,
            stack_shape_offsets,
            local_types,
            stack_shape_types,
        })
    }

    /// Get the FuncKey for the function that produced this frame
    /// slot.
    pub fn func_key(&self) -> FuncKey {
        self.func_key
    }

    /// Get the local offsets and types.
    pub fn locals(&self) -> impl Iterator<Item = (FrameStateSlotOffset, FrameValType)> {
        (0..self.num_locals()).map(|i| self.local(i).unwrap())
    }

    /// Get the type and offset for a given local.
    pub fn local(&self, index: usize) -> Option<(FrameStateSlotOffset, FrameValType)> {
        let offset = FrameStateSlotOffset(self.local_offsets.get(index)?.get(LittleEndian));
        let ty = FrameValType::try_from(*self.local_types.get(index)?).expect("Invalid type");
        Some((offset, ty))
    }

    /// Get the number of locals in the frame.
    pub fn num_locals(&self) -> usize {
        self.local_offsets.len()
    }

    /// Get the offsets and types for operand stack values, from top
    /// of stack (most recently pushed) down.
    pub fn stack(
        &self,
        shape: FrameStackShape,
    ) -> impl Iterator<Item = (FrameStateSlotOffset, FrameValType)> {
        fn unpack_option_shape(shape: FrameStackShape) -> Option<FrameStackShape> {
            if shape.0 == u32::MAX {
                None
            } else {
                Some(shape)
            }
        }

        let mut shape = unpack_option_shape(shape);
        core::iter::from_fn(move || {
            shape.map(|s| {
                let parent = FrameStackShape(self.stack_shape_parents[s.index()].get(LittleEndian));
                let parent = unpack_option_shape(parent);
                let offset =
                    FrameStateSlotOffset(self.stack_shape_offsets[s.index()].get(LittleEndian));
                let ty = FrameValType::try_from(self.stack_shape_types[s.index()])
                    .expect("Invalid type");
                shape = parent;
                (offset, ty)
            })
        })
    }

    /// Returns an iterator over all storage in this frame.
    pub fn stack_and_locals(
        &self,
        shape: FrameStackShape,
    ) -> impl Iterator<Item = (FrameStateSlotOffset, FrameValType)> + '_ {
        self.locals().chain(self.stack(shape))
    }
}
