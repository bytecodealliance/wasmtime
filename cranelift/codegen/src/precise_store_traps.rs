//! Precise-store-traps pass.
//!
//! On some instruction-set architectures, a store that crosses a page
//! boundary such that one of the pages would fault on a write can
//! sometimes still perform part of its memory update on the other
//! page. This becomes relevant, and problematic, when page
//! protections are load-bearing for Wasm VM semantics: see [this
//! issue] where a partially-out-of-bounds store in Wasm is currently
//! defined to perform no side-effect, but with a common lowering on
//! several ISAs and on some microarchitectures does actually perform
//! a "torn write".
//!
//! [this issue]: https://github.com/WebAssembly/design/issues/1490
//!
//! This pass performs a transform on CLIF that should avoid "torn
//! partially-faulting stores" by performing a throwaway *load* before
//! every store, of the same size and to the same address. This
//! throwaway load will fault if the store would have faulted due to
//! not-present pages (this still does nothing for
//! readonly-page-faults). Because the load happens before the store
//! in program order, if it faults, any ISA that guarantees precise
//! exceptions (all ISAs that we support) will ensure that the store
//! has no side-effects. (Microarchitecturally, once the faulting
//! instruction retires, the later not-yet-retired entries in the
//! store buffer will be flushed.)
//!
//! This is not on by default and remains an "experimental" option
//! while the Wasm spec resolves this issue, and serves for now to
//! allow collecting data on overheads and experimenting on affected
//! machines.

use crate::cursor::{Cursor, FuncCursor};
use crate::ir::types::*;
use crate::ir::*;

fn covering_type_for_value(func: &Function, value: Value) -> Type {
    match func.dfg.value_type(value).bits() {
        8 => I8,
        16 => I16,
        32 => I32,
        64 => I64,
        128 => I8X16,
        _ => unreachable!(),
    }
}

/// Perform the precise-store-traps transform on a function body.
pub fn do_precise_store_traps(func: &mut Function) {
    let mut pos = FuncCursor::new(func);
    while let Some(_block) = pos.next_block() {
        while let Some(inst) = pos.next_inst() {
            match &pos.func.dfg.insts[inst] {
                &InstructionData::StackStore {
                    opcode: _,
                    arg: data,
                    stack_slot,
                    offset,
                } => {
                    let ty = covering_type_for_value(&pos.func, data);
                    let _ = pos.ins().stack_load(ty, stack_slot, offset);
                }
                &InstructionData::DynamicStackStore {
                    opcode: _,
                    arg: data,
                    dynamic_stack_slot,
                } => {
                    let ty = covering_type_for_value(&pos.func, data);
                    let _ = pos.ins().dynamic_stack_load(ty, dynamic_stack_slot);
                }
                &InstructionData::Store {
                    opcode,
                    args,
                    flags,
                    offset,
                } => {
                    let (data, addr) = (args[0], args[1]);
                    let ty = match opcode {
                        Opcode::Store => covering_type_for_value(&pos.func, data),
                        Opcode::Istore8 => I8,
                        Opcode::Istore16 => I16,
                        Opcode::Istore32 => I32,
                        _ => unreachable!(),
                    };
                    let _ = pos.ins().load(ty, flags, addr, offset);
                }
                &InstructionData::StoreNoOffset {
                    opcode: Opcode::AtomicStore,
                    args,
                    flags,
                } => {
                    let (data, addr) = (args[0], args[1]);
                    let ty = covering_type_for_value(&pos.func, data);
                    let _ = pos.ins().atomic_load(ty, flags, addr);
                }
                &InstructionData::AtomicCas { .. } | &InstructionData::AtomicRmw { .. } => {
                    // Nothing: already does a read before the write.
                }
                &InstructionData::NullAry {
                    opcode: Opcode::Debugtrap,
                } => {
                    // Marked as `can_store`, but no concerns here.
                }
                inst => {
                    assert!(!inst.opcode().can_store());
                }
            }
        }
    }
}
