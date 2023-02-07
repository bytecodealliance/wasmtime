use crate::{
    abi::{align_to, calculate_frame_adjustment, ABIArg, ABIResult, ABI},
    isa::CallingConvention,
    masm::{CalleeKind, MacroAssembler, OperandSize, RegImm},
    reg::Reg,
};
use smallvec::SmallVec;
use std::mem;
use wasmparser::{FuncType, ValType};

/// A trampoline to provide interopt between different calling
/// conventions.
pub(crate) struct Trampoline<'a, A, M>
where
    A: ABI,
    M: MacroAssembler,
{
    /// The macro assembler.
    masm: &'a mut M,
    /// The ABI.
    abi: &'a A,
    /// The main scratch register for the current architecture. It is
    /// not allocatable for the callee.
    scratch_reg: Reg,
    /// A second scratch register. This will be allocatable for the
    /// callee, so it can only be used after the callee-saved
    /// registers are on the stack.
    alloc_scratch_reg: Reg,
    /// Registers to be saved as part of the trampoline's prologue and epilogue.
    callee_saved_regs: SmallVec<[Reg; 9]>,
    /// The calling convention used by the trampoline,
    /// which is the Wasmtime variant of the system ABI's
    /// calling convention.
    call_conv: &'a CallingConvention,
}

impl<'a, A, M> Trampoline<'a, A, M>
where
    A: ABI,
    M: MacroAssembler,
{
    /// Create a new trampoline.
    pub fn new(
        masm: &'a mut M,
        abi: &'a A,
        scratch_reg: Reg,
        alloc_scratch_reg: Reg,
        call_conv: &'a CallingConvention,
    ) -> Self {
        Self {
            masm,
            abi,
            scratch_reg,
            alloc_scratch_reg,
            callee_saved_regs: <A as ABI>::callee_saved_regs(call_conv),
            call_conv,
        }
    }

    /// Emit the host to wasm trampoline.
    pub fn emit_host_to_wasm(&mut self, ty: &FuncType) {
        // The host to wasm trampoline is currently hard coded (see vmcontext.rs
        // in the wasmtime-runtime crate, `VMArrayCallFunction`).  The first two
        // parameters are VMContexts (not used at this time). The third
        // parameter is the function pointer to call.  The fourth parameter is
        // an address to storage space for both the return value and the
        // arguments to the function.
        let trampoline_ty = FuncType::new(
            vec![ValType::I64, ValType::I64, ValType::I64, ValType::I64],
            vec![],
        );

        let trampoline_sig = self.abi.sig(&trampoline_ty, self.call_conv);

        // Hard-coding the size in bytes of the trampoline arguments
        // since it's static, based on the current signature we should
        // always have 4 arguments, each of which is 8 bytes.
        let trampoline_arg_size = 32;

        let callee_sig = self.abi.sig(ty, &CallingConvention::Default);

        let val_ptr = if let ABIArg::Reg { reg, ty: _ty } = &trampoline_sig.params[3] {
            Ok(RegImm::reg(*reg))
        } else {
            Err(anyhow::anyhow!("Expected the val ptr to be in a register"))
        }
        .unwrap();

        self.prologue();

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

        self.masm
            .mov(val_ptr, RegImm::reg(self.scratch_reg), OperandSize::S64);

        // How much we need to adjust the stack pointer by to account
        // for the alignment required by the ISA.
        let delta = calculate_frame_adjustment(
            self.masm.sp_offset(),
            self.abi.arg_base_offset() as u32,
            self.abi.call_stack_align() as u32,
        );

        // The total amount of stack space we need to reserve for the
        // arguments.
        let total_arg_stack_space = align_to(
            callee_sig.stack_bytes + delta,
            self.abi.call_stack_align() as u32,
        );

        self.masm.reserve_stack(total_arg_stack_space);

        // The max size a value can be when reading from the params
        // memory location.
        let value_size = mem::size_of::<u128>();

        callee_sig.params.iter().enumerate().for_each(|(i, param)| {
            let value_offset = (i * value_size) as u32;

            match param {
                ABIArg::Reg { reg, ty } => self.masm.load(
                    self.masm.address_from_reg(self.scratch_reg, value_offset),
                    *reg,
                    (*ty).into(),
                ),
                ABIArg::Stack { offset, ty } => {
                    self.masm.load(
                        self.masm.address_from_reg(self.scratch_reg, value_offset),
                        self.alloc_scratch_reg,
                        (*ty).into(),
                    );
                    self.masm.store(
                        RegImm::reg(self.alloc_scratch_reg),
                        self.masm.address_at_sp(*offset),
                        (*ty).into(),
                    );
                }
            }
        });

        // Move the function pointer from it's stack location into a
        // scratch register.
        self.masm.load(
            self.masm.address_from_sp(func_ptr_offset),
            self.scratch_reg,
            OperandSize::S64,
        );

        // Call the function that was passed into the trampoline.
        self.masm.call(CalleeKind::Indirect(self.scratch_reg));

        self.masm.free_stack(total_arg_stack_space);

        // Move the val ptr back into the scratch register so we can
        // load the return values.
        self.masm.load(
            self.masm.address_from_sp(val_ptr_offset),
            self.scratch_reg,
            OperandSize::S64,
        );

        // Move the return values into the value ptr.  We are only
        // supporting a single return value at this time.
        let ABIResult::Reg { reg, ty } = &callee_sig.result;
        self.masm.store(
            RegImm::reg(*reg),
            self.masm.address_from_reg(self.scratch_reg, 0),
            (*ty).unwrap().into(),
        );
        self.epilogue(trampoline_arg_size);
    }

    /// The trampoline's prologue.
    fn prologue(&mut self) {
        self.masm.prologue();
        // Save any callee-saved registers.
        for r in &self.callee_saved_regs {
            self.masm.push(*r);
        }
    }

    /// The trampoline's epilogue.
    fn epilogue(&mut self, arg_size: u32) {
        // Free the stack space allocated by pushing the trampoline arguments.
        self.masm.free_stack(arg_size);
        // Restore the callee-saved registers.
        for r in self.callee_saved_regs.iter().rev() {
            self.masm.pop(*r);
        }
        self.masm.epilogue(0);
    }
}
