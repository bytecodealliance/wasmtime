//! Optimization driver using ISLE rewrite rules on an egraph.

use crate::egraph::{NewOrExistingInst, OptimizeCtx};
use crate::ir::condcodes;
pub use crate::ir::condcodes::{FloatCC, IntCC};
use crate::ir::dfg::ValueDef;
pub use crate::ir::immediates::{Ieee32, Ieee64, Imm64, Offset32, Uimm32, Uimm64, Uimm8};
pub use crate::ir::types::*;
pub use crate::ir::{
    dynamic_to_fixed, AtomicRmwOp, Block, Constant, DataFlowGraph, DynamicStackSlot, FuncRef,
    GlobalValue, Heap, HeapImm, Immediate, InstructionData, JumpTable, MemFlags, Opcode, StackSlot,
    Table, TrapCode, Type, Value,
};
use crate::isle_common_prelude_methods;
use crate::machinst::isle::*;
use crate::trace;
use cranelift_entity::packed_option::ReservedValue;
use smallvec::{smallvec, SmallVec};
use std::marker::PhantomData;

#[allow(dead_code)]
pub type Unit = ();
pub type Range = (usize, usize);
pub type ValueArray2 = [Value; 2];
pub type ValueArray3 = [Value; 3];

pub type ConstructorVec<T> = SmallVec<[T; 8]>;

pub(crate) mod generated_code;
use generated_code::ContextIter;

pub(crate) struct IsleContext<'a, 'b> {
    pub(crate) ctx: &'a mut OptimizeCtx<'b>,
}

pub(crate) struct InstDataEtorIter<'a, 'b> {
    stack: SmallVec<[Value; 8]>,
    _phantom1: PhantomData<&'a ()>,
    _phantom2: PhantomData<&'b ()>,
}
impl<'a, 'b> InstDataEtorIter<'a, 'b> {
    fn new(root: Value) -> Self {
        debug_assert_ne!(root, Value::reserved_value());
        Self {
            stack: smallvec![root],
            _phantom1: PhantomData,
            _phantom2: PhantomData,
        }
    }
}

impl<'a, 'b> ContextIter for InstDataEtorIter<'a, 'b>
where
    'b: 'a,
{
    type Context = IsleContext<'a, 'b>;
    type Output = (Type, InstructionData);

    fn next(&mut self, ctx: &mut IsleContext<'a, 'b>) -> Option<Self::Output> {
        while let Some(value) = self.stack.pop() {
            debug_assert_ne!(value, Value::reserved_value());
            let value = ctx.ctx.func.dfg.resolve_aliases(value);
            trace!("iter: value {:?}", value);
            match ctx.ctx.func.dfg.value_def(value) {
                ValueDef::Union(x, y) => {
                    debug_assert_ne!(x, Value::reserved_value());
                    debug_assert_ne!(y, Value::reserved_value());
                    trace!(" -> {}, {}", x, y);
                    self.stack.push(x);
                    self.stack.push(y);
                    continue;
                }
                ValueDef::Result(inst, _) if ctx.ctx.func.dfg.inst_results(inst).len() == 1 => {
                    let ty = ctx.ctx.func.dfg.value_type(value);
                    trace!(" -> value of type {}", ty);
                    return Some((ty, ctx.ctx.func.dfg[inst].clone()));
                }
                _ => {}
            }
        }
        None
    }
}

impl<'a, 'b> generated_code::Context for IsleContext<'a, 'b> {
    isle_common_prelude_methods!();

    type inst_data_etor_iter = InstDataEtorIter<'a, 'b>;

    fn inst_data_etor(&mut self, eclass: Value) -> Option<InstDataEtorIter<'a, 'b>> {
        Some(InstDataEtorIter::new(eclass))
    }

    fn make_inst_ctor(&mut self, ty: Type, op: &InstructionData) -> Value {
        let value = self
            .ctx
            .insert_pure_enode(NewOrExistingInst::New(op.clone(), ty));
        trace!("make_inst_ctor: {:?} -> {}", op, value);
        value
    }

    fn value_array_2_ctor(&mut self, arg0: Value, arg1: Value) -> ValueArray2 {
        [arg0, arg1]
    }

    fn value_array_3_ctor(&mut self, arg0: Value, arg1: Value, arg2: Value) -> ValueArray3 {
        [arg0, arg1, arg2]
    }

    #[inline]
    fn value_type(&mut self, val: Value) -> Type {
        self.ctx.func.dfg.value_type(val)
    }

    fn remat(&mut self, value: Value) -> Value {
        trace!("remat: {}", value);
        self.ctx.remat_values.insert(value);
        self.ctx.stats.remat += 1;
        value
    }

    fn subsume(&mut self, value: Value) -> Value {
        trace!("subsume: {}", value);
        self.ctx.subsume_values.insert(value);
        self.ctx.stats.subsume += 1;
        value
    }
}
