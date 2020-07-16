use alloc::vec::Vec;
use crate::ir::{ArgumentExtension, ArgumentPurpose, StackSlot};
use crate::ir::types::Type;
use crate::isa::spirv::inst::Inst;
use crate::machinst::abi::ABIBody;
use crate::settings;
use crate::ir::Function;
use crate::isa::spirv::inst::EmitState;
use crate::binemit::Stackmap;

use regalloc::*;

use spirv_headers::Op;
use rspirv::dr::{Operand, Instruction};

use std::sync::atomic::{Ordering, AtomicU32};

struct SpirvReturn {
    id: u32,
    real_reg: RealReg,
}

pub(crate) struct SpirvABIBody {
    flags: settings::Flags,
    next_id: AtomicU32,
    global_values: Vec<Inst>,
    rets: Vec<SpirvReturn>,
}

fn spirv_real_reg(id: u32) -> Reg {
    use core::convert::TryInto;
    Reg::new_real(RegClass::I64, 0, id.try_into().unwrap())
}

impl SpirvABIBody {
    pub(crate) fn new(func: &Function, flags: settings::Flags) -> Self {
        // let mut args = vec![];
        // let mut next_int_arg = 0;
        // for param in &f.signature.params {
        //     match param.purpose {
        //         ir::ArgumentPurpose::VMContext if f.signature.call_conv.extends_baldrdash() => {
        //             // `VMContext` is `r14` in Baldrdash.
        //             args.push(ABIArg::Reg(regs::r14().to_real_reg()));
        //         }

        //         ir::ArgumentPurpose::Normal | ir::ArgumentPurpose::VMContext => {
        //             if in_int_reg(param.value_type) {
        //                 if let Some(reg) = get_intreg_for_arg_systemv(next_int_arg) {
        //                     args.push(ABIArg::Reg(reg.to_real_reg()));
        //                 } else {
        //                     unimplemented!("passing arg on the stack");
        //                 }
        //                 next_int_arg += 1;
        //             } else {
        //                 unimplemented!("non int normal register")
        //             }
        //         }

        //         _ => unimplemented!("other parameter purposes"),
        //     }
        // }

        let next_id = AtomicU32::new(1);
        let mut global_values = vec![];
        let mut rets = vec![];

        for ret in &func.signature.returns {
            match ret.purpose {
                ArgumentPurpose::Normal => {
                    match ret.value_type {
                        I32 => {
                            let id = next_id.fetch_add(1, Ordering::SeqCst);
                            //global_values.push(Inst::type_int(
                            //    Some(Writable::from_reg(spirv_real_reg(id))),
                            //    I32.lane_bits().into(),
                            //    0
                            //    ));

                            rets.push(SpirvReturn {
                                id,
                                real_reg: spirv_real_reg(id).to_real_reg() // jb-todo: index is wrong here 
                            });
                        },
                        _ => unimplemented!("Invalid return type"),
                    }
                },
                _ => unimplemented!("Not all types are supported")
            }
        }

        Self {
            flags,
            next_id,
            global_values,
            rets,
        }
    }
}

impl SpirvABIBody {
    fn id(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }
}

impl ABIBody for SpirvABIBody {
    type I = Inst;

    fn gen_retval_area_setup(&self) -> Option<Inst> {
        None
    }
    
    fn temp_needed(&self) -> bool {
        false
    }

    fn spillslots_to_stackmap(&self, _slots: &[SpillSlot], _state: &EmitState) -> Stackmap {
        unimplemented!("spillslots_to_stackmap")
    }

    fn stack_args_size(&self) -> u32 {
        unimplemented!("I need to be computed!")
    }

    fn init(&mut self, maybe_tmp: Option<Writable<Reg>>) {
    }

    fn flags(&self) -> &settings::Flags {
        &self.flags
    }

    fn num_args(&self) -> usize {0
        //self.sig.args.len()
    }

    fn num_retvals(&self) -> usize {
        //self.sig.rets.len()
        self.rets.len()
    }

    fn num_stackslots(&self) -> usize {
        //assert!(self.stackslots.len() == 0);
        0
    }

    fn liveins(&self) -> Set<RealReg> {
        let mut set: Set<RealReg> = Set::empty();
        // for &arg in &self.sig.args {
        //     if let ABIArg::Reg(r, _) = arg {
        //         set.insert(r);
        //     }
        // }
        set
    }

    fn liveouts(&self) -> Set<RealReg> {
        let mut set: Set<RealReg> = Set::empty();
        for ret in &self.rets {
            set.insert(ret.real_reg.clone());
        }
        set
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
        vec![
           // Inst::new(Op::Nop, None, None, vec![])
        ]
    }

    fn gen_ret(&self) -> Inst {
        if self.num_retvals() == 0 {
            Inst::new(Op::Return, None, None, vec![])
        } else {
            Inst::new(Op::ReturnValue, Some(self.rets[0].id), None, vec![])
        }
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
        self.global_values.clone()
    }

    fn frame_size(&self) -> u32 {
        0
    }

    fn get_spillslot_size(&self, rc: RegClass, ty: Type) -> u32 {
        0
    }

    fn gen_spill(&self, _to_slot: SpillSlot, _from_reg: RealReg, _ty: Option<Type>) -> Inst {
        unimplemented!()
    }

    fn gen_reload(&self, _to_reg: Writable<RealReg>, _from_slot: SpillSlot, _ty: Option<Type>) -> Inst {
        unimplemented!()
    }
}
