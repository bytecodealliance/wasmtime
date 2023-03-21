use crate::{
    abi::{align_to, ABIArg, ABIResult, ABI},
    codegen::call::calculate_frame_adjustment,
    masm::{CallKind, MacroAssembler, OperandSize, RegImm},
    reg::Reg,
};
use std::mem;
use wasmparser::{FuncType, ValType};

pub(crate) struct Trampoline<'a, A, M>
where
    A: ABI,
    M: MacroAssembler,
{
    masm: &'a mut M,
    abi: &'a A,
    scratch: Reg,
    argv: Reg,
}

impl<'a, A, M> Trampoline<'a, A, M>
where
    A: ABI,
    M: MacroAssembler,
{
    pub fn new(masm: &'a mut M, abi: &'a A, scratch: Reg, argv: Reg) -> Self {
        Self {
            masm,
            abi,
            scratch,
            argv,
        }
    }

    pub fn emit_host_to_wasm(&mut self, ty: &FuncType) {
        // The host to wasm trampoline is currently hard coded (see vmcontext.rs in the
        // wasmtime-runtime crate, VMTrampoline)
        let trampoline_ty = FuncType::new(
            vec![ValType::I64, ValType::I64, ValType::I64, ValType::I64],
            vec![],
        );
        let trampoline_sig = self.abi.sig(&trampoline_ty);

        let trampoline_arg_size = 32;

        let callee_sig = self.abi.sig(ty);

        let val_ptr = if let ABIArg::Reg { reg, ty: _ty } = &trampoline_sig.params[3] {
            Ok(RegImm::reg(*reg))
        } else {
            Err(anyhow::anyhow!(""))
        }
        .unwrap();

        self.masm.prologue();

        let mut offsets: [u32; 4] = [0; 4];

        trampoline_sig
            .params
            .iter()
            .enumerate()
            .for_each(|(i, param)| {
                if let ABIArg::Reg { reg, ty: _ty } = param {
                    let offset = self.masm.push(*reg);
                    offsets[i] = offset;
                }
            });

        // How big of an operand do we need here? My stub signature has an I32 but is that right?
        self.masm.mov(
            val_ptr,
            RegImm::reg(self.scratch),
            crate::masm::OperandSize::S64,
        );

        let delta = calculate_frame_adjustment(
            self.masm.sp_offset(),
            self.abi.arg_base_offset() as u32,
            self.abi.call_stack_align() as u32,
        );

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
                    self.masm.address_from_reg(self.scratch, value_offset),
                    *reg,
                    (*ty).into(),
                ),
                ABIArg::Stack { offset, ty } => {
                    self.masm.load(
                        self.masm.address_from_reg(self.scratch, value_offset),
                        self.argv,
                        (*ty).into(),
                    );
                    self.masm.store(
                        RegImm::reg(self.argv),
                        self.masm.address_from_sp(24 - *offset),
                        (*ty).into(),
                    );
                }
            }
        });

        // Move the function pointer from it's stack location into a scratch register
        self.masm.load(
            self.masm
                .address_from_sp(self.masm.sp_offset() - offsets[2]),
            self.scratch,
            OperandSize::S64,
        );

        // Call the function that was passed into the trampoline
        self.masm.call(CallKind::Indirect(self.scratch));

        self.masm.free_stack(total_arg_stack_space);

        // Move the val ptr back into the scratch register so we can load the return values
        self.masm.load(
            self.masm.address_from_sp(trampoline_arg_size - offsets[3]),
            self.scratch,
            OperandSize::S64,
        );

        // Move the return values into the value ptr
        // Only doing a single return value for now
        let ABIResult::Reg { reg, ty } = &callee_sig.result;
        self.masm.store(
            RegImm::reg(*reg),
            self.masm.address_from_reg(self.scratch, 0),
            (*ty).unwrap().into(),
        );

        self.masm.epilogue(trampoline_arg_size);
    }
}
