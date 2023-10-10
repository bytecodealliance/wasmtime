use crate::cursor::{Cursor, FuncCursor};
use crate::flowgraph;
use crate::ir;
use crate::isa::riscv64::Riscv64Backend;

// Used by ISLE
use crate::ir::condcodes::*;
use crate::ir::immediates::*;
use crate::ir::types::*;
use crate::ir::*;
use crate::machinst::isle::*;

#[allow(dead_code, unused_variables)]
mod generated {
    include!(concat!(env!("ISLE_DIR"), "/legalize_riscv64.rs"));
}

pub fn run(isa: &Riscv64Backend, func: &mut ir::Function, cfg: &mut flowgraph::ControlFlowGraph) {
    let mut cx = Legalize {
        isa,
        pos: FuncCursor::new(func),
        cfg,
        replace: None,
    };
    let func_begin = cx.pos.position();
    cx.pos.set_position(func_begin);
    while let Some(_block) = cx.pos.next_block() {
        while let Some(inst) = cx.pos.next_inst() {
            cx.replace = Some(inst);
            match generated::constructor_legalize(&mut cx, inst) {
                Some(_) => {}
                None => {}
            }
        }
    }
}

struct Legalize<'a> {
    isa: &'a Riscv64Backend,
    pos: FuncCursor<'a>,
    cfg: &'a mut flowgraph::ControlFlowGraph,
    replace: Option<Inst>,
}

impl generated::Context for Legalize<'_> {
    crate::isle_common_prelude_methods!();

    fn inst_data(&mut self, inst: Inst) -> InstructionData {
        self.pos.func.dfg.insts[inst]
    }

    fn ins(&mut self, ty: Type, data: &InstructionData) -> Inst {
        self.pos.ins().build(data.clone(), ty).0
    }

    fn replace(&mut self, ty: Type, data: &InstructionData) -> Inst {
        let ins = self.pos.func.dfg.replace(self.replace.unwrap());
        ins.build(data.clone(), ty).0
    }

    fn value_type(&mut self, val: Value) -> Type {
        self.pos.func.dfg.value_type(val)
    }

    fn first_result(&mut self, inst: Inst) -> Value {
        let results = self.pos.func.dfg.inst_results(inst);
        assert_eq!(results.len(), 1,);
        results[0]
    }

    fn result_type(&mut self, inst: Inst) -> Option<Type> {
        let results = self.pos.func.dfg.inst_results(inst);
        if results.len() == 1 {
            Some(self.value_type(results[0]))
        } else {
            None
        }
    }

    fn value_array_2_ctor(&mut self, arg0: Value, arg1: Value) -> ValueArray2 {
        [arg0, arg1]
    }

    fn value_array_3_ctor(&mut self, arg0: Value, arg1: Value, arg2: Value) -> ValueArray3 {
        [arg0, arg1, arg2]
    }
}
