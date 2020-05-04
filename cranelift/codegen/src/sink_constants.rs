use crate::entity::SecondaryMap;
use crate::ir::{Function, Inst, InstructionData, Opcode, Type, ValueDef};

use smallvec::SmallVec;

use log::debug;

/// Sink constants in `func` to just before uses.
pub fn do_sink_constants(func: &mut Function) {
    // Const instructions to remove.
    let mut remove = SecondaryMap::with_default(false);
    let mut remove_list: SmallVec<[Inst; 16]> = SmallVec::new();
    // Insertions: (before_this_inst, replace_this_arg, const_data, type).
    let mut insert: SmallVec<[(Inst, usize, InstructionData, Type); 16]> = SmallVec::new();

    debug!("do_sink_constants: function {:?}", func);

    for bb in func.layout.blocks() {
        for inst in func.layout.block_insts(bb) {
            if is_const(func, inst) && !remove[inst] {
                remove[inst] = true;
                remove_list.push(inst);
            }
            for (i, arg) in func.dfg.inst_args(inst).iter().enumerate() {
                let v = func.dfg.resolve_aliases(*arg);
                if let ValueDef::Result(src_inst, _) = func.dfg.value_def(v) {
                    if is_const(func, src_inst) {
                        let data = func.dfg[src_inst].clone();
                        let ty = func.dfg.ctrl_typevar(src_inst);
                        insert.push((inst, i, data, ty));
                    }
                }
            }
        }
    }

    for (inst, arg, data, ty) in insert.into_iter() {
        let const_inst = func.dfg.make_inst(data);
        func.dfg.make_inst_results(const_inst, ty);
        func.layout.insert_inst(const_inst, inst);
        let const_val = func.dfg.inst_results(const_inst)[0];
        func.dfg.inst_args_mut(inst)[arg] = const_val;
    }
    for inst in remove_list.into_iter() {
        func.layout.remove_inst(inst);
    }

    debug!("do_sink_constants: resulting function {:?}", func);
}

fn is_const(func: &Function, inst: Inst) -> bool {
    match func.dfg[inst].opcode() {
        Opcode::Iconst | Opcode::Bconst | Opcode::F32const | Opcode::F64const | Opcode::Vconst => {
            true
        }
        _ => false,
    }
}
