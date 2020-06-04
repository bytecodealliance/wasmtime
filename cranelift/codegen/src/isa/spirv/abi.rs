use alloc::vec::Vec;
use crate::ir::{ArgumentExtension, StackSlot};
use crate::ir::types::Type;
use crate::isa::spirv::inst::Inst;
use crate::machinst::abi::ABIBody;
use crate::settings;
use crate::ir::Function;
use regalloc::*;
use spirv_headers::Op;

pub(crate) struct SpirvABIBody {
    flags: settings::Flags,
}

impl SpirvABIBody {
    pub(crate) fn new(func: &Function, flags: settings::Flags) -> Self {
        Self {
            flags
        }
    }
}

impl ABIBody for SpirvABIBody {
    type I = Inst;

    fn flags(&self) -> &settings::Flags {
        &self.flags
    }

    fn num_args(&self) -> usize {
        0
    }

    fn num_retvals(&self) -> usize {
        0
    }

    fn num_stackslots(&self) -> usize {
        0
    }

    fn liveins(&self) -> Set<RealReg> {
        Set::empty()
    }

    fn liveouts(&self) -> Set<RealReg> {
        Set::empty()
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
        Vec::new()
    }

    fn gen_ret(&self) -> Inst {
        Inst::new(Op::Return, None, None, vec![])
    }

    fn gen_epilogue_placeholder(&self) -> Inst {
        unimplemented!()
    }

    fn set_num_spillslots(&mut self, slots: usize) {
        
    }

    fn set_clobbered(&mut self, clobbered: Set<Writable<RealReg>>) {
        
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
        Vec::new()
    }

    fn gen_epilogue(&self) -> Vec<Inst> {
        Vec::new()
    }

    fn frame_size(&self) -> u32 {
        0
    }

    fn get_spillslot_size(&self, rc: RegClass, ty: Type) -> u32 {
        0
    }

    fn gen_spill(&self, _to_slot: SpillSlot, _from_reg: RealReg, _ty: Type) -> Inst {
        unimplemented!()
    }

    fn gen_reload(&self, _to_reg: Writable<RealReg>, _from_slot: SpillSlot, _ty: Type) -> Inst {
        unimplemented!()
    }
}
