//! The [step] function interprets a single Cranelift instruction given its [State] and
//! [InstructionContext]; the interpretation is generic over [Value]s.
use crate::instruction::InstructionContext;
use crate::state::{MemoryError, State};
use crate::value::{Value, ValueConversionKind, ValueError};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    types, Block, FuncRef, Function, InstructionData, Opcode, TrapCode, Value as ValueRef,
};
use log::trace;
use smallvec::{smallvec, SmallVec};
use thiserror::Error;

/// Interpret a single Cranelift instruction. Note that program traps and interpreter errors are
/// distinct: a program trap results in `Ok(Flow::Trap(...))` whereas an interpretation error (e.g.
/// the types of two values are incompatible) results in `Err(...)`.
#[allow(unused_variables)]
pub fn step<'a, V, I>(
    state: &mut dyn State<'a, V>,
    inst_context: I,
) -> Result<ControlFlow<'a, V>, StepError>
where
    V: Value,
    I: InstructionContext,
{
    let inst = inst_context.data();
    let ctrl_ty = inst_context.controlling_type().unwrap();
    trace!(
        "Step: {}{}",
        inst.opcode(),
        if ctrl_ty.is_invalid() {
            String::new()
        } else {
            format!(".{}", ctrl_ty)
        }
    );

    macro_rules! args {
        () => {
            state
                .collect_values(inst_context.args())
                .map_err(|v| StepError::UnknownValue(v))?
        };
        ($range:expr) => {{
            SmallVec::<[V; 1]>::from(&args!()[$range])
        }};
    }
    macro_rules! arg {
        ($index:expr) => {{
            let value_ref = inst_context.args()[$index];
            state
                .get_value(value_ref)
                .ok_or(StepError::UnknownValue(value_ref))?
        }};
    }
    macro_rules! imm {
        () => {
            V::from(inst.imm_value().unwrap())
        };
    }
    macro_rules! imm_as_ctrl_ty {
        () => {
            V::convert(
                V::from(inst.imm_value().unwrap()),
                ValueConversionKind::Exact(ctrl_ty),
            )?
        };
    }
    macro_rules! binary {
        ($op:path, $a:expr, $b:expr) => {
            assign![$op($a, $b)?]
        };
    }
    macro_rules! binary_unsigned {
        ($op:path, $a:expr, $b:expr) => {
            assign![$op(
                $a.convert(ValueConversionKind::ToUnsigned)?,
                $b.convert(ValueConversionKind::ToUnsigned)?
            )?
            .convert(ValueConversionKind::ToSigned)?]
        };
    }
    macro_rules! assign {
        ($a:expr) => {
            ControlFlow::Assign(smallvec![$a])
        };
    }
    macro_rules! choose {
        ($op:expr, $a:expr, $b:expr) => {
            assign!(if $op { $a } else { $b })
        };
    }
    macro_rules! branch {
        () => {
            inst.branch_destination().unwrap()
        };
    }
    macro_rules! branch_when {
        ($e: expr) => {
            if $e {
                ControlFlow::ContinueAt(branch!(), args!(1..))
            } else {
                ControlFlow::Continue
            }
        };
    }
    macro_rules! trap_code {
        () => {
            inst.trap_code().unwrap()
        };
    }
    macro_rules! trap_when {
        ($e: expr, $t: expr) => {
            if $e {
                ControlFlow::Trap($t)
            } else {
                ControlFlow::Continue
            }
        };
    }
    macro_rules! icmp {
        () => {
            icmp!(inst.cond_code().unwrap(), &arg!(0), &arg!(1))
        };
        ($f: expr, $a: expr, $b: expr) => {
            match $f {
                IntCC::Equal => Value::eq($a, $b)?,
                IntCC::NotEqual => !Value::eq($a, $b)?,
                IntCC::SignedGreaterThan => Value::gt($a, $b)?,
                IntCC::SignedGreaterThanOrEqual => Value::ge($a, $b)?,
                IntCC::SignedLessThan => Value::lt($a, $b)?,
                IntCC::SignedLessThanOrEqual => Value::le($a, $b)?,
                IntCC::UnsignedGreaterThan => Value::gt(
                    &$a.clone().convert(ValueConversionKind::ToUnsigned)?,
                    &$b.clone().convert(ValueConversionKind::ToUnsigned)?,
                )?,
                IntCC::UnsignedGreaterThanOrEqual => Value::ge(
                    &$a.clone().convert(ValueConversionKind::ToUnsigned)?,
                    &$b.clone().convert(ValueConversionKind::ToUnsigned)?,
                )?,
                IntCC::UnsignedLessThan => Value::lt(
                    &$a.clone().convert(ValueConversionKind::ToUnsigned)?,
                    &$b.clone().convert(ValueConversionKind::ToUnsigned)?,
                )?,
                IntCC::UnsignedLessThanOrEqual => Value::le(
                    &$a.clone().convert(ValueConversionKind::ToUnsigned)?,
                    &$b.clone().convert(ValueConversionKind::ToUnsigned)?,
                )?,
                IntCC::Overflow => unimplemented!("IntCC::Overflow"),
                IntCC::NotOverflow => unimplemented!("IntCC::NotOverflow"),
            }
        };
    }
    // TODO should not have to use signed less-than/greater-than
    macro_rules! fcmp {
        () => {
            fcmp!(inst.fp_cond_code().unwrap(), &arg!(0), &arg!(1))
        };
        ($f: expr, $a: expr, $b: expr) => {
            match $f {
                FloatCC::Ordered => Value::eq($a, $b)? || Value::lt($a, $b)? || Value::gt($a, $b)?,
                FloatCC::Unordered => Value::uno($a, $b)?,
                FloatCC::Equal => Value::eq($a, $b)?,
                FloatCC::NotEqual => {
                    Value::lt($a, $b)? || Value::gt($a, $b)? || Value::uno($a, $b)?
                }
                FloatCC::OrderedNotEqual => Value::lt($a, $b)? || Value::gt($a, $b)?,
                FloatCC::UnorderedOrEqual => Value::eq($a, $b)? || Value::uno($a, $b)?,
                FloatCC::LessThan => Value::lt($a, $b)?,
                FloatCC::LessThanOrEqual => Value::lt($a, $b)? || Value::eq($a, $b)?,
                FloatCC::GreaterThan => Value::gt($a, $b)?,
                FloatCC::GreaterThanOrEqual => Value::gt($a, $b)? || Value::eq($a, $b)?,
                FloatCC::UnorderedOrLessThan => Value::uno($a, $b)? || Value::lt($a, $b)?,
                FloatCC::UnorderedOrLessThanOrEqual => {
                    Value::uno($a, $b)? || Value::lt($a, $b)? || Value::eq($a, $b)?
                }
                FloatCC::UnorderedOrGreaterThan => Value::uno($a, $b)? || Value::gt($a, $b)?,
                FloatCC::UnorderedOrGreaterThanOrEqual => {
                    Value::uno($a, $b)? || Value::gt($a, $b)? || Value::eq($a, $b)?
                }
            }
        };
    }
    fn sum<V: Value>(head: V, tail: SmallVec<[V; 1]>) -> Result<i64, ValueError> {
        let mut acc = head;
        for t in tail {
            acc = Value::add(acc, t)?;
        }
        acc.into_int()
    }

    Ok(match inst.opcode() {
        Opcode::Jump | Opcode::Fallthrough => ControlFlow::ContinueAt(branch!(), args!()),
        Opcode::Brz => branch_when!(!arg!(0).into_bool()?),
        Opcode::Brnz => branch_when!(arg!(0).into_bool()?),
        Opcode::BrIcmp => branch_when!(icmp!()),
        Opcode::Brif => branch_when!(state.has_iflag(inst.cond_code().unwrap())),
        Opcode::Brff => branch_when!(state.has_fflag(inst.fp_cond_code().unwrap())),
        Opcode::BrTable => unimplemented!("BrTable"),
        Opcode::JumpTableEntry => unimplemented!("JumpTableEntry"),
        Opcode::JumpTableBase => unimplemented!("JumpTableBase"),
        Opcode::IndirectJumpTableBr => unimplemented!("IndirectJumpTableBr"),
        Opcode::Trap => ControlFlow::Trap(CraneliftTrap::User(trap_code!())),
        Opcode::Debugtrap => ControlFlow::Trap(CraneliftTrap::Debug),
        Opcode::ResumableTrap => ControlFlow::Trap(CraneliftTrap::Resumable),
        Opcode::Trapz => trap_when!(!arg!(0).into_bool()?, CraneliftTrap::User(trap_code!())),
        Opcode::Trapnz => trap_when!(arg!(0).into_bool()?, CraneliftTrap::User(trap_code!())),
        Opcode::ResumableTrapnz => trap_when!(arg!(0).into_bool()?, CraneliftTrap::Resumable),
        Opcode::Trapif => trap_when!(
            state.has_iflag(inst.cond_code().unwrap()),
            CraneliftTrap::User(trap_code!())
        ),
        Opcode::Trapff => trap_when!(
            state.has_fflag(inst.fp_cond_code().unwrap()),
            CraneliftTrap::User(trap_code!())
        ),
        Opcode::Return => ControlFlow::Return(args!()),
        Opcode::FallthroughReturn => ControlFlow::Return(args!()),
        Opcode::Call => {
            if let InstructionData::Call { func_ref, .. } = inst {
                let function = state
                    .get_function(func_ref)
                    .ok_or(StepError::UnknownFunction(func_ref))?;
                ControlFlow::Call(function, args!())
            } else {
                unreachable!()
            }
        }
        Opcode::CallIndirect => unimplemented!("CallIndirect"),
        Opcode::FuncAddr => unimplemented!("FuncAddr"),
        Opcode::Load
        | Opcode::LoadComplex
        | Opcode::Uload8
        | Opcode::Uload8Complex
        | Opcode::Sload8
        | Opcode::Sload8Complex
        | Opcode::Uload16
        | Opcode::Uload16Complex
        | Opcode::Sload16
        | Opcode::Sload16Complex
        | Opcode::Uload32
        | Opcode::Uload32Complex
        | Opcode::Sload32
        | Opcode::Sload32Complex
        | Opcode::Uload8x8
        | Opcode::Uload8x8Complex
        | Opcode::Sload8x8
        | Opcode::Sload8x8Complex
        | Opcode::Uload16x4
        | Opcode::Uload16x4Complex
        | Opcode::Sload16x4
        | Opcode::Sload16x4Complex
        | Opcode::Uload32x2
        | Opcode::Uload32x2Complex
        | Opcode::Sload32x2
        | Opcode::Sload32x2Complex => {
            let address = sum(imm!(), args!())? as usize;
            let ctrl_ty = inst_context.controlling_type().unwrap();
            let (load_ty, kind) = match inst.opcode() {
                Opcode::Load | Opcode::LoadComplex => (ctrl_ty, None),
                Opcode::Uload8 | Opcode::Uload8Complex => {
                    (types::I8, Some(ValueConversionKind::ZeroExtend(ctrl_ty)))
                }
                Opcode::Sload8 | Opcode::Sload8Complex => {
                    (types::I8, Some(ValueConversionKind::SignExtend(ctrl_ty)))
                }
                Opcode::Uload16 | Opcode::Uload16Complex => {
                    (types::I16, Some(ValueConversionKind::ZeroExtend(ctrl_ty)))
                }
                Opcode::Sload16 | Opcode::Sload16Complex => {
                    (types::I16, Some(ValueConversionKind::SignExtend(ctrl_ty)))
                }
                Opcode::Uload32 | Opcode::Uload32Complex => {
                    (types::I32, Some(ValueConversionKind::ZeroExtend(ctrl_ty)))
                }
                Opcode::Sload32 | Opcode::Sload32Complex => {
                    (types::I32, Some(ValueConversionKind::SignExtend(ctrl_ty)))
                }
                Opcode::Uload8x8
                | Opcode::Uload8x8Complex
                | Opcode::Sload8x8
                | Opcode::Sload8x8Complex
                | Opcode::Uload16x4
                | Opcode::Uload16x4Complex
                | Opcode::Sload16x4
                | Opcode::Sload16x4Complex
                | Opcode::Uload32x2
                | Opcode::Uload32x2Complex
                | Opcode::Sload32x2
                | Opcode::Sload32x2Complex => unimplemented!(),
                _ => unreachable!(),
            };
            let loaded = state.load_heap(address, load_ty)?;
            let extended = if let Some(c) = kind {
                loaded.convert(c)?
            } else {
                loaded
            };
            ControlFlow::Assign(smallvec!(extended))
        }
        Opcode::Store
        | Opcode::StoreComplex
        | Opcode::Istore8
        | Opcode::Istore8Complex
        | Opcode::Istore16
        | Opcode::Istore16Complex
        | Opcode::Istore32
        | Opcode::Istore32Complex => {
            let address = sum(imm!(), args!(1..))? as usize;
            let kind = match inst.opcode() {
                Opcode::Store | Opcode::StoreComplex => None,
                Opcode::Istore8 | Opcode::Istore8Complex => {
                    Some(ValueConversionKind::Truncate(types::I8))
                }
                Opcode::Istore16 | Opcode::Istore16Complex => {
                    Some(ValueConversionKind::Truncate(types::I16))
                }
                Opcode::Istore32 | Opcode::Istore32Complex => {
                    Some(ValueConversionKind::Truncate(types::I32))
                }
                _ => unreachable!(),
            };
            let reduced = if let Some(c) = kind {
                arg!(0).convert(c)?
            } else {
                arg!(0)
            };
            state.store_heap(address, reduced)?;
            ControlFlow::Continue
        }
        Opcode::StackLoad => {
            let address = sum(imm!(), args!(1..))? as usize;
            let load_ty = inst_context.controlling_type().unwrap();
            let loaded = state.load_stack(address, load_ty)?;
            ControlFlow::Assign(smallvec!(loaded))
        }
        Opcode::StackStore => {
            let address = sum(imm!(), args!(1..))? as usize;
            state.store_stack(address, arg!(0))?;
            ControlFlow::Continue
        }
        Opcode::StackAddr => unimplemented!("StackAddr"),
        Opcode::GlobalValue => unimplemented!("GlobalValue"),
        Opcode::SymbolValue => unimplemented!("SymbolValue"),
        Opcode::TlsValue => unimplemented!("TlsValue"),
        Opcode::HeapAddr => unimplemented!("HeapAddr"),
        Opcode::GetPinnedReg => unimplemented!("GetPinnedReg"),
        Opcode::SetPinnedReg => unimplemented!("SetPinnedReg"),
        Opcode::TableAddr => unimplemented!("TableAddr"),
        Opcode::Iconst => assign!(Value::int(imm!().into_int()?, ctrl_ty)?),
        Opcode::F32const => assign!(imm!()),
        Opcode::F64const => assign!(imm!()),
        Opcode::Bconst => assign!(imm!()),
        Opcode::Vconst => unimplemented!("Vconst"),
        Opcode::ConstAddr => unimplemented!("ConstAddr"),
        Opcode::Null => unimplemented!("Null"),
        Opcode::Nop => ControlFlow::Continue,
        Opcode::Select => choose!(arg!(0).into_bool()?, arg!(1), arg!(2)),
        Opcode::Selectif => choose!(state.has_iflag(inst.cond_code().unwrap()), arg!(1), arg!(2)),
        Opcode::SelectifSpectreGuard => unimplemented!("SelectifSpectreGuard"),
        Opcode::Bitselect => {
            let mask_a = Value::and(arg!(0), arg!(1))?;
            let mask_b = Value::and(Value::not(arg!(0))?, arg!(2))?;
            assign!(Value::or(mask_a, mask_b)?)
        }
        Opcode::Copy => assign!(arg!(0)),
        Opcode::Spill => unimplemented!("Spill"),
        Opcode::Fill => unimplemented!("Fill"),
        Opcode::FillNop => assign!(arg!(0)),
        Opcode::DummySargT => unimplemented!("DummySargT"),
        Opcode::Regmove => ControlFlow::Continue,
        Opcode::CopySpecial => ControlFlow::Continue,
        Opcode::CopyToSsa => assign!(arg!(0)),
        Opcode::CopyNop => unimplemented!("CopyNop"),
        Opcode::AdjustSpDown => unimplemented!("AdjustSpDown"),
        Opcode::AdjustSpUpImm => unimplemented!("AdjustSpUpImm"),
        Opcode::AdjustSpDownImm => unimplemented!("AdjustSpDownImm"),
        Opcode::IfcmpSp => unimplemented!("IfcmpSp"),
        Opcode::Regspill => unimplemented!("Regspill"),
        Opcode::Regfill => unimplemented!("Regfill"),
        Opcode::Safepoint => unimplemented!("Safepoint"),
        Opcode::Icmp => assign!(Value::bool(icmp!(), ctrl_ty.as_bool())?),
        Opcode::IcmpImm => assign!(Value::bool(
            icmp!(inst.cond_code().unwrap(), &arg!(0), &imm_as_ctrl_ty!()),
            ctrl_ty.as_bool()
        )?),
        Opcode::Ifcmp | Opcode::IfcmpImm => {
            let arg1 = match inst.opcode() {
                Opcode::Ifcmp => arg!(1),
                Opcode::IfcmpImm => imm_as_ctrl_ty!(),
                _ => unreachable!(),
            };
            state.clear_flags();
            for f in &[
                IntCC::Equal,
                IntCC::NotEqual,
                IntCC::SignedLessThan,
                IntCC::SignedGreaterThanOrEqual,
                IntCC::SignedGreaterThan,
                IntCC::SignedLessThanOrEqual,
                IntCC::UnsignedLessThan,
                IntCC::UnsignedGreaterThanOrEqual,
                IntCC::UnsignedGreaterThan,
                IntCC::UnsignedLessThanOrEqual,
            ] {
                if icmp!(f, &arg!(0), &arg1) {
                    state.set_iflag(*f);
                }
            }
            ControlFlow::Continue
        }
        Opcode::Imin => choose!(Value::gt(&arg!(1), &arg!(0))?, arg!(0), arg!(1)),
        Opcode::Umin => choose!(
            Value::gt(
                &arg!(1).convert(ValueConversionKind::ToUnsigned)?,
                &arg!(0).convert(ValueConversionKind::ToUnsigned)?
            )?,
            arg!(0),
            arg!(1)
        ),
        Opcode::Imax => choose!(Value::gt(&arg!(0), &arg!(1))?, arg!(0), arg!(1)),
        Opcode::Umax => choose!(
            Value::gt(
                &arg!(0).convert(ValueConversionKind::ToUnsigned)?,
                &arg!(1).convert(ValueConversionKind::ToUnsigned)?
            )?,
            arg!(0),
            arg!(1)
        ),
        Opcode::AvgRound => {
            let sum = Value::add(arg!(0), arg!(1))?;
            let one = Value::int(1, arg!(0).ty())?;
            let inc = Value::add(sum, one)?;
            let two = Value::int(2, arg!(0).ty())?;
            binary!(Value::div, inc, two)
        }
        Opcode::Iadd => binary!(Value::add, arg!(0), arg!(1)),
        Opcode::UaddSat => unimplemented!("UaddSat"),
        Opcode::SaddSat => unimplemented!("SaddSat"),
        Opcode::Isub => binary!(Value::sub, arg!(0), arg!(1)),
        Opcode::UsubSat => unimplemented!("UsubSat"),
        Opcode::SsubSat => unimplemented!("SsubSat"),
        Opcode::Ineg => binary!(Value::sub, Value::int(0, ctrl_ty)?, arg!(0)),
        Opcode::Iabs => unimplemented!("Iabs"),
        Opcode::Imul => binary!(Value::mul, arg!(0), arg!(1)),
        Opcode::Umulhi => unimplemented!("Umulhi"),
        Opcode::Smulhi => unimplemented!("Smulhi"),
        Opcode::Udiv => binary_unsigned!(Value::div, arg!(0), arg!(1)),
        Opcode::Sdiv => binary!(Value::div, arg!(0), arg!(1)),
        Opcode::Urem => binary_unsigned!(Value::rem, arg!(0), arg!(1)),
        Opcode::Srem => binary!(Value::rem, arg!(0), arg!(1)),
        Opcode::IaddImm => binary!(Value::add, arg!(0), imm_as_ctrl_ty!()),
        Opcode::ImulImm => binary!(Value::mul, arg!(0), imm_as_ctrl_ty!()),
        Opcode::UdivImm => binary_unsigned!(Value::div, arg!(0), imm!()),
        Opcode::SdivImm => binary!(Value::div, arg!(0), imm_as_ctrl_ty!()),
        Opcode::UremImm => binary_unsigned!(Value::rem, arg!(0), imm!()),
        Opcode::SremImm => binary!(Value::rem, arg!(0), imm_as_ctrl_ty!()),
        Opcode::IrsubImm => binary!(Value::sub, imm_as_ctrl_ty!(), arg!(0)),
        Opcode::IaddCin => unimplemented!("IaddCin"),
        Opcode::IaddIfcin => unimplemented!("IaddIfcin"),
        Opcode::IaddCout => unimplemented!("IaddCout"),
        Opcode::IaddIfcout => unimplemented!("IaddIfcout"),
        Opcode::IaddCarry => unimplemented!("IaddCarry"),
        Opcode::IaddIfcarry => unimplemented!("IaddIfcarry"),
        Opcode::IsubBin => unimplemented!("IsubBin"),
        Opcode::IsubIfbin => unimplemented!("IsubIfbin"),
        Opcode::IsubBout => unimplemented!("IsubBout"),
        Opcode::IsubIfbout => unimplemented!("IsubIfbout"),
        Opcode::IsubBorrow => unimplemented!("IsubBorrow"),
        Opcode::IsubIfborrow => unimplemented!("IsubIfborrow"),
        Opcode::Band => binary!(Value::and, arg!(0), arg!(1)),
        Opcode::Bor => binary!(Value::or, arg!(0), arg!(1)),
        Opcode::Bxor => binary!(Value::xor, arg!(0), arg!(1)),
        Opcode::Bnot => assign!(Value::not(arg!(0))?),
        Opcode::BandNot => binary!(Value::and, arg!(0), Value::not(arg!(1))?),
        Opcode::BorNot => binary!(Value::or, arg!(0), Value::not(arg!(1))?),
        Opcode::BxorNot => binary!(Value::xor, arg!(0), Value::not(arg!(1))?),
        Opcode::BandImm => binary!(Value::and, arg!(0), imm_as_ctrl_ty!()),
        Opcode::BorImm => binary!(Value::or, arg!(0), imm_as_ctrl_ty!()),
        Opcode::BxorImm => binary!(Value::xor, arg!(0), imm_as_ctrl_ty!()),
        Opcode::Rotl => binary!(Value::rotl, arg!(0), arg!(1)),
        Opcode::Rotr => binary!(Value::rotr, arg!(0), arg!(1)),
        Opcode::RotlImm => binary!(Value::rotl, arg!(0), imm_as_ctrl_ty!()),
        Opcode::RotrImm => binary!(Value::rotr, arg!(0), imm_as_ctrl_ty!()),
        Opcode::Ishl => binary!(Value::shl, arg!(0), arg!(1)),
        Opcode::Ushr => binary!(Value::ushr, arg!(0), arg!(1)),
        Opcode::Sshr => binary!(Value::ishr, arg!(0), arg!(1)),
        Opcode::IshlImm => binary!(Value::shl, arg!(0), imm_as_ctrl_ty!()),
        Opcode::UshrImm => binary!(Value::ushr, arg!(0), imm_as_ctrl_ty!()),
        Opcode::SshrImm => binary!(Value::ishr, arg!(0), imm_as_ctrl_ty!()),
        Opcode::Bitrev => unimplemented!("Bitrev"),
        Opcode::Clz => unimplemented!("Clz"),
        Opcode::Cls => unimplemented!("Cls"),
        Opcode::Ctz => unimplemented!("Ctz"),
        Opcode::Popcnt => unimplemented!("Popcnt"),
        Opcode::Fcmp => assign!(Value::bool(fcmp!(), ctrl_ty.as_bool())?),
        Opcode::Ffcmp => {
            state.clear_flags();
            for f in &[
                FloatCC::Ordered,
                FloatCC::Unordered,
                FloatCC::Equal,
                FloatCC::NotEqual,
                FloatCC::OrderedNotEqual,
                FloatCC::UnorderedOrEqual,
                FloatCC::LessThan,
                FloatCC::LessThanOrEqual,
                FloatCC::GreaterThan,
                FloatCC::GreaterThanOrEqual,
                FloatCC::UnorderedOrLessThan,
                FloatCC::UnorderedOrLessThanOrEqual,
                FloatCC::UnorderedOrGreaterThan,
                FloatCC::UnorderedOrGreaterThanOrEqual,
            ] {
                if fcmp!(f, &arg!(0), &arg!(1)) {
                    state.set_fflag(*f);
                }
            }
            ControlFlow::Continue
        }
        Opcode::Fadd => binary!(Value::add, arg!(0), arg!(1)),
        Opcode::Fsub => binary!(Value::sub, arg!(0), arg!(1)),
        Opcode::Fmul => binary!(Value::mul, arg!(0), arg!(1)),
        Opcode::Fdiv => binary!(Value::div, arg!(0), arg!(1)),
        Opcode::Sqrt => unimplemented!("Sqrt"),
        Opcode::Fma => unimplemented!("Fma"),
        Opcode::Fneg => binary!(Value::sub, Value::float(0, ctrl_ty)?, arg!(0)),
        Opcode::Fabs => unimplemented!("Fabs"),
        Opcode::Fcopysign => unimplemented!("Fcopysign"),
        Opcode::Fmin => choose!(
            Value::is_nan(&arg!(0))? || Value::lt(&arg!(0), &arg!(1))?,
            arg!(0),
            arg!(1)
        ),
        Opcode::FminPseudo => unimplemented!("FminPseudo"),
        Opcode::Fmax => choose!(
            Value::is_nan(&arg!(0))? || Value::gt(&arg!(0), &arg!(1))?,
            arg!(0),
            arg!(1)
        ),
        Opcode::FmaxPseudo => unimplemented!("FmaxPseudo"),
        Opcode::Ceil => unimplemented!("Ceil"),
        Opcode::Floor => unimplemented!("Floor"),
        Opcode::Trunc => unimplemented!("Trunc"),
        Opcode::Nearest => unimplemented!("Nearest"),
        Opcode::IsNull => unimplemented!("IsNull"),
        Opcode::IsInvalid => unimplemented!("IsInvalid"),
        Opcode::Trueif => choose!(
            state.has_iflag(inst.cond_code().unwrap()),
            Value::bool(true, ctrl_ty)?,
            Value::bool(false, ctrl_ty)?
        ),
        Opcode::Trueff => choose!(
            state.has_fflag(inst.fp_cond_code().unwrap()),
            Value::bool(true, ctrl_ty)?,
            Value::bool(false, ctrl_ty)?
        ),
        Opcode::Bitcast
        | Opcode::RawBitcast
        | Opcode::ScalarToVector
        | Opcode::Breduce
        | Opcode::Bextend
        | Opcode::Bint
        | Opcode::Bmask
        | Opcode::Ireduce => assign!(Value::convert(
            arg!(0),
            ValueConversionKind::Exact(ctrl_ty)
        )?),
        Opcode::Snarrow => assign!(Value::convert(
            arg!(0),
            ValueConversionKind::Truncate(ctrl_ty)
        )?),
        Opcode::Sextend => assign!(Value::convert(
            arg!(0),
            ValueConversionKind::SignExtend(ctrl_ty)
        )?),
        Opcode::Unarrow => assign!(Value::convert(
            arg!(0),
            ValueConversionKind::Truncate(ctrl_ty)
        )?),
        Opcode::Uextend => assign!(Value::convert(
            arg!(0),
            ValueConversionKind::ZeroExtend(ctrl_ty)
        )?),
        Opcode::Fpromote => assign!(Value::convert(
            arg!(0),
            ValueConversionKind::Exact(ctrl_ty)
        )?),
        Opcode::Fdemote => assign!(Value::convert(
            arg!(0),
            ValueConversionKind::RoundNearestEven(ctrl_ty)
        )?),
        Opcode::Shuffle => unimplemented!("Shuffle"),
        Opcode::Swizzle => unimplemented!("Swizzle"),
        Opcode::Splat => unimplemented!("Splat"),
        Opcode::LoadSplat => unimplemented!("LoadSplat"),
        Opcode::Insertlane => unimplemented!("Insertlane"),
        Opcode::Extractlane => unimplemented!("Extractlane"),
        Opcode::VhighBits => unimplemented!("VhighBits"),
        Opcode::Vsplit => unimplemented!("Vsplit"),
        Opcode::Vconcat => unimplemented!("Vconcat"),
        Opcode::Vselect => unimplemented!("Vselect"),
        Opcode::VanyTrue => unimplemented!("VanyTrue"),
        Opcode::VallTrue => unimplemented!("VallTrue"),
        Opcode::SwidenLow => unimplemented!("SwidenLow"),
        Opcode::SwidenHigh => unimplemented!("SwidenHigh"),
        Opcode::UwidenLow => unimplemented!("UwidenLow"),
        Opcode::UwidenHigh => unimplemented!("UwidenHigh"),
        Opcode::FcvtToUint => unimplemented!("FcvtToUint"),
        Opcode::FcvtToUintSat => unimplemented!("FcvtToUintSat"),
        Opcode::FcvtToSint => unimplemented!("FcvtToSint"),
        Opcode::FcvtToSintSat => unimplemented!("FcvtToSintSat"),
        Opcode::FcvtFromUint => unimplemented!("FcvtFromUint"),
        Opcode::FcvtFromSint => unimplemented!("FcvtFromSint"),
        Opcode::Isplit => unimplemented!("Isplit"),
        Opcode::Iconcat => unimplemented!("Iconcat"),
        Opcode::AtomicRmw => unimplemented!("AtomicRmw"),
        Opcode::AtomicCas => unimplemented!("AtomicCas"),
        Opcode::AtomicLoad => unimplemented!("AtomicLoad"),
        Opcode::AtomicStore => unimplemented!("AtomicStore"),
        Opcode::Fence => unimplemented!("Fence"),

        // TODO: these instructions should be removed once the new backend makes these obsolete
        // (see https://github.com/bytecodealliance/wasmtime/issues/1936); additionally, the
        // "all-arch" feature for cranelift-codegen would become unnecessary for this crate.
        Opcode::X86Udivmodx
        | Opcode::X86Sdivmodx
        | Opcode::X86Umulx
        | Opcode::X86Smulx
        | Opcode::X86Cvtt2si
        | Opcode::X86Vcvtudq2ps
        | Opcode::X86Fmin
        | Opcode::X86Fmax
        | Opcode::X86Push
        | Opcode::X86Pop
        | Opcode::X86Bsr
        | Opcode::X86Bsf
        | Opcode::X86Pshufd
        | Opcode::X86Pshufb
        | Opcode::X86Pblendw
        | Opcode::X86Pextr
        | Opcode::X86Pinsr
        | Opcode::X86Insertps
        | Opcode::X86Punpckh
        | Opcode::X86Punpckl
        | Opcode::X86Movsd
        | Opcode::X86Movlhps
        | Opcode::X86Psll
        | Opcode::X86Psrl
        | Opcode::X86Psra
        | Opcode::X86Pmullq
        | Opcode::X86Pmuludq
        | Opcode::X86Ptest
        | Opcode::X86Pmaxs
        | Opcode::X86Pmaxu
        | Opcode::X86Pmins
        | Opcode::X86Pminu
        | Opcode::X86Palignr
        | Opcode::X86ElfTlsGetAddr
        | Opcode::X86MachoTlsGetAddr => unimplemented!("x86 instruction: {}", inst.opcode()),
    })
}

#[derive(Error, Debug)]
pub enum StepError {
    #[error("unable to retrieve value from SSA reference: {0}")]
    UnknownValue(ValueRef),
    #[error("unable to find the following function: {0}")]
    UnknownFunction(FuncRef),
    #[error("cannot step with these values")]
    ValueError(#[from] ValueError),
    #[error("failed to access memory")]
    MemoryError(#[from] MemoryError),
}

/// Enumerate the ways in which the control flow can change based on a single step in a Cranelift
/// interpreter.
#[derive(Debug)]
pub enum ControlFlow<'a, V> {
    /// Return one or more values from an instruction to be assigned to a left-hand side, e.g.:
    /// in `v0 = iadd v1, v2`, the sum of `v1` and `v2` is assigned to `v0`.
    Assign(SmallVec<[V; 1]>),
    /// Continue to the next available instruction, e.g.: in `nop`, we expect to resume execution
    /// at the instruction after it.
    Continue,
    /// Jump to another block with the given parameters, e.g.: in `brz v0, block42, [v1, v2]`, if
    /// the condition is true, we continue execution at the first instruction of `block42` with the
    /// values in `v1` and `v2` filling in the block parameters.
    ContinueAt(Block, SmallVec<[V; 1]>),
    /// Indicates a call the given [Function] with the supplied arguments.
    Call(&'a Function, SmallVec<[V; 1]>),
    /// Return from the current function with the given parameters, e.g.: `return [v1, v2]`.
    Return(SmallVec<[V; 1]>),
    /// Stop with a program-generated trap; note that these are distinct from errors that may occur
    /// during interpretation.
    Trap(CraneliftTrap),
}

impl<'a, V> ControlFlow<'a, V> {
    /// For convenience, we can unwrap the [ControlFlow] state assuming that it is a
    /// [ControlFlow::Return], panicking otherwise.
    pub fn unwrap_return(self) -> Vec<V> {
        if let ControlFlow::Return(values) = self {
            values.into_vec()
        } else {
            panic!("expected the control flow to be in the return state")
        }
    }
}

#[derive(Error, Debug)]
pub enum CraneliftTrap {
    #[error("user code: {0}")]
    User(TrapCode),
    #[error("user debug")]
    Debug,
    #[error("resumable")]
    Resumable,
}
