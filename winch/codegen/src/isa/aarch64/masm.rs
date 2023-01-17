use crate::{
    abi::{addressing_mode::Address, local::LocalSlot},
    isa::reg::Reg,
    masm::{MacroAssembler as Masm, OperandSize, RegImm},
};
use cranelift_codegen::{Final, MachBufferFinalized};

#[derive(Default)]
pub(crate) struct MacroAssembler;

impl Masm for MacroAssembler {
    fn prologue(&mut self) {
        todo!()
    }

    fn epilogue(&mut self, _locals_size: u32) {
        todo!()
    }

    fn reserve_stack(&mut self, _bytes: u32) {
        todo!()
    }

    fn local_address(&mut self, _local: &LocalSlot) -> Address {
        todo!()
    }

    fn store(&mut self, _src: RegImm, _dst: Address, _size: OperandSize) {
        todo!()
    }

    fn load(&mut self, _src: Address, _dst: Reg, _size: OperandSize) {}

    fn sp_offset(&mut self) -> u32 {
        0u32
    }

    fn finalize(self) -> MachBufferFinalized<Final> {
        todo!()
    }

    fn mov(&mut self, _src: RegImm, _dst: RegImm, _size: OperandSize) {
        todo!()
    }

    fn add(&mut self, _dst: RegImm, __lhs: RegImm, __rhs: RegImm, _size: OperandSize) {
        todo!()
    }

    fn zero(&mut self, _reg: Reg) {
        todo!()
    }

    fn push(&mut self, _reg: Reg) -> u32 {
        todo!()
    }
}
