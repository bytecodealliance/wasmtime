use crate::cursor::{Cursor, FuncCursor};
use crate::flowgraph;
use crate::ir::{self, Inst};

pub fn run<T>(
    backend: &T,
    func: &mut ir::Function,
    cfg: &mut flowgraph::ControlFlowGraph,
    constructor_legalize: fn(&mut LegalizeContext<'_, T>, Inst) -> Option<Inst>,
) {
    let mut cx = LegalizeContext {
        backend,
        pos: FuncCursor::new(func),
        cfg,
        replace: None,
    };
    let func_begin = cx.pos.position();
    cx.pos.set_position(func_begin);
    while let Some(_block) = cx.pos.next_block() {
        while let Some(inst) = cx.pos.next_inst() {
            cx.replace = Some(inst);
            match constructor_legalize(&mut cx, inst) {
                Some(_) => {}
                None => {}
            }
        }
    }
}

pub struct LegalizeContext<'a, T> {
    pub backend: &'a T,
    pub pos: FuncCursor<'a>,
    pub cfg: &'a mut flowgraph::ControlFlowGraph,
    pub replace: Option<Inst>,
}

/// Generate common methods for the legalization trait on `LegalizeContext`.
#[macro_export]
macro_rules! isle_common_legalizer_methods {
    () => {
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

        fn value_list_slice(&mut self, list: ValueList) -> ValueSlice {
            (list, 0)
        }

        fn value_slice_unwrap(&mut self, slice: ValueSlice) -> Option<(Value, ValueSlice)> {
            let (list, off) = slice;
            if let Some(val) = list.get(off, &self.pos.func.dfg.value_lists) {
                Some((val, (list, off + 1)))
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
    };
}
