use crate::{
    abi::{align_to, ABIOperand, ABISig, LocalSlot, ABI},
    masm::MacroAssembler,
};
use anyhow::Result;
use smallvec::SmallVec;
use std::ops::Range;
use wasmparser::{BinaryReader, FuncValidator, ValidatorResources};
use wasmtime_environ::{TypeConvert, WasmValType};

// TODO:
// SpiderMonkey's implementation uses 16;
// (ref: https://searchfox.org/mozilla-central/source/js/src/wasm/WasmBCFrame.h#585)
// during instrumentation we should measure to verify if this is a good default.
pub(crate) type Locals = SmallVec<[LocalSlot; 16]>;

/// Function defined locals start and end in the frame.
pub(crate) struct DefinedLocalsRange(Range<u32>);

impl DefinedLocalsRange {
    /// Get a reference to the inner range.
    pub fn as_range(&self) -> &Range<u32> {
        &self.0
    }
}

/// An abstraction to read the defined locals from the Wasm binary for a function.
#[derive(Default)]
pub(crate) struct DefinedLocals {
    /// The defined locals for a function.
    pub defined_locals: Locals,
    /// The size of the defined locals.
    pub stack_size: u32,
}

impl DefinedLocals {
    /// Compute the local slots for a Wasm function.
    pub fn new<A: ABI>(
        types: &impl TypeConvert,
        reader: &mut BinaryReader<'_>,
        validator: &mut FuncValidator<ValidatorResources>,
    ) -> Result<Self> {
        let mut next_stack: u32 = 0;
        // The first 32 bits of a Wasm binary function describe the number of locals.
        let local_count = reader.read_var_u32()?;
        let mut slots: Locals = Default::default();

        for _ in 0..local_count {
            let position = reader.original_position();
            let count = reader.read_var_u32()?;
            let ty = reader.read()?;
            validator.define_locals(position, count, ty)?;

            let ty = types.convert_valtype(ty);
            for _ in 0..count {
                let ty_size = <A as ABI>::sizeof(&ty);
                next_stack = align_to(next_stack, ty_size as u32) + (ty_size as u32);
                slots.push(LocalSlot::new(ty, next_stack));
            }
        }

        Ok(Self {
            defined_locals: slots,
            stack_size: next_stack,
        })
    }
}

/// Frame handler abstraction.
pub(crate) struct Frame {
    /// The size of the entire local area; the arguments plus the function defined locals.
    pub locals_size: u32,

    /// The range in the frame corresponding to the defined locals range.
    pub defined_locals_range: DefinedLocalsRange,

    /// The local slots for the current function.
    ///
    /// Locals get calculated when allocating a frame and are readonly
    /// through the function compilation lifetime.
    pub locals: Locals,

    /// The offset to the slot containing the `VMContext`.
    pub vmctx_slot: LocalSlot,

    /// The slot holding the address of the results area.
    pub results_base_slot: Option<LocalSlot>,
}

impl Frame {
    /// Allocate a new [`Frame`].
    pub fn new<A: ABI>(sig: &ABISig, defined_locals: &DefinedLocals) -> Result<Self> {
        let (mut locals, defined_locals_start) = Self::compute_arg_slots::<A>(sig)?;

        // The defined locals have a zero-based offset by default
        // so we need to add the defined locals start to the offset.
        locals.extend(
            defined_locals
                .defined_locals
                .iter()
                .map(|l| LocalSlot::new(l.ty, l.offset + defined_locals_start)),
        );

        // Align the locals to add a slot for the VMContext pointer.
        let ptr_size = <A as ABI>::word_bytes() as u32;
        let vmctx_offset =
            align_to(defined_locals_start + defined_locals.stack_size, ptr_size) + ptr_size;

        let (results_base_slot, locals_size) = if sig.params.has_retptr() {
            match sig.params.unwrap_results_area_operand() {
                ABIOperand::Stack { ty, offset, .. } => (
                    Some(LocalSlot::stack_arg(
                        *ty,
                        *offset + (<A as ABI>::arg_base_offset() as u32),
                    )),
                    align_to(vmctx_offset, <A as ABI>::stack_align().into()),
                ),
                ABIOperand::Reg { ty, .. } => {
                    let offs = align_to(vmctx_offset, ptr_size) + ptr_size;
                    (
                        Some(LocalSlot::new(*ty, offs)),
                        align_to(offs, <A as ABI>::stack_align().into()),
                    )
                }
            }
        } else {
            (
                None,
                align_to(vmctx_offset, <A as ABI>::stack_align().into()),
            )
        };

        Ok(Self {
            locals,
            locals_size,
            vmctx_slot: LocalSlot::i64(vmctx_offset),
            defined_locals_range: DefinedLocalsRange(
                defined_locals_start..(defined_locals_start + defined_locals.stack_size),
            ),
            results_base_slot,
        })
    }

    /// Get a local slot.
    pub fn get_local(&self, index: u32) -> Option<&LocalSlot> {
        self.locals.get(index as usize)
    }

    /// Returns the address of the local at the given index.
    ///
    /// # Panics
    /// This function panics if the the index is not associated to a local.
    pub fn get_local_address<M: MacroAssembler>(
        &self,
        index: u32,
        masm: &mut M,
    ) -> (WasmValType, M::Address) {
        self.get_local(index)
            .map(|slot| (slot.ty, masm.local_address(slot)))
            .unwrap_or_else(|| panic!("Invalid local slot: {}", index))
    }

    fn compute_arg_slots<A: ABI>(sig: &ABISig) -> Result<(Locals, u32)> {
        // Go over the function ABI-signature and
        // calculate the stack slots.
        //
        //  for each parameter p; when p
        //
        //  Stack =>
        //      The slot offset is calculated from the ABIOperand offset
        //      relative the to the frame pointer (and its inclusions, e.g.
        //      return address).
        //
        //  Register =>
        //     The slot is calculated by accumulating into the `next_frame_size`
        //     the size + alignment of the type that the register is holding.
        //
        //  NOTE
        //      This implementation takes inspiration from SpiderMonkey's implementation
        //      to calculate local slots for function arguments
        //      (https://searchfox.org/mozilla-central/source/js/src/wasm/WasmBCFrame.cpp#83).
        //      The main difference is that SpiderMonkey's implementation
        //      doesn't append any sort of metadata to the locals regarding stack
        //      addressing mode (stack pointer or frame pointer), the offset is
        //      declared negative if the local belongs to a stack argument;
        //      that's enough to later calculate address of the local later on.
        //
        //      Winch appends an addressing mode to each slot, in the end
        //      we want positive addressing from the stack pointer
        //      for both locals and stack arguments.

        let arg_base_offset = <A as ABI>::arg_base_offset().into();
        let mut next_stack = 0u32;

        // Skip the results base param; if present, the [Frame] will create
        // a dedicated slot for it.
        let slots: Locals = sig
            .params_without_retptr()
            .into_iter()
            .map(|arg| Self::abi_arg_slot(&arg, &mut next_stack, arg_base_offset))
            .collect();

        Ok((slots, next_stack))
    }

    fn abi_arg_slot(arg: &ABIOperand, next_stack: &mut u32, arg_base_offset: u32) -> LocalSlot {
        match arg {
            // Create a local slot, for input register spilling,
            // with type-size aligned access.
            ABIOperand::Reg { ty, size, .. } => {
                *next_stack = align_to(*next_stack, *size) + *size;
                LocalSlot::new(*ty, *next_stack)
            }
            // Create a local slot, with an offset from the arguments base in
            // the stack; which is the frame pointer + return address.
            ABIOperand::Stack { ty, offset, .. } => {
                LocalSlot::stack_arg(*ty, offset + arg_base_offset)
            }
        }
    }
}
