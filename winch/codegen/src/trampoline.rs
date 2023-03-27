use crate::{
    abi::{align_to, ABIArg, ABIResult, ABI},
    codegen::call::calculate_frame_adjustment,
    masm::{CallKind, MacroAssembler, OperandSize, RegImm},
    reg::Reg,
};
use std::mem;
use wasmparser::{FuncType, ValType};

/// A trampoline to provide interopt between different calling conventions.
pub(crate) struct Trampoline<'a, A, M>
where
    A: ABI,
    M: MacroAssembler,
{
    /// The macro assembler.
    masm: &'a mut M,
    /// The ABI.
    abi: &'a A,
    /// A scratch register.
    scratch_1: Reg,
    /// A second scratch register. This can be allocatable for the callee.
    scratch_2: Reg,
}

impl<'a, A, M> Trampoline<'a, A, M>
where
    A: ABI,
    M: MacroAssembler,
{
    /// Create a new trampoline.
    pub fn new(masm: &'a mut M, abi: &'a A, scratch_1: Reg, scratch_2: Reg) -> Self {
        Self {
            masm,
            abi,
            scratch_1,
            scratch_2,
        }
    }

    /// Emit the host to wasm trampoline.
    pub fn emit_host_to_wasm(&mut self, ty: &FuncType) {
        // The host to wasm trampoline is currently hard coded (see vmcontext.rs in the
        // wasmtime-runtime crate, VMTrampoline)
        let trampoline_ty = FuncType::new(
            vec![ValType::I64, ValType::I64, ValType::I64, ValType::I64],
            vec![],
        );
        let trampoline_sig = self.abi.sig(&trampoline_ty);

        // Hard-coding the size in bytes of the trampoline arguments since it's static, based on
        // the current signature we should always have 4 arguments, each of which is 8 bytes
        let trampoline_arg_size = 32;

        let callee_sig = self.abi.sig(ty);

        let val_ptr = if let ABIArg::Reg { reg, ty: _ty } = &trampoline_sig.params[3] {
            Ok(RegImm::reg(*reg))
        } else {
            Err(anyhow::anyhow!("Expected the val ptr to be in a register"))
        }
        .unwrap();

        self.masm.prologue();

        let mut trampoline_arg_offsets: [u32; 4] = [0; 4];

        trampoline_sig
            .params
            .iter()
            .enumerate()
            .for_each(|(i, param)| {
                if let ABIArg::Reg { reg, ty: _ty } = param {
                    let offset = self.masm.push(*reg);
                    trampoline_arg_offsets[i] = offset;
                }
            });

        let val_ptr_offset = trampoline_arg_offsets[3];
        let func_ptr_offset = trampoline_arg_offsets[2];

        self.masm.mov(
            val_ptr,
            RegImm::reg(self.scratch_1),
            crate::masm::OperandSize::S64,
        );

        // How much we need to adjust the stack pointer by to account for the alignment
        // required by the ISA
        let delta = calculate_frame_adjustment(
            self.masm.sp_offset(),
            self.abi.arg_base_offset() as u32,
            self.abi.call_stack_align() as u32,
        );

        // The total amount of stack space we need to reserve for the arguments
        let total_arg_stack_space = align_to(
            callee_sig.stack_bytes + delta,
            self.abi.call_stack_align() as u32,
        );

        self.masm.reserve_stack(total_arg_stack_space);

        // The max size a value can be when reading from the params memory location
        let value_size = mem::size_of::<u128>();

        callee_sig.params.iter().enumerate().for_each(|(i, param)| {
            let value_offset = (i * value_size) as u32;

            match param {
                ABIArg::Reg { reg, ty } => self.masm.load(
                    self.masm.address_from_reg(self.scratch_1, value_offset),
                    *reg,
                    (*ty).into(),
                ),
                ABIArg::Stack { offset, ty } => {
                    self.masm.load(
                        self.masm.address_from_reg(self.scratch_1, value_offset),
                        self.scratch_2,
                        (*ty).into(),
                    );
                    self.masm.store(
                        RegImm::reg(self.scratch_2),
                        self.masm.address_from_sp(*offset),
                        (*ty).into(),
                    );
                }
            }
        });

        // Move the function pointer from it's stack location into a scratch register
        self.masm.load(
            self.masm
                .address_from_sp(self.masm.sp_offset() - func_ptr_offset),
            self.scratch_1,
            OperandSize::S64,
        );

        // Call the function that was passed into the trampoline
        self.masm.call(CallKind::Indirect(self.scratch_1));

        self.masm.free_stack(total_arg_stack_space);

        // Move the val ptr back into the scratch register so we can load the return values
        self.masm.load(
            self.masm.address_from_sp(self.masm.sp_offset() - val_ptr_offset),
            self.scratch_1,
            OperandSize::S64,
        );

        // Move the return values into the value ptr
        // Only doing a single return value for now
        let ABIResult::Reg { reg, ty } = &callee_sig.result;
        self.masm.store(
            RegImm::reg(*reg),
            self.masm.address_from_reg(self.scratch_1, 0),
            (*ty).unwrap().into(),
        );

        self.masm.epilogue(trampoline_arg_size);
    }
}
