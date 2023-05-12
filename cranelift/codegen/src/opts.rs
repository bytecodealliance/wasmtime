//! Optimization driver using ISLE rewrite rules on an egraph.

use crate::egraph::{NewOrExistingInst, OptimizeCtx};
use crate::ir::condcodes;
pub use crate::ir::condcodes::{FloatCC, IntCC};
use crate::ir::dfg::ValueDef;
pub use crate::ir::immediates::{Ieee32, Ieee64, Imm64, Offset32, Uimm32, Uimm64, Uimm8, V128Imm};
pub use crate::ir::types::*;
pub use crate::ir::{
    dynamic_to_fixed, AtomicRmwOp, Block, BlockCall, Constant, DataFlowGraph, DynamicStackSlot,
    FuncRef, GlobalValue, Immediate, InstructionData, JumpTable, MemFlags, Opcode, StackSlot,
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

pub(crate) struct IsleContext<'a, 'b, 'c> {
    pub(crate) ctx: &'a mut OptimizeCtx<'b, 'c>,
}

pub(crate) struct InstDataEtorIter<'a, 'b, 'c> {
    stack: SmallVec<[Value; 8]>,
    _phantom1: PhantomData<&'a ()>,
    _phantom2: PhantomData<&'b ()>,
    _phantom3: PhantomData<&'c ()>,
}
impl<'a, 'b, 'c> InstDataEtorIter<'a, 'b, 'c> {
    fn new(root: Value) -> Self {
        debug_assert_ne!(root, Value::reserved_value());
        Self {
            stack: smallvec![root],
            _phantom1: PhantomData,
            _phantom2: PhantomData,
            _phantom3: PhantomData,
        }
    }
}

impl<'a, 'b, 'c> ContextIter for InstDataEtorIter<'a, 'b, 'c>
where
    'b: 'a,
    'c: 'b,
{
    type Context = IsleContext<'a, 'b, 'c>;
    type Output = (Type, InstructionData);

    fn next(&mut self, ctx: &mut IsleContext<'a, 'b, 'c>) -> Option<Self::Output> {
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
                    return Some((ty, ctx.ctx.func.dfg.insts[inst].clone()));
                }
                _ => {}
            }
        }
        None
    }
}

impl<'a, 'b, 'c> generated_code::Context for IsleContext<'a, 'b, 'c> {
    isle_common_prelude_methods!();

    type inst_data_etor_iter = InstDataEtorIter<'a, 'b, 'c>;

    fn inst_data_etor(&mut self, eclass: Value) -> InstDataEtorIter<'a, 'b, 'c> {
        InstDataEtorIter::new(eclass)
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

    fn splat64(&mut self, val: u64) -> Constant {
        let val = u128::from(val);
        let val = val | (val << 64);
        let imm = V128Imm(val.to_le_bytes());
        self.ctx.func.dfg.constants.insert(imm.into())
    }
}
