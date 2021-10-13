//! Glue for working with `peepmatic`-generated peephole optimizers.

use crate::cursor::{Cursor, FuncCursor};
use crate::ir::{
    condcodes::IntCC,
    dfg::DataFlowGraph,
    entities::{Inst, Value},
    immediates::{Imm64, Uimm64},
    instructions::{InstructionData, Opcode},
    types, InstBuilder,
};
use crate::isa::TargetIsa;
use peepmatic_runtime::{
    cc::ConditionCode,
    instruction_set::InstructionSet,
    part::{Constant, Part},
    r#type::{BitWidth, Kind, Type},
    PeepholeOptimizations, PeepholeOptimizer,
};
use std::borrow::Cow;
use std::boxed::Box;
use std::convert::{TryFrom, TryInto};
use std::iter;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

peepmatic_traits::define_parse_and_typing_rules_for_operator! {
    Opcode {
        band => Band {
            parameters(iNN, iNN);
            result(iNN);
        }
        band_imm => BandImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        bconst => Bconst {
            immediates(b1);
            result(bNN);
        }
        bint => Bint {
            parameters(bNN);
            result(iNN);
        }
        bor => Bor {
            parameters(iNN, iNN);
            result(iNN);
        }
        bor_imm => BorImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        brnz => Brnz {
            parameters(bool_or_int);
            result(void);
        }
        brz => Brz {
            parameters(bool_or_int);
            result(void);
        }
        bxor => Bxor {
            parameters(iNN, iNN);
            result(iNN);
        }
        bxor_imm => BxorImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        iadd => Iadd {
            parameters(iNN, iNN);
            result(iNN);
        }
        iadd_imm => IaddImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        icmp => Icmp {
            immediates(cc);
            parameters(iNN, iNN);
            result(b1);
        }
        icmp_imm => IcmpImm {
            immediates(cc, iNN);
            parameters(iNN);
            result(b1);
        }
        iconst => Iconst {
            immediates(iNN);
            result(iNN);
        }
        ifcmp => Ifcmp {
            parameters(iNN, iNN);
            result(cpu_flags);
        }
        ifcmp_imm => IfcmpImm {
            immediates(iNN);
            parameters(iNN);
            result(cpu_flags);
        }
        imul => Imul {
            parameters(iNN, iNN);
            result(iNN);
        }
        imul_imm => ImulImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        ireduce => Ireduce {
            parameters(iNN);
            result(iMM);
            is_reduce(true);
        }
        irsub_imm => IrsubImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        ishl => Ishl {
            parameters(iNN, iNN);
            result(iNN);
        }
        ishl_imm => IshlImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        isub => Isub {
            parameters(iNN, iNN);
            result(iNN);
        }
        rotl => Rotl {
            parameters(iNN, iNN);
            result(iNN);
        }
        rotl_imm => RotlImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        rotr => Rotr {
            parameters(iNN, iNN);
            result(iNN);
        }
        rotr_imm => RotrImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        sdiv => Sdiv {
            parameters(iNN, iNN);
            result(iNN);
        }
        sdiv_imm => SdivImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        select => Select {
            parameters(bool_or_int, any_t, any_t);
            result(any_t);
        }
        sextend => Sextend {
            parameters(iNN);
            result(iMM);
            is_extend(true);
        }
        srem => Srem {
            parameters(iNN, iNN);
            result(iNN);
        }
        srem_imm => SremImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        sshr => Sshr {
            parameters(iNN, iNN);
            result(iNN);
        }
        sshr_imm => SshrImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        trapnz => Trapnz {
            parameters(bool_or_int);
            result(void);
        }
        trapz => Trapz {
            parameters(bool_or_int);
            result(void);
        }
        udiv => Udiv {
            parameters(iNN, iNN);
            result(iNN);
        }
        udiv_imm => UdivImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        uextend => Uextend {
            parameters(iNN);
            result(iMM);
            is_extend(true);
        }
        urem => Urem {
            parameters(iNN, iNN);
            result(iNN);
        }
        urem_imm => UremImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
        ushr => Ushr {
            parameters(iNN, iNN);
            result(iNN);
        }
        ushr_imm => UshrImm {
            immediates(iNN);
            parameters(iNN);
            result(iNN);
        }
    }
    parse_cfg(feature = "rebuild-peephole-optimizers");
}

/// Code required to rebuild Peepmatic-based peephole optimizers.
///
/// This module is used to scope imports and dependencies that are only required
/// for building peephole optimizers (as opposed to just using pre-built
/// peephole optimizers). This helps ensure that our regular builds using
/// pre-built peephole optimizers stay lean.
#[cfg(feature = "rebuild-peephole-optimizers")]
mod rebuild {
    use super::*;
    use alloc::vec::Vec;
    use std::fs;
    use std::path::Path;

    /// Rebuild the `preopt.peepmatic` peephole optimizer.
    ///
    /// Saves and overwrites the old `preopt.serialized` build and returns a
    /// copy of the result.
    pub fn rebuild_preopt() -> Vec<u8> {
        let codegen_path = Path::new(include_str!(concat!(
            env!("OUT_DIR"),
            "/CRANELIFT_CODEGEN_PATH"
        )));
        let source_path = codegen_path.join("src").join("preopt.peepmatic");

        let preopt = peepmatic::compile_file::<Opcode>(&source_path)
            .expect("failed to compile `src/preopt.peepmatic`");

        let serialized_path = codegen_path.join("src").join("preopt.serialized");
        preopt
            .serialize_to_file(&serialized_path)
            .expect("failed to serialize peephole optimizer to `src/preopt.serialized`");
        fs::read(&serialized_path).expect("failed to read `src/preopt.serialized`")
    }
}

/// Get the `preopt.peepmatic` peephole optimizer.
pub(crate) fn preopt<'a, 'b>(
    isa: &'b dyn TargetIsa,
) -> PeepholeOptimizer<'static, 'a, &'b dyn TargetIsa> {
    #[cfg(feature = "rebuild-peephole-optimizers")]
    fn get_serialized() -> Cow<'static, [u8]> {
        rebuild::rebuild_preopt().into()
    }

    #[cfg(not(feature = "rebuild-peephole-optimizers"))]
    fn get_serialized() -> Cow<'static, [u8]> {
        static SERIALIZED: &[u8] = include_bytes!("preopt.serialized");
        SERIALIZED.into()
    }

    // Once initialized, this must never be re-assigned. The initialized value
    // is semantically "static data" and is intentionally leaked for the whole
    // program's lifetime.
    static DESERIALIZED: AtomicPtr<PeepholeOptimizations<Opcode>> = AtomicPtr::new(ptr::null_mut());

    // If `DESERIALIZED` has already been initialized, then just use it.
    let ptr = DESERIALIZED.load(Ordering::SeqCst);
    if let Some(peep_opts) = unsafe { ptr.as_ref() } {
        return peep_opts.optimizer(isa);
    }

    // Otherwise, if `DESERIALIZED` hasn't been initialized, then we need to
    // deserialize the peephole optimizations and initialize it. However,
    // another thread could be doing the same thing concurrently, so there is a
    // race to see who initializes `DESERIALIZED` first, and we need to be
    // prepared to both win or lose that race.
    let peep_opts = PeepholeOptimizations::deserialize(&get_serialized())
        .expect("should always be able to deserialize `preopt.serialized`");
    let peep_opts = Box::into_raw(Box::new(peep_opts));

    // Only update `DESERIALIZE` if it is still null, attempting to perform the
    // one-time transition from null -> non-null.
    if DESERIALIZED
        .compare_and_swap(ptr::null_mut(), peep_opts, Ordering::SeqCst)
        .is_null()
    {
        // We won the race to initialize `DESERIALIZED`.
        debug_assert_eq!(DESERIALIZED.load(Ordering::SeqCst), peep_opts);
        let peep_opts = unsafe { &*peep_opts };
        return peep_opts.optimizer(isa);
    }

    // We lost the race to initialize `DESERIALIZED`. Drop our no-longer-needed
    // instance of `peep_opts` and get the pointer to the instance that won the
    // race.
    let _ = unsafe { Box::from_raw(peep_opts) };
    let peep_opts = DESERIALIZED.load(Ordering::SeqCst);
    let peep_opts = unsafe { peep_opts.as_ref().unwrap() };
    peep_opts.optimizer(isa)
}

/// Either a `Value` or an `Inst`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueOrInst {
    Value(Value),
    Inst(Inst),
}

impl ValueOrInst {
    /// Get the underlying `Value` if any.
    pub fn value(&self) -> Option<Value> {
        match *self {
            Self::Value(v) => Some(v),
            Self::Inst(_) => None,
        }
    }

    /// Get the underlying `Inst` if any.
    pub fn inst(&self) -> Option<Inst> {
        match *self {
            Self::Inst(i) => Some(i),
            Self::Value(_) => None,
        }
    }

    /// Unwrap the underlying `Value`, panicking if it is not a `Value.
    pub fn unwrap_value(&self) -> Value {
        self.value().unwrap()
    }

    /// Unwrap the underlying `Inst`, panicking if it is not a `Inst.
    pub fn unwrap_inst(&self) -> Inst {
        self.inst().unwrap()
    }

    /// Is this a `Value`?
    pub fn is_value(&self) -> bool {
        self.value().is_some()
    }

    /// Is this an `Inst`?
    pub fn is_inst(&self) -> bool {
        self.inst().is_some()
    }

    fn resolve_inst(&self, dfg: &DataFlowGraph) -> Option<Inst> {
        match *self {
            ValueOrInst::Inst(i) => Some(i),
            ValueOrInst::Value(v) => dfg.value_def(v).inst(),
        }
    }

    fn result_bit_width(&self, dfg: &DataFlowGraph) -> u8 {
        match *self {
            ValueOrInst::Value(v) => dfg.value_type(v).bits().try_into().unwrap(),
            ValueOrInst::Inst(inst) => {
                let result = dfg.first_result(inst);
                dfg.value_type(result).bits().try_into().unwrap()
            }
        }
    }

    fn to_constant(&self, pos: &mut FuncCursor) -> Option<Constant> {
        let inst = self.resolve_inst(&pos.func.dfg)?;
        match pos.func.dfg[inst] {
            InstructionData::UnaryImm {
                opcode: Opcode::Iconst,
                imm,
            } => {
                let width = self.result_bit_width(&pos.func.dfg).try_into().unwrap();
                let x: i64 = imm.into();
                Some(Constant::Int(x as u64, width))
            }
            InstructionData::UnaryBool {
                opcode: Opcode::Bconst,
                imm,
            } => {
                let width = self.result_bit_width(&pos.func.dfg).try_into().unwrap();
                Some(Constant::Bool(imm, width))
            }
            _ => None,
        }
    }
}

impl From<Value> for ValueOrInst {
    fn from(v: Value) -> ValueOrInst {
        ValueOrInst::Value(v)
    }
}

impl From<Inst> for ValueOrInst {
    fn from(i: Inst) -> ValueOrInst {
        ValueOrInst::Inst(i)
    }
}

/// Get the fixed bit width of `bit_width`, or if it is polymorphic, the bit
/// width of `root`.
fn bit_width(dfg: &DataFlowGraph, bit_width: BitWidth, root: Inst) -> u8 {
    bit_width.fixed_width().unwrap_or_else(|| {
        let tyvar = dfg.ctrl_typevar(root);
        let ty = dfg.compute_result_type(root, 0, tyvar).unwrap();
        u8::try_from(ty.bits()).unwrap()
    })
}

/// Convert the constant `c` into an instruction.
fn const_to_value<'a>(builder: impl InstBuilder<'a>, c: Constant, root: Inst) -> Value {
    match c {
        Constant::Bool(b, width) => {
            let width = bit_width(builder.data_flow_graph(), width, root);
            let ty = match width {
                1 => types::B1,
                8 => types::B8,
                16 => types::B16,
                32 => types::B32,
                64 => types::B64,
                128 => types::B128,
                _ => unreachable!(),
            };
            builder.bconst(ty, b)
        }
        Constant::Int(x, width) => {
            let width = bit_width(builder.data_flow_graph(), width, root);
            let ty = match width {
                8 => types::I8,
                16 => types::I16,
                32 => types::I32,
                64 => types::I64,
                128 => types::I128,
                _ => unreachable!(),
            };
            builder.iconst(ty, x as i64)
        }
    }
}

fn part_to_value(pos: &mut FuncCursor, root: Inst, part: Part<ValueOrInst>) -> Option<Value> {
    match part {
        Part::Instruction(ValueOrInst::Inst(inst)) => {
            pos.func.dfg.inst_results(inst).first().copied()
        }
        Part::Instruction(ValueOrInst::Value(v)) => Some(v),
        Part::Constant(c) => Some(const_to_value(pos.ins(), c, root)),
        Part::ConditionCode(_) => None,
    }
}

impl TryFrom<Constant> for Imm64 {
    type Error = &'static str;

    fn try_from(c: Constant) -> Result<Self, Self::Error> {
        match c {
            Constant::Int(x, _) => Ok(Imm64::from(x as i64)),
            Constant::Bool(..) => Err("cannot create Imm64 from Constant::Bool"),
        }
    }
}

impl Into<Constant> for Imm64 {
    #[inline]
    fn into(self) -> Constant {
        let x: i64 = self.into();
        Constant::Int(x as _, BitWidth::SixtyFour)
    }
}

impl Into<Part<ValueOrInst>> for Imm64 {
    #[inline]
    fn into(self) -> Part<ValueOrInst> {
        let c: Constant = self.into();
        c.into()
    }
}

fn part_to_imm64(pos: &mut FuncCursor, part: Part<ValueOrInst>) -> Imm64 {
    return match part {
        Part::Instruction(x) => match x.to_constant(pos).unwrap_or_else(|| cannot_convert()) {
            Constant::Int(x, _) => (x as i64).into(),
            Constant::Bool(..) => cannot_convert(),
        },
        Part::Constant(Constant::Int(x, _)) => (x as i64).into(),
        Part::ConditionCode(_) | Part::Constant(Constant::Bool(..)) => cannot_convert(),
    };

    #[inline(never)]
    #[cold]
    fn cannot_convert() -> ! {
        panic!("cannot convert part into `Imm64`")
    }
}

impl Into<Constant> for Uimm64 {
    #[inline]
    fn into(self) -> Constant {
        let x: u64 = self.into();
        Constant::Int(x, BitWidth::SixtyFour)
    }
}

impl Into<Part<ValueOrInst>> for Uimm64 {
    #[inline]
    fn into(self) -> Part<ValueOrInst> {
        let c: Constant = self.into();
        c.into()
    }
}

fn peepmatic_to_intcc(cc: ConditionCode) -> IntCC {
    match cc {
        ConditionCode::Eq => IntCC::Equal,
        ConditionCode::Ne => IntCC::NotEqual,
        ConditionCode::Slt => IntCC::SignedLessThan,
        ConditionCode::Sle => IntCC::SignedGreaterThanOrEqual,
        ConditionCode::Sgt => IntCC::SignedGreaterThan,
        ConditionCode::Sge => IntCC::SignedLessThanOrEqual,
        ConditionCode::Ult => IntCC::UnsignedLessThan,
        ConditionCode::Uge => IntCC::UnsignedGreaterThanOrEqual,
        ConditionCode::Ugt => IntCC::UnsignedGreaterThan,
        ConditionCode::Ule => IntCC::UnsignedLessThanOrEqual,
        ConditionCode::Of => IntCC::Overflow,
        ConditionCode::Nof => IntCC::NotOverflow,
    }
}

fn intcc_to_peepmatic(cc: IntCC) -> ConditionCode {
    match cc {
        IntCC::Equal => ConditionCode::Eq,
        IntCC::NotEqual => ConditionCode::Ne,
        IntCC::SignedLessThan => ConditionCode::Slt,
        IntCC::SignedGreaterThanOrEqual => ConditionCode::Sle,
        IntCC::SignedGreaterThan => ConditionCode::Sgt,
        IntCC::SignedLessThanOrEqual => ConditionCode::Sge,
        IntCC::UnsignedLessThan => ConditionCode::Ult,
        IntCC::UnsignedGreaterThanOrEqual => ConditionCode::Uge,
        IntCC::UnsignedGreaterThan => ConditionCode::Ugt,
        IntCC::UnsignedLessThanOrEqual => ConditionCode::Ule,
        IntCC::Overflow => ConditionCode::Of,
        IntCC::NotOverflow => ConditionCode::Nof,
    }
}

fn peepmatic_ty_to_ir_ty(ty: Type, dfg: &DataFlowGraph, root: Inst) -> types::Type {
    match (ty.kind, bit_width(dfg, ty.bit_width, root)) {
        (Kind::Int, 8) => types::I8,
        (Kind::Int, 16) => types::I16,
        (Kind::Int, 32) => types::I32,
        (Kind::Int, 64) => types::I64,
        (Kind::Int, 128) => types::I128,
        (Kind::Bool, 1) => types::B1,
        (Kind::Bool, 8) => types::I8,
        (Kind::Bool, 16) => types::I16,
        (Kind::Bool, 32) => types::I32,
        (Kind::Bool, 64) => types::I64,
        (Kind::Bool, 128) => types::I128,
        _ => unreachable!(),
    }
}

// NB: the unsafe contract we must uphold here is that our implementation of
// `instruction_result_bit_width` must always return a valid, non-zero bit
// width.
unsafe impl<'a, 'b> InstructionSet<'b> for &'a dyn TargetIsa {
    type Context = FuncCursor<'b>;

    type Operator = Opcode;

    type Instruction = ValueOrInst;

    fn replace_instruction(
        &self,
        pos: &mut FuncCursor<'b>,
        old: ValueOrInst,
        new: Part<ValueOrInst>,
    ) -> ValueOrInst {
        log::trace!("replace {:?} with {:?}", old, new);
        let old_inst = old.resolve_inst(&pos.func.dfg).unwrap();

        // Try to convert `new` to an instruction, because we prefer replacing
        // an old instruction with a new one wholesale. However, if the
        // replacement cannot be converted to an instruction (e.g. the
        // right-hand side is a block/function parameter value) then we change
        // the old instruction's result to an alias of the new value.
        let new_inst = match new {
            Part::Instruction(ValueOrInst::Inst(inst)) => Some(inst),
            Part::Instruction(ValueOrInst::Value(_)) => {
                // Do not try and follow the value definition. If we transplant
                // this value's instruction, and there are other uses of this
                // value, then we could mess up ordering between instructions.
                None
            }
            Part::Constant(c) => {
                let v = const_to_value(pos.ins(), c, old_inst);
                let inst = pos.func.dfg.value_def(v).unwrap_inst();
                Some(inst)
            }
            Part::ConditionCode(_) => None,
        };

        match new_inst {
            Some(new_inst) => {
                pos.func.transplant_inst(old_inst, new_inst);
                debug_assert_eq!(pos.current_inst(), Some(old_inst));
                old_inst.into()
            }
            None => {
                let new_value = part_to_value(pos, old_inst, new).unwrap();

                let old_results = pos.func.dfg.detach_results(old_inst);
                let old_results = old_results.as_slice(&pos.func.dfg.value_lists);
                assert_eq!(old_results.len(), 1);
                let old_value = old_results[0];

                pos.func.dfg.change_to_alias(old_value, new_value);
                pos.func.dfg.replace(old_inst).nop();

                new_value.into()
            }
        }
    }

    fn operator<E>(
        &self,
        pos: &mut FuncCursor<'b>,
        value_or_inst: ValueOrInst,
        operands: &mut E,
    ) -> Option<Opcode>
    where
        E: Extend<Part<Self::Instruction>>,
    {
        let inst = value_or_inst.resolve_inst(&pos.func.dfg)?;
        Some(match pos.func.dfg[inst] {
            InstructionData::Binary {
                opcode: opcode @ Opcode::Band,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Bor,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Bxor,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Iadd,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Ifcmp,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Imul,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Ishl,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Isub,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Rotl,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Rotr,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Sdiv,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Srem,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Sshr,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Udiv,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Urem,
                args,
            }
            | InstructionData::Binary {
                opcode: opcode @ Opcode::Ushr,
                args,
            } => {
                operands.extend(args.iter().map(|v| Part::Instruction((*v).into())));
                opcode
            }

            InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::BandImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::BorImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::BxorImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::IaddImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::IfcmpImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::ImulImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::IrsubImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::IshlImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::RotlImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::RotrImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::SdivImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::SremImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::SshrImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::UdivImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::UremImm,
                imm,
                arg,
            }
            | InstructionData::BinaryImm64 {
                opcode: opcode @ Opcode::UshrImm,
                imm,
                arg,
            } => {
                operands.extend(
                    iter::once(imm.into()).chain(iter::once(Part::Instruction(arg.into()))),
                );
                opcode
            }

            InstructionData::Branch {
                opcode: opcode @ Opcode::Brnz,
                ref args,
                destination: _,
            }
            | InstructionData::Branch {
                opcode: opcode @ Opcode::Brz,
                ref args,
                destination: _,
            } => {
                operands.extend(
                    args.as_slice(&pos.func.dfg.value_lists)
                        .iter()
                        .map(|v| Part::Instruction((*v).into()))
                        // NB: Peepmatic only knows about the condition, not any
                        // of the arguments to the block, which are special
                        // cased elsewhere, if/when we actually replace the
                        // instruction.
                        .take(1),
                );
                opcode
            }

            InstructionData::CondTrap {
                opcode: opcode @ Opcode::Trapnz,
                arg,
                code: _,
            }
            | InstructionData::CondTrap {
                opcode: opcode @ Opcode::Trapz,
                arg,
                code: _,
            } => {
                operands.extend(iter::once(Part::Instruction(arg.into())));
                opcode
            }

            InstructionData::IntCompare {
                opcode: opcode @ Opcode::Icmp,
                cond,
                args,
            } => {
                operands.extend(
                    iter::once(intcc_to_peepmatic(cond).into())
                        .chain(args.iter().map(|v| Part::Instruction((*v).into()))),
                );
                opcode
            }

            InstructionData::IntCompareImm {
                opcode: opcode @ Opcode::IcmpImm,
                cond,
                imm,
                arg,
            } => {
                operands.extend(
                    iter::once(intcc_to_peepmatic(cond).into())
                        .chain(iter::once(Part::Constant(imm.into())))
                        .chain(iter::once(Part::Instruction(arg.into()))),
                );
                opcode
            }

            InstructionData::Ternary {
                opcode: opcode @ Opcode::Select,
                ref args,
            } => {
                operands.extend(args.iter().map(|v| Part::Instruction((*v).into())));
                opcode
            }

            InstructionData::Unary {
                opcode: opcode @ Opcode::Bint,
                arg,
            }
            | InstructionData::Unary {
                opcode: opcode @ Opcode::Ireduce,
                arg,
            }
            | InstructionData::Unary {
                opcode: opcode @ Opcode::Sextend,
                arg,
            }
            | InstructionData::Unary {
                opcode: opcode @ Opcode::Uextend,
                arg,
            } => {
                operands.extend(iter::once(Part::Instruction(arg.into())));
                opcode
            }

            InstructionData::UnaryBool { opcode, imm } => {
                operands.extend(iter::once(Part::Constant(Constant::Bool(
                    imm,
                    BitWidth::Polymorphic,
                ))));
                opcode
            }

            InstructionData::UnaryImm {
                opcode: opcode @ Opcode::Iconst,
                imm,
            } => {
                operands.extend(iter::once(imm.into()));
                opcode
            }
            ref otherwise => {
                log::trace!("Not supported by Peepmatic: {:?}", otherwise);
                return None;
            }
        })
    }

    fn make_inst_1(
        &self,
        pos: &mut FuncCursor<'b>,
        root: ValueOrInst,
        operator: Opcode,
        r#type: Type,
        a: Part<ValueOrInst>,
    ) -> ValueOrInst {
        log::trace!("make_inst_1: {:?}({:?})", operator, a);

        let root = root.resolve_inst(&pos.func.dfg).unwrap();
        match operator {
            Opcode::Bconst => {
                let c = a.unwrap_constant();
                let val = const_to_value(pos.ins(), c, root);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Bint => {
                let a = part_to_value(pos, root, a).unwrap();
                let ty = peepmatic_ty_to_ir_ty(r#type, &pos.func.dfg, root);
                let val = pos.ins().bint(ty, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Bnot => {
                let a = part_to_value(pos, root, a).unwrap();
                let val = pos.ins().bnot(a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Brnz => {
                let a = part_to_value(pos, root, a).unwrap();

                // NB: branching instructions must be the root of an
                // optimization's right-hand side, so we get the destination
                // block and arguments from the left-hand side's root. Peepmatic
                // doesn't currently represent labels or varargs.
                let block = pos.func.dfg[root].branch_destination().unwrap();
                let args = pos.func.dfg.inst_args(root)[1..].to_vec();

                pos.ins().brnz(a, block, &args).into()
            }
            Opcode::Brz => {
                let a = part_to_value(pos, root, a).unwrap();

                // See the comment in the `Opcode::Brnz` match argm.
                let block = pos.func.dfg[root].branch_destination().unwrap();
                let args = pos.func.dfg.inst_args(root)[1..].to_vec();

                pos.ins().brz(a, block, &args).into()
            }
            Opcode::Iconst => {
                let a = a.unwrap_constant();
                let val = const_to_value(pos.ins(), a, root);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Ireduce => {
                let a = part_to_value(pos, root, a).unwrap();
                let ty = peepmatic_ty_to_ir_ty(r#type, &pos.func.dfg, root);
                let val = pos.ins().ireduce(ty, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Sextend => {
                let a = part_to_value(pos, root, a).unwrap();
                let ty = peepmatic_ty_to_ir_ty(r#type, &pos.func.dfg, root);
                let val = pos.ins().sextend(ty, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Trapnz => {
                let a = part_to_value(pos, root, a).unwrap();

                // NB: similar to branching instructions (see comment in the
                // `Opcode::Brnz` match arm) trapping instructions must be the
                // root of an optimization's right-hand side, and we get the
                // trap code from the root of the left-hand side. Peepmatic
                // doesn't currently represent trap codes.
                let code = pos.func.dfg[root].trap_code().unwrap();

                pos.ins().trapnz(a, code).into()
            }
            Opcode::Trapz => {
                let a = part_to_value(pos, root, a).unwrap();
                // See comment in the `Opcode::Trapnz` match arm.
                let code = pos.func.dfg[root].trap_code().unwrap();
                pos.ins().trapz(a, code).into()
            }
            Opcode::Uextend => {
                let a = part_to_value(pos, root, a).unwrap();
                let ty = peepmatic_ty_to_ir_ty(r#type, &pos.func.dfg, root);
                let val = pos.ins().uextend(ty, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            _ => unreachable!(),
        }
    }

    fn make_inst_2(
        &self,
        pos: &mut FuncCursor<'b>,
        root: ValueOrInst,
        operator: Opcode,
        _: Type,
        a: Part<ValueOrInst>,
        b: Part<ValueOrInst>,
    ) -> ValueOrInst {
        log::trace!("make_inst_2: {:?}({:?}, {:?})", operator, a, b);

        let root = root.resolve_inst(&pos.func.dfg).unwrap();
        match operator {
            Opcode::Band => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().band(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::BandImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().band_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Bor => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().bor(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::BorImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().bor_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Bxor => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().bxor(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::BxorImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().bxor_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Iadd => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().iadd(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::IaddImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().iadd_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Ifcmp => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().ifcmp(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::IfcmpImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().ifcmp_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Imul => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().imul(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::ImulImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().imul_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::IrsubImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().irsub_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Ishl => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().ishl(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::IshlImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().ishl_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Isub => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().isub(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Rotl => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().rotl(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::RotlImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().rotl_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Rotr => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().rotr(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::RotrImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().rotr_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Sdiv => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().sdiv(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::SdivImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().sdiv_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Srem => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().srem(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::SremImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().srem_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Sshr => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().sshr(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::SshrImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().sshr_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Udiv => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().udiv(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::UdivImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().udiv_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Urem => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().urem(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::UremImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().urem_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Ushr => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().ushr(a, b);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::UshrImm => {
                let a = part_to_imm64(pos, a);
                let b = part_to_value(pos, root, b).unwrap();
                let val = pos.ins().ushr_imm(b, a);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            _ => unreachable!(),
        }
    }

    fn make_inst_3(
        &self,
        pos: &mut FuncCursor<'b>,
        root: ValueOrInst,
        operator: Opcode,
        _: Type,
        a: Part<ValueOrInst>,
        b: Part<ValueOrInst>,
        c: Part<ValueOrInst>,
    ) -> ValueOrInst {
        log::trace!("make_inst_3: {:?}({:?}, {:?}, {:?})", operator, a, b, c);

        let root = root.resolve_inst(&pos.func.dfg).unwrap();
        match operator {
            Opcode::Icmp => {
                let cond = a.unwrap_condition_code();
                let cond = peepmatic_to_intcc(cond);
                let b = part_to_value(pos, root, b).unwrap();
                let c = part_to_value(pos, root, c).unwrap();
                let val = pos.ins().icmp(cond, b, c);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::IcmpImm => {
                let cond = a.unwrap_condition_code();
                let cond = peepmatic_to_intcc(cond);
                let imm = part_to_imm64(pos, b);
                let c = part_to_value(pos, root, c).unwrap();
                let val = pos.ins().icmp_imm(cond, c, imm);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            Opcode::Select => {
                let a = part_to_value(pos, root, a).unwrap();
                let b = part_to_value(pos, root, b).unwrap();
                let c = part_to_value(pos, root, c).unwrap();
                let val = pos.ins().select(a, b, c);
                pos.func.dfg.value_def(val).unwrap_inst().into()
            }
            _ => unreachable!(),
        }
    }

    fn instruction_to_constant(
        &self,
        pos: &mut FuncCursor<'b>,
        value_or_inst: ValueOrInst,
    ) -> Option<Constant> {
        value_or_inst.to_constant(pos)
    }

    fn instruction_result_bit_width(
        &self,
        pos: &mut FuncCursor<'b>,
        value_or_inst: ValueOrInst,
    ) -> u8 {
        value_or_inst.result_bit_width(&pos.func.dfg)
    }

    fn native_word_size_in_bits(&self, _pos: &mut FuncCursor<'b>) -> u8 {
        self.pointer_bits()
    }
}

#[cfg(test)]
#[cfg(any(feature = "x64", feature = "x86", feature = "arm64"))]
mod tests {
    use super::*;
    use crate::isa::{lookup, TargetIsa};
    use crate::settings::{builder, Flags};
    use std::str::FromStr;
    use target_lexicon::triple;

    fn isa() -> Box<dyn TargetIsa> {
        // We need a triple to instantiate and run the peephole optimizer, but we
        // don't care which one when we're just trying to trigger a rebuild of the
        // peephole optimizer (it doesn't affect the serialized bytes at all).
        let triple = if cfg!(any(feature = "x64", feature = "x86")) {
            triple!("x86_64")
        } else if cfg!(feature = "arm64") {
            triple!("aarch64")
        } else {
            panic!("unknown arch")
        };
        lookup(triple).unwrap().finish(Flags::new(builder()))
    }

    #[test]
    fn get_peepmatic_preopt() {
        let isa = isa();
        let _ = preopt(&*isa);
    }
}
