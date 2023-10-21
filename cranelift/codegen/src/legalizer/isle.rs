use crate::cursor::{Cursor, FuncCursor};
use crate::flowgraph::ControlFlowGraph;
use crate::ir::{self, Inst, InstBuilder};
use crate::trace;

pub fn run<T>(
    backend: &T,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
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
    pub cfg: &'a mut ControlFlowGraph,
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

        fn expand_trapz(&mut self, arg: Value, cc: &ir::TrapCode) -> Inst {
            crate::legalizer::isle::expand_cond_trap(
                self.replace.unwrap(),
                self.pos.func,
                self.cfg,
                ir::Opcode::Trapz,
                arg,
                *cc,
            )
        }

        fn expand_trapnz(&mut self, arg: Value, cc: &ir::TrapCode) -> Inst {
            crate::legalizer::isle::expand_cond_trap(
                self.replace.unwrap(),
                self.pos.func,
                self.cfg,
                ir::Opcode::Trapnz,
                arg,
                *cc,
            )
        }

        fn expand_resumable_trapnz(&mut self, arg: Value, cc: &ir::TrapCode) -> Inst {
            crate::legalizer::isle::expand_cond_trap(
                self.replace.unwrap(),
                self.pos.func,
                self.cfg,
                ir::Opcode::ResumableTrapnz,
                arg,
                *cc,
            )
        }
    };
}

/// Custom expansion for conditional trap instructions.
pub fn expand_cond_trap(
    inst: ir::Inst,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    opcode: ir::Opcode,
    arg: ir::Value,
    code: ir::TrapCode,
) -> Inst {
    trace!(
        "expanding conditional trap: {:?}: {}",
        inst,
        func.dfg.display_inst(inst)
    );

    // Parse the instruction.
    let trapz = match opcode {
        ir::Opcode::Trapz => true,
        ir::Opcode::Trapnz | ir::Opcode::ResumableTrapnz => false,
        _ => panic!("Expected cond trap: {}", func.dfg.display_inst(inst)),
    };

    // Split the block after `inst`:
    //
    //     trapnz arg
    //     ..
    //
    // Becomes:
    //
    //     brif arg, new_block_trap, new_block_resume
    //
    //   new_block_trap:
    //     trap
    //
    //   new_block_resume:
    //     ..
    let old_block = func
        .layout
        .inst_block(inst)
        .expect("Instruction not in layout.");
    let new_block_trap = func.dfg.make_block();
    let new_block_resume = func.dfg.make_block();

    // Trapping is a rare event, mark the trapping block as cold.
    func.layout.set_cold(new_block_trap);

    // Replace trap instruction by the inverted condition.
    if trapz {
        func.dfg
            .replace(inst)
            .brif(arg, new_block_resume, &[], new_block_trap, &[]);
    } else {
        func.dfg
            .replace(inst)
            .brif(arg, new_block_trap, &[], new_block_resume, &[]);
    }

    // Insert the new label and the unconditional trap terminator.
    let mut pos = FuncCursor::new(func).after_inst(inst);
    pos.use_srcloc(inst);
    pos.insert_block(new_block_trap);

    let inst = match opcode {
        ir::Opcode::Trapz | ir::Opcode::Trapnz => pos.ins().trap(code),
        ir::Opcode::ResumableTrapnz => {
            pos.ins().resumable_trap(code);
            pos.ins().jump(new_block_resume, &[])
        }
        _ => unreachable!(),
    };

    // Insert the new label and resume the execution when the trap fails.
    pos.insert_block(new_block_resume);

    // Finally update the CFG.
    cfg.recompute_block(pos.func, old_block);
    cfg.recompute_block(pos.func, new_block_resume);
    cfg.recompute_block(pos.func, new_block_trap);

    inst
}
