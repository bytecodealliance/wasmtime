use crate::cursor::{Cursor, CursorPosition, FuncCursor};
use crate::flowgraph::ControlFlowGraph;
use crate::ir::{self, Inst, InstBuilder};
use crate::isa::TargetIsa;
use crate::trace;

pub fn run<T>(
    backend: &T,
    func: &mut ir::Function,
    cfg: &mut ControlFlowGraph,
    constructor_legalize: fn(&mut LegalizeContext<'_, T>, Inst) -> Option<CursorPosition>,
) where
    T: TargetIsa,
{
    let pos = FuncCursor::new(func);
    let mut cx = LegalizeContext {
        backend,
        prev_position: pos.position(),
        pos,
        cfg,
        replace: None,
    };
    let func_begin = cx.pos.position();
    cx.pos.set_position(func_begin);
    while let Some(_block) = cx.pos.next_block() {
        cx.prev_position = cx.pos.position();
        while let Some(inst) = cx.pos.next_inst() {
            cx.replace = Some(inst);
            match constructor_legalize(&mut cx, inst) {
                Some(pos) => cx.pos.set_position(pos),
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
    pub prev_position: CursorPosition,
}

/// Generate common methods for the legalization trait on `LegalizeContext`.
#[macro_export]
macro_rules! isle_common_legalizer_methods {
    () => {
        crate::isle_common_prelude_methods!();

        fn inst_data(&mut self, inst: Inst) -> InstructionData {
            self.pos.func.dfg.insts[inst]
        }

        fn gv_data(&mut self, gv: GlobalValue) -> GlobalValueData {
            self.pos.func.global_values[gv].clone()
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

        fn expand_trapz(&mut self, arg: Value, cc: &ir::TrapCode) -> CursorPosition {
            crate::legalizer::isle::expand_cond_trap(
                self.replace.unwrap(),
                self.pos.func,
                self.cfg,
                ir::Opcode::Trapz,
                arg,
                *cc,
            )
        }

        fn expand_trapnz(&mut self, arg: Value, cc: &ir::TrapCode) -> CursorPosition {
            crate::legalizer::isle::expand_cond_trap(
                self.replace.unwrap(),
                self.pos.func,
                self.cfg,
                ir::Opcode::Trapnz,
                arg,
                *cc,
            )
        }

        fn expand_resumable_trapnz(&mut self, arg: Value, cc: &ir::TrapCode) -> CursorPosition {
            crate::legalizer::isle::expand_cond_trap(
                self.replace.unwrap(),
                self.pos.func,
                self.cfg,
                ir::Opcode::ResumableTrapnz,
                arg,
                *cc,
            )
        }

        fn pointer_type(&mut self) -> ir::Type {
            (self.backend as &dyn crate::isa::TargetIsa).pointer_type()
        }

        fn const_vector_scale(&mut self, ty: ir::Type) -> u64 {
            assert!(ty.bytes() <= 16);

            // Use a minimum of 128-bits for the base type.
            let base_bytes = std::cmp::max(ty.bytes(), 16);
            (self.backend.dynamic_vector_bytes(ty) / base_bytes).into()
        }

        fn update_inst_facts_with_gv(&mut self, gv: GlobalValue, val: Value) {
            if let Some(fact) = &self.pos.func.global_value_facts[gv] {
                if self.pos.func.dfg.facts[val].is_none() {
                    let fact = fact.clone();
                    self.pos.func.dfg.facts[val] = Some(fact);
                }
            }
        }

        fn replace_vmctx_addr(&mut self, global_value: ir::GlobalValue) -> CursorPosition {
            // Get the value representing the `vmctx` argument.
            let vmctx = self
                .pos
                .func
                .special_param(ir::ArgumentPurpose::VMContext)
                .expect("Missing vmctx parameter");
            let inst = self.replace.unwrap();

            // Replace the `global_value` instruction's value with an alias to the vmctx arg.
            let result = self.pos.func.dfg.first_result(inst);
            self.pos.func.dfg.clear_results(inst);
            self.pos.func.dfg.change_to_alias(result, vmctx);
            self.pos.func.layout.remove_inst(inst);

            // If there was a fact on the GV, then copy it to the vmctx arg
            // blockparam def.
            if let Some(fact) = &self.pos.func.global_value_facts[global_value] {
                if self.pos.func.dfg.facts[vmctx].is_none() {
                    let fact = fact.clone();
                    self.pos.func.dfg.facts[vmctx] = Some(fact);
                }
            }

            CursorPosition::At(inst)
        }

        fn cursor_position_at(&mut self, i: Inst) -> CursorPosition {
            CursorPosition::At(i)
        }

        fn prev_position(&mut self) -> CursorPosition {
            self.prev_position
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
) -> CursorPosition {
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

    match opcode {
        ir::Opcode::Trapz | ir::Opcode::Trapnz => {
            pos.ins().trap(code);
        }
        ir::Opcode::ResumableTrapnz => {
            pos.ins().resumable_trap(code);
            pos.ins().jump(new_block_resume, &[]);
        }
        _ => unreachable!(),
    }

    // Insert the new label and resume the execution when the trap fails.
    pos.insert_block(new_block_resume);

    // Finally update the CFG.
    cfg.recompute_block(pos.func, old_block);
    cfg.recompute_block(pos.func, new_block_resume);
    cfg.recompute_block(pos.func, new_block_trap);

    CursorPosition::Before(new_block_resume)
}
