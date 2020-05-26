use alloc::vec::Vec;
use crate::ir::{ArgumentExtension, StackSlot};
use crate::ir::types::Type;
use crate::isa::spirv::inst::Inst;
use crate::machinst::abi::ABIBody;
use crate::settings;
use regalloc::*;

pub(crate) struct SpirvABIBody {
    
}

impl SpirvABIBody {
    pub(crate) fn new() -> Self {
        Self {

        }
    }
}

impl ABIBody for SpirvABIBody {
    type I = Inst;

    fn flags(&self) -> &settings::Flags {
        unimplemented!()
    }

    fn num_args(&self) -> usize {
        unimplemented!()
    }

    fn num_retvals(&self) -> usize {
        unimplemented!()
    }

    fn num_stackslots(&self) -> usize {
        unimplemented!()
    }

    fn liveins(&self) -> Set<RealReg> {
        unimplemented!()

    }

    fn liveouts(&self) -> Set<RealReg> {
        unimplemented!()

    }

    fn gen_copy_arg_to_reg(&self, idx: usize, to_reg: Writable<Reg>) -> Inst {
        unimplemented!()

    }

    fn gen_copy_reg_to_retval(
        &self,
        idx: usize,
        from_reg: Writable<Reg>,
        ext: ArgumentExtension,
    ) -> Vec<Inst> {
        unimplemented!()

    }

    fn gen_ret(&self) -> Inst {
        unimplemented!()
    }

    fn gen_epilogue_placeholder(&self) -> Inst {
        unimplemented!()
    }

    fn set_num_spillslots(&mut self, slots: usize) {
        unimplemented!()
    }

    fn set_clobbered(&mut self, clobbered: Set<Writable<RealReg>>) {
        unimplemented!()
    }

    fn stackslot_addr(&self, _slot: StackSlot, _offset: u32, _into_reg: Writable<Reg>) -> Inst {
        unimplemented!()
    }

    fn load_stackslot(
        &self,
        _slot: StackSlot,
        _offset: u32,
        _ty: Type,
        _into_reg: Writable<Reg>,
    ) -> Inst {
        unimplemented!("load_stackslot")
    }

    fn store_stackslot(&self, _slot: StackSlot, _offset: u32, _ty: Type, _from_reg: Reg) -> Inst {
        unimplemented!("store_stackslot")
    }

    fn load_spillslot(&self, _slot: SpillSlot, _ty: Type, _into_reg: Writable<Reg>) -> Inst {
        unimplemented!("load_spillslot")
    }

    fn store_spillslot(&self, _slot: SpillSlot, _ty: Type, _from_reg: Reg) -> Inst {
        unimplemented!("store_spillslot")
    }

    fn gen_prologue(&mut self) -> Vec<Inst> {
        unimplemented!()
    }

    fn gen_epilogue(&self) -> Vec<Inst> {
        unimplemented!()
    }

    fn frame_size(&self) -> u32 {
        unimplemented!()
    }

    fn get_spillslot_size(&self, rc: RegClass, ty: Type) -> u32 {
        unimplemented!()
    }

    fn gen_spill(&self, _to_slot: SpillSlot, _from_reg: RealReg, _ty: Type) -> Inst {
        unimplemented!()
    }

    fn gen_reload(&self, _to_reg: Writable<RealReg>, _from_slot: SpillSlot, _ty: Type) -> Inst {
        unimplemented!()
    }
}
