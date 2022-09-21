use crate::abi::{local::LocalSlot, ABIArg, ABISig, ABI};
use smallvec::SmallVec;
use wasmtime_environ::{FunctionBodyData, WasmFuncType, WasmType};

use crate::abi::{align_to, ty_size};
use crate::frame::Frame;
use anyhow::Result;

// TODO:
// SpiderMonkey's implementation uses 16; but we should measure if this is
// a good default.
type Locals = SmallVec<[LocalSlot; 16]>;

/// Per-function compilation environment
pub(crate) struct CompilationEnv<'x, 'a: 'x, A: ABI, C: Assembler> {
    /// A reference to the function body
    function: &'x mut FunctionBodyData<'a>,

    /// The stack frame handler for the current function
    frame: Frame,
    /// The local slots for the current function
    ///
    /// The locals get calculated when constructing a
    /// compilation env and are read-only from there
    locals: Locals,

    /// The ABI used in this compilation environment
    abi: A,

    /// The ABI-specific representation of the function signature
    sig: ABISig,
}

impl<'x, 'a: 'x, A: ABI, C: Assembler> CompilationEnv<'x, 'a, A, C> {
    /// Allocate a new compilation environment
    pub fn new(
        signature: &WasmFuncType,
        function: &'x mut FunctionBodyData<'a>,
        abi: A,
        asm: C,
    ) -> Result<Self> {
        let sig = abi.sig(&signature);
        let (locals, locals_size) = compute_local_slots(&sig, function, &abi)?;

        Ok(Self {
            abi,
            sig,
            locals,
            frame: Frame::new(locals_size),
            function,
            asm,
        })
    }

    // TODO Order
    // 1. Emit prologue
    //   1.1 Without any stack checks, the idea is to get to code emission and have an initial pass on the Assembler
    //   1.2 Register input spilling
    // 2. Function body
    // 3. Epilogue
    // 4. Stack checks
    /// Emit the function body to machine code
    pub fn emit(&mut self) -> Result<()> {
        self.emit_start().and(self.emit_body()).and(self.emit_end())
    }

    // Emit the usual function start instruction sequence
    // for the current function:
    // 1. Prologue
    // 2. Stack checks
    // 3. Stack allocation
    fn emit_start(&self) -> Result<()> {
        Ok(())
    }

    // 1. Perform input register spilling
    // 2. Emit machine code per instruction
    fn emit_body(&self) -> Result<()> {
        Ok(())
    }

    // Emit the usual function end instruction sequence
    fn emit_end(&self) -> Result<()> {
        Ok(())
    }
}

fn compute_local_slots<A: ABI>(
    sig: &ABISig,
    body_data: &mut FunctionBodyData,
    abi: &A,
) -> Result<(Locals, u64)> {
    // Go over the function ABI-signature and
    // calculate the stack slots
    //
    //  for each parameter p; when p
    //
    //  Stack =>
    //      The slot offset is calculated from the ABIArg offset
    //      relative the to the frame pointer (and its inclusions, e.g.
    //      return address)
    //
    //  Register =>
    //     The slot is calculated by accumulating into the `next_frame_size`
    //     the size + alignment of the type that the register is holding
    //
    //  Internal notes:
    //      SpiderMonkey's implementation doesn't append any sort of
    //      metadata to the locals regarding stack addressing mode
    //      (stack pointer or frame pointer), the offset is
    //      declared negative if the local belongs to a stack argument;
    //      that's enough to later calculate address of the local
    //
    //      Winch appends an addressing mode to each slot, in the end
    //      we want positive addressing for both locals and stack arguments

    let arg_base_offset: u64 = abi.arg_base_offset().into();
    let stack_align: u64 = abi.stack_align().into();
    let mut next_stack: u64 = 0;
    let mut slots: Locals = sig
        .params
        .iter()
        .map(|arg| abi_arg_to_slot(&arg, &mut next_stack, arg_base_offset))
        .collect();

    // Validate function-defined locals and calculate their stack slots
    append_local_slots(&mut slots, body_data, &mut next_stack)?;

    // Align the stack to the stack alignment specified by each
    // ISA ABI
    let locals_size = align_to(next_stack, stack_align);

    Ok((slots, locals_size))
}

fn abi_arg_to_slot(arg: &ABIArg, next_stack: &mut u64, arg_base_offset: u64) -> LocalSlot {
    match arg {
        // Create a local slot, for input register spilling,
        // with type-size aligned access
        ABIArg::Reg { ty, reg: _ } => {
            let ty_size = ty_size(&ty) as u64;
            *next_stack = align_to(*next_stack, ty_size) + ty_size;
            LocalSlot::new(*ty, *next_stack)
        }
        // Create a local slot, with an offset from the arguments base in
        // the stack; which is the frame pointer + return address
        ABIArg::Stack { ty, offset } => LocalSlot::stack_arg(*ty, offset + arg_base_offset),
    }
}

fn append_local_slots(
    slots: &mut Locals,
    body_data: &mut FunctionBodyData,
    next_stack: &mut u64,
) -> Result<()> {
    let mut reader = body_data.body.get_binary_reader();
    let validator = &mut body_data.validator;
    let local_count = reader.read_var_u32()?;

    for _ in 0..local_count {
        let position = reader.original_position();
        let count = reader.read_var_u32()?;
        let ty = reader.read_val_type()?;
        validator.define_locals(position, count, ty)?;

        let ty: WasmType = ty.try_into()?;
        for _ in 0..count {
            let ty_size = ty_size(&ty) as u64;
            *next_stack = align_to(*next_stack, ty_size);
            slots.push(LocalSlot::new(ty, *next_stack));
        }
    }

    Ok(())
}
