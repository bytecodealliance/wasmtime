//! The [step] function interprets a single Cranelift instruction given its [State] and
//! [InstructionContext]; the interpretation is generic over [Value]s.
use crate::address::{Address, AddressSize};
use crate::instruction::InstructionContext;
use crate::state::{MemoryError, State};
use crate::value::{Value, ValueConversionKind, ValueError, ValueResult};
use cranelift_codegen::data_value::DataValue;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    types, Block, FuncRef, Function, InstructionData, Opcode, TrapCode, Value as ValueRef,
};
use log::trace;
use smallvec::{smallvec, SmallVec};
use std::convert::{TryFrom, TryInto};
use std::ops::RangeFrom;
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

    // The following closures make the `step` implementation much easier to express. Note that they
    // frequently close over the `state` or `inst_context` for brevity.

    // Retrieve the current value for an instruction argument.
    let arg = |index: usize| -> Result<V, StepError> {
        let value_ref = inst_context.args()[index];
        state
            .get_value(value_ref)
            .ok_or(StepError::UnknownValue(value_ref))
    };

    // Retrieve the current values for all of an instruction's arguments.
    let args = || -> Result<SmallVec<[V; 1]>, StepError> {
        state
            .collect_values(inst_context.args())
            .map_err(|v| StepError::UnknownValue(v))
    };

    // Retrieve the current values for a range of an instruction's arguments.
    let args_range = |indexes: RangeFrom<usize>| -> Result<SmallVec<[V; 1]>, StepError> {
        Ok(SmallVec::<[V; 1]>::from(&args()?[indexes]))
    };

    // Retrieve the immediate value for an instruction, expecting it to exist.
    let imm = || -> V {
        V::from(match inst {
            InstructionData::UnaryConst {
                constant_handle, ..
            } => {
                let buffer = state
                    .get_current_function()
                    .dfg
                    .constants
                    .get(constant_handle.clone())
                    .as_slice();
                DataValue::V128(buffer.try_into().expect("a 16-byte data buffer"))
            }
            InstructionData::Shuffle { imm, .. } => {
                let mask = state
                    .get_current_function()
                    .dfg
                    .immediates
                    .get(imm)
                    .unwrap()
                    .as_slice();
                DataValue::V128(mask.try_into().expect("a 16-byte vector mask"))
            }
            _ => inst.imm_value().unwrap(),
        })
    };

    // Retrieve the immediate value for an instruction and convert it to the controlling type of the
    // instruction. For example, since `InstructionData` stores all integer immediates in a 64-bit
    // size, this will attempt to convert `iconst.i8 ...` to an 8-bit size.
    let imm_as_ctrl_ty =
        || -> Result<V, ValueError> { V::convert(imm(), ValueConversionKind::Exact(ctrl_ty)) };

    // Indicate that the result of a step is to assign a single value to an instruction's results.
    let assign = |value: V| ControlFlow::Assign(smallvec![value]);

    // Indicate that the result of a step is to assign multiple values to an instruction's results.
    let assign_multiple = |values: &[V]| ControlFlow::Assign(SmallVec::from(values));

    // Similar to `assign` but converts some errors into traps
    let assign_or_trap = |value: ValueResult<V>| match value {
        Ok(v) => Ok(assign(v)),
        Err(ValueError::IntegerDivisionByZero) => Ok(ControlFlow::Trap(CraneliftTrap::User(
            TrapCode::IntegerDivisionByZero,
        ))),
        Err(ValueError::IntegerOverflow) => Ok(ControlFlow::Trap(CraneliftTrap::User(
            TrapCode::IntegerOverflow,
        ))),
        Err(e) => Err(e),
    };

    let memerror_to_trap = |e: MemoryError| match e {
        MemoryError::InvalidAddress(_) => TrapCode::HeapOutOfBounds,
        MemoryError::InvalidAddressType(_) => TrapCode::HeapOutOfBounds,
        MemoryError::InvalidOffset { .. } => TrapCode::HeapOutOfBounds,
        MemoryError::InvalidEntry { .. } => TrapCode::HeapOutOfBounds,
        MemoryError::OutOfBoundsStore { .. } => TrapCode::HeapOutOfBounds,
        MemoryError::OutOfBoundsLoad { .. } => TrapCode::HeapOutOfBounds,
    };

    // Assigns or traps depending on the value of the result
    let assign_or_memtrap = |res| match res {
        Ok(v) => assign(v),
        Err(e) => ControlFlow::Trap(CraneliftTrap::User(memerror_to_trap(e))),
    };

    // Continues or traps depending on the value of the result
    let continue_or_memtrap = |res| match res {
        Ok(_) => ControlFlow::Continue,
        Err(e) => ControlFlow::Trap(CraneliftTrap::User(memerror_to_trap(e))),
    };

    let calculate_addr = |imm: V, args: SmallVec<[V; 1]>| -> ValueResult<u64> {
        let imm = imm.convert(ValueConversionKind::ZeroExtend(ctrl_ty))?;
        let args = args
            .into_iter()
            .map(|v| v.convert(ValueConversionKind::ZeroExtend(ctrl_ty)))
            .collect::<ValueResult<SmallVec<[V; 1]>>>()?;

        Ok(sum(imm, args)? as u64)
    };

    // Interpret a binary instruction with the given `op`, assigning the resulting value to the
    // instruction's results.
    let binary = |op: fn(V, V) -> ValueResult<V>,
                  left: V,
                  right: V|
     -> ValueResult<ControlFlow<V>> { Ok(assign(op(left, right)?)) };

    // Similar to `binary` but converts select `ValueError`'s into trap `ControlFlow`'s
    let binary_can_trap = |op: fn(V, V) -> ValueResult<V>,
                           left: V,
                           right: V|
     -> ValueResult<ControlFlow<V>> { assign_or_trap(op(left, right)) };

    // Same as `binary_can_trap`, but converts the values to their unsigned form before the
    // operation and back to signed form afterwards. Since Cranelift types have no notion of
    // signedness, this enables operations that depend on sign.
    let binary_unsigned_can_trap =
        |op: fn(V, V) -> ValueResult<V>, left: V, right: V| -> ValueResult<ControlFlow<V>> {
            assign_or_trap(
                op(
                    left.convert(ValueConversionKind::ToUnsigned)?,
                    right.convert(ValueConversionKind::ToUnsigned)?,
                )
                .and_then(|v| v.convert(ValueConversionKind::ToSigned)),
            )
        };

    // Choose whether to assign `left` or `right` to the instruction's result based on a `condition`.
    let choose = |condition: bool, left: V, right: V| -> ControlFlow<V> {
        assign(if condition { left } else { right })
    };

    // Retrieve an instruction's branch destination; expects the instruction to be a branch.
    let branch = || -> Block { inst.branch_destination().unwrap() };

    // Based on `condition`, indicate where to continue the control flow.
    let branch_when = |condition: bool| -> Result<ControlFlow<V>, StepError> {
        let branch_args = match inst {
            InstructionData::Jump { .. } => args_range(0..),
            InstructionData::BranchInt { .. }
            | InstructionData::BranchFloat { .. }
            | InstructionData::Branch { .. } => args_range(1..),
            InstructionData::BranchIcmp { .. } => args_range(2..),
            _ => panic!("Unrecognized branch inst: {:?}", inst),
        }?;

        Ok(if condition {
            ControlFlow::ContinueAt(branch(), branch_args)
        } else {
            ControlFlow::Continue
        })
    };

    // Retrieve an instruction's trap code; expects the instruction to be a trap.
    let trap_code = || -> TrapCode { inst.trap_code().unwrap() };

    // Based on `condition`, either trap or not.
    let trap_when = |condition: bool, trap: CraneliftTrap| -> ControlFlow<V> {
        if condition {
            ControlFlow::Trap(trap)
        } else {
            ControlFlow::Continue
        }
    };

    // Helper for summing a sequence of values.
    fn sum<V: Value>(head: V, tail: SmallVec<[V; 1]>) -> ValueResult<i128> {
        let mut acc = head;
        for t in tail {
            acc = Value::add(acc, t)?;
        }
        acc.into_int()
    }

    // Interpret a Cranelift instruction.
    Ok(match inst.opcode() {
        Opcode::Jump => ControlFlow::ContinueAt(branch(), args()?),
        Opcode::Brz => branch_when(
            !arg(0)?
                .convert(ValueConversionKind::ToBoolean)?
                .into_bool()?,
        )?,
        Opcode::Brnz => branch_when(
            arg(0)?
                .convert(ValueConversionKind::ToBoolean)?
                .into_bool()?,
        )?,
        Opcode::BrIcmp => {
            branch_when(icmp(ctrl_ty, inst.cond_code().unwrap(), &arg(0)?, &arg(1)?)?.into_bool()?)?
        }
        Opcode::Brif => branch_when(state.has_iflag(inst.cond_code().unwrap()))?,
        Opcode::Brff => branch_when(state.has_fflag(inst.fp_cond_code().unwrap()))?,
        Opcode::BrTable => {
            if let InstructionData::BranchTable {
                table, destination, ..
            } = inst
            {
                let jt_data = &state.get_current_function().jump_tables[table];

                // Convert to usize to remove negative indexes from the following operations
                let jump_target = usize::try_from(arg(0)?.into_int()?)
                    .ok()
                    .and_then(|i| jt_data.as_slice().get(i))
                    .copied()
                    .unwrap_or(destination);

                ControlFlow::ContinueAt(jump_target, SmallVec::new())
            } else {
                unreachable!()
            }
        }
        Opcode::Trap => ControlFlow::Trap(CraneliftTrap::User(trap_code())),
        Opcode::Debugtrap => ControlFlow::Trap(CraneliftTrap::Debug),
        Opcode::ResumableTrap => ControlFlow::Trap(CraneliftTrap::Resumable),
        Opcode::Trapz => trap_when(!arg(0)?.into_bool()?, CraneliftTrap::User(trap_code())),
        Opcode::Trapnz => trap_when(arg(0)?.into_bool()?, CraneliftTrap::User(trap_code())),
        Opcode::ResumableTrapnz => trap_when(arg(0)?.into_bool()?, CraneliftTrap::Resumable),
        Opcode::Trapif => trap_when(
            state.has_iflag(inst.cond_code().unwrap()),
            CraneliftTrap::User(trap_code()),
        ),
        Opcode::Trapff => trap_when(
            state.has_fflag(inst.fp_cond_code().unwrap()),
            CraneliftTrap::User(trap_code()),
        ),
        Opcode::Return => ControlFlow::Return(args()?),
        Opcode::FallthroughReturn => ControlFlow::Return(args()?),
        Opcode::Call => {
            if let InstructionData::Call { func_ref, .. } = inst {
                let function = state
                    .get_function(func_ref)
                    .ok_or(StepError::UnknownFunction(func_ref))?;
                ControlFlow::Call(function, args()?)
            } else {
                unreachable!()
            }
        }
        Opcode::CallIndirect => unimplemented!("CallIndirect"),
        Opcode::FuncAddr => unimplemented!("FuncAddr"),
        Opcode::Load
        | Opcode::Uload8
        | Opcode::Sload8
        | Opcode::Uload16
        | Opcode::Sload16
        | Opcode::Uload32
        | Opcode::Sload32
        | Opcode::Uload8x8
        | Opcode::Sload8x8
        | Opcode::Uload16x4
        | Opcode::Sload16x4
        | Opcode::Uload32x2
        | Opcode::Sload32x2 => {
            let ctrl_ty = inst_context.controlling_type().unwrap();
            let (load_ty, kind) = match inst.opcode() {
                Opcode::Load => (ctrl_ty, None),
                Opcode::Uload8 => (types::I8, Some(ValueConversionKind::ZeroExtend(ctrl_ty))),
                Opcode::Sload8 => (types::I8, Some(ValueConversionKind::SignExtend(ctrl_ty))),
                Opcode::Uload16 => (types::I16, Some(ValueConversionKind::ZeroExtend(ctrl_ty))),
                Opcode::Sload16 => (types::I16, Some(ValueConversionKind::SignExtend(ctrl_ty))),
                Opcode::Uload32 => (types::I32, Some(ValueConversionKind::ZeroExtend(ctrl_ty))),
                Opcode::Sload32 => (types::I32, Some(ValueConversionKind::SignExtend(ctrl_ty))),
                Opcode::Uload8x8
                | Opcode::Sload8x8
                | Opcode::Uload16x4
                | Opcode::Sload16x4
                | Opcode::Uload32x2
                | Opcode::Sload32x2 => unimplemented!(),
                _ => unreachable!(),
            };

            let addr_value = calculate_addr(imm(), args()?)?;
            let loaded = assign_or_memtrap(
                Address::try_from(addr_value).and_then(|addr| state.checked_load(addr, load_ty)),
            );

            match (loaded, kind) {
                (ControlFlow::Assign(ret), Some(c)) => ControlFlow::Assign(
                    ret.into_iter()
                        .map(|loaded| loaded.convert(c.clone()))
                        .collect::<ValueResult<SmallVec<[V; 1]>>>()?,
                ),
                (cf, _) => cf,
            }
        }
        Opcode::Store | Opcode::Istore8 | Opcode::Istore16 | Opcode::Istore32 => {
            let kind = match inst.opcode() {
                Opcode::Store => None,
                Opcode::Istore8 => Some(ValueConversionKind::Truncate(types::I8)),
                Opcode::Istore16 => Some(ValueConversionKind::Truncate(types::I16)),
                Opcode::Istore32 => Some(ValueConversionKind::Truncate(types::I32)),
                _ => unreachable!(),
            };

            let addr_value = calculate_addr(imm(), args_range(1..)?)?;
            let reduced = if let Some(c) = kind {
                arg(0)?.convert(c)?
            } else {
                arg(0)?
            };
            continue_or_memtrap(
                Address::try_from(addr_value).and_then(|addr| state.checked_store(addr, reduced)),
            )
        }
        Opcode::StackLoad => {
            let load_ty = inst_context.controlling_type().unwrap();
            let slot = inst.stack_slot().unwrap();
            let offset = sum(imm(), args()?)? as u64;
            assign_or_memtrap({
                state
                    .stack_address(AddressSize::_64, slot, offset)
                    .and_then(|addr| state.checked_load(addr, load_ty))
            })
        }
        Opcode::StackStore => {
            let arg = arg(0)?;
            let slot = inst.stack_slot().unwrap();
            let offset = sum(imm(), args_range(1..)?)? as u64;
            continue_or_memtrap({
                state
                    .stack_address(AddressSize::_64, slot, offset)
                    .and_then(|addr| state.checked_store(addr, arg))
            })
        }
        Opcode::StackAddr => {
            let load_ty = inst_context.controlling_type().unwrap();
            let slot = inst.stack_slot().unwrap();
            let offset = sum(imm(), args()?)? as u64;
            assign_or_memtrap({
                AddressSize::try_from(load_ty).and_then(|addr_size| {
                    let addr = state.stack_address(addr_size, slot, offset)?;
                    let dv = DataValue::try_from(addr)?;
                    Ok(dv.into())
                })
            })
        }
        Opcode::GlobalValue => unimplemented!("GlobalValue"),
        Opcode::SymbolValue => unimplemented!("SymbolValue"),
        Opcode::TlsValue => unimplemented!("TlsValue"),
        Opcode::HeapAddr => unimplemented!("HeapAddr"),
        Opcode::GetPinnedReg => unimplemented!("GetPinnedReg"),
        Opcode::SetPinnedReg => unimplemented!("SetPinnedReg"),
        Opcode::TableAddr => unimplemented!("TableAddr"),
        Opcode::Iconst => assign(Value::int(imm().into_int()?, ctrl_ty)?),
        Opcode::F32const => assign(imm()),
        Opcode::F64const => assign(imm()),
        Opcode::Bconst => assign(imm()),
        Opcode::Vconst => assign(imm()),
        Opcode::ConstAddr => unimplemented!("ConstAddr"),
        Opcode::Null => unimplemented!("Null"),
        Opcode::Nop => ControlFlow::Continue,
        Opcode::Select => choose(arg(0)?.into_bool()?, arg(1)?, arg(2)?),
        Opcode::Selectif => choose(state.has_iflag(inst.cond_code().unwrap()), arg(1)?, arg(2)?),
        Opcode::SelectifSpectreGuard => unimplemented!("SelectifSpectreGuard"),
        Opcode::Bitselect => {
            let mask_a = Value::and(arg(0)?, arg(1)?)?;
            let mask_b = Value::and(Value::not(arg(0)?)?, arg(2)?)?;
            assign(Value::or(mask_a, mask_b)?)
        }
        Opcode::Copy => assign(arg(0)?),
        Opcode::IfcmpSp => unimplemented!("IfcmpSp"),
        Opcode::Icmp => assign(icmp(
            ctrl_ty,
            inst.cond_code().unwrap(),
            &arg(0)?,
            &arg(1)?,
        )?),
        Opcode::IcmpImm => assign(icmp(
            ctrl_ty,
            inst.cond_code().unwrap(),
            &arg(0)?,
            &imm_as_ctrl_ty()?,
        )?),
        Opcode::Ifcmp | Opcode::IfcmpImm => {
            let arg0 = arg(0)?;
            let arg1 = match inst.opcode() {
                Opcode::Ifcmp => arg(1)?,
                Opcode::IfcmpImm => imm_as_ctrl_ty()?,
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
                if icmp(ctrl_ty, *f, &arg0, &arg1)?.into_bool()? {
                    state.set_iflag(*f);
                }
            }
            ControlFlow::Continue
        }
        Opcode::Imin => choose(Value::gt(&arg(1)?, &arg(0)?)?, arg(0)?, arg(1)?),
        Opcode::Umin => choose(
            Value::gt(
                &arg(1)?.convert(ValueConversionKind::ToUnsigned)?,
                &arg(0)?.convert(ValueConversionKind::ToUnsigned)?,
            )?,
            arg(0)?,
            arg(1)?,
        ),
        Opcode::Imax => choose(Value::gt(&arg(0)?, &arg(1)?)?, arg(0)?, arg(1)?),
        Opcode::Umax => choose(
            Value::gt(
                &arg(0)?.convert(ValueConversionKind::ToUnsigned)?,
                &arg(1)?.convert(ValueConversionKind::ToUnsigned)?,
            )?,
            arg(0)?,
            arg(1)?,
        ),
        Opcode::AvgRound => {
            let sum = Value::add(arg(0)?, arg(1)?)?;
            let one = Value::int(1, arg(0)?.ty())?;
            let inc = Value::add(sum, one)?;
            let two = Value::int(2, arg(0)?.ty())?;
            binary(Value::div, inc, two)?
        }
        Opcode::Iadd => binary(Value::add, arg(0)?, arg(1)?)?,
        Opcode::UaddSat => assign(binary_arith(
            arg(0)?,
            arg(1)?,
            ctrl_ty,
            Value::add_sat,
            true,
        )?),
        Opcode::SaddSat => assign(binary_arith(
            arg(0)?,
            arg(1)?,
            ctrl_ty,
            Value::add_sat,
            false,
        )?),
        Opcode::Isub => binary(Value::sub, arg(0)?, arg(1)?)?,
        Opcode::UsubSat => assign(binary_arith(
            arg(0)?,
            arg(1)?,
            ctrl_ty,
            Value::sub_sat,
            true,
        )?),
        Opcode::SsubSat => assign(binary_arith(
            arg(0)?,
            arg(1)?,
            ctrl_ty,
            Value::sub_sat,
            false,
        )?),
        Opcode::Ineg => binary(Value::sub, Value::int(0, ctrl_ty)?, arg(0)?)?,
        Opcode::Iabs => {
            let (min_val, _) = ctrl_ty.lane_type().bounds(true);
            let min_val: V = Value::int(min_val as i128, ctrl_ty.lane_type())?;
            let arg0 = extractlanes(&arg(0)?, ctrl_ty.lane_type())?;
            let new_vec = arg0
                .into_iter()
                .map(|lane| {
                    if Value::eq(&lane, &min_val)? {
                        Ok(min_val.clone())
                    } else {
                        Value::int(lane.into_int()?.abs(), ctrl_ty.lane_type())
                    }
                })
                .collect::<ValueResult<SimdVec<V>>>()?;
            assign(vectorizelanes(&new_vec, ctrl_ty)?)
        }
        Opcode::Imul => binary(Value::mul, arg(0)?, arg(1)?)?,
        Opcode::Umulhi | Opcode::Smulhi => {
            let double_length = match ctrl_ty.lane_bits() {
                8 => types::I16,
                16 => types::I32,
                32 => types::I64,
                64 => types::I128,
                _ => unimplemented!("Unsupported integer length {}", ctrl_ty.bits()),
            };
            let conv_type = if inst.opcode() == Opcode::Umulhi {
                ValueConversionKind::ZeroExtend(double_length)
            } else {
                ValueConversionKind::SignExtend(double_length)
            };
            let arg0 = extractlanes(&arg(0)?, ctrl_ty.lane_type())?;
            let arg1 = extractlanes(&arg(1)?, ctrl_ty.lane_type())?;

            let res = arg0
                .into_iter()
                .zip(arg1)
                .map(|(x, y)| {
                    let x = x.convert(conv_type.clone())?;
                    let y = y.convert(conv_type.clone())?;

                    Ok(Value::mul(x, y)?
                        .convert(ValueConversionKind::ExtractUpper(ctrl_ty.lane_type()))?)
                })
                .collect::<ValueResult<SimdVec<V>>>()?;

            assign(vectorizelanes(&res, ctrl_ty)?)
        }
        Opcode::Udiv => binary_unsigned_can_trap(Value::div, arg(0)?, arg(1)?)?,
        Opcode::Sdiv => binary_can_trap(Value::div, arg(0)?, arg(1)?)?,
        Opcode::Urem => binary_unsigned_can_trap(Value::rem, arg(0)?, arg(1)?)?,
        Opcode::Srem => binary_can_trap(Value::rem, arg(0)?, arg(1)?)?,
        Opcode::IaddImm => binary(Value::add, arg(0)?, imm_as_ctrl_ty()?)?,
        Opcode::ImulImm => binary(Value::mul, arg(0)?, imm_as_ctrl_ty()?)?,
        Opcode::UdivImm => binary_unsigned_can_trap(Value::div, arg(0)?, imm())?,
        Opcode::SdivImm => binary_can_trap(Value::div, arg(0)?, imm_as_ctrl_ty()?)?,
        Opcode::UremImm => binary_unsigned_can_trap(Value::rem, arg(0)?, imm())?,
        Opcode::SremImm => binary_can_trap(Value::rem, arg(0)?, imm_as_ctrl_ty()?)?,
        Opcode::IrsubImm => binary(Value::sub, imm_as_ctrl_ty()?, arg(0)?)?,
        Opcode::IaddCin => choose(
            Value::into_bool(arg(2)?)?,
            Value::add(Value::add(arg(0)?, arg(1)?)?, Value::int(1, ctrl_ty)?)?,
            Value::add(arg(0)?, arg(1)?)?,
        ),
        Opcode::IaddIfcin => unimplemented!("IaddIfcin"),
        Opcode::IaddCout => {
            let sum = Value::add(arg(0)?, arg(1)?)?;
            let carry = Value::lt(&sum, &arg(0)?)? && Value::lt(&sum, &arg(1)?)?;
            assign_multiple(&[sum, Value::bool(carry, types::B1)?])
        }
        Opcode::IaddIfcout => unimplemented!("IaddIfcout"),
        Opcode::IaddCarry => {
            let mut sum = Value::add(arg(0)?, arg(1)?)?;
            if Value::into_bool(arg(2)?)? {
                sum = Value::add(sum, Value::int(1, ctrl_ty)?)?
            }
            let carry = Value::lt(&sum, &arg(0)?)? && Value::lt(&sum, &arg(1)?)?;
            assign_multiple(&[sum, Value::bool(carry, types::B1)?])
        }
        Opcode::IaddIfcarry => unimplemented!("IaddIfcarry"),
        Opcode::IsubBin => choose(
            Value::into_bool(arg(2)?)?,
            Value::sub(arg(0)?, Value::add(arg(1)?, Value::int(1, ctrl_ty)?)?)?,
            Value::sub(arg(0)?, arg(1)?)?,
        ),
        Opcode::IsubIfbin => unimplemented!("IsubIfbin"),
        Opcode::IsubBout => {
            let sum = Value::sub(arg(0)?, arg(1)?)?;
            let borrow = Value::lt(&arg(0)?, &arg(1)?)?;
            assign_multiple(&[sum, Value::bool(borrow, types::B1)?])
        }
        Opcode::IsubIfbout => unimplemented!("IsubIfbout"),
        Opcode::IsubBorrow => {
            let rhs = if Value::into_bool(arg(2)?)? {
                Value::add(arg(1)?, Value::int(1, ctrl_ty)?)?
            } else {
                arg(1)?
            };
            let borrow = Value::lt(&arg(0)?, &rhs)?;
            let sum = Value::sub(arg(0)?, rhs)?;
            assign_multiple(&[sum, Value::bool(borrow, types::B1)?])
        }
        Opcode::IsubIfborrow => unimplemented!("IsubIfborrow"),
        Opcode::Band => binary(Value::and, arg(0)?, arg(1)?)?,
        Opcode::Bor => binary(Value::or, arg(0)?, arg(1)?)?,
        Opcode::Bxor => binary(Value::xor, arg(0)?, arg(1)?)?,
        Opcode::Bnot => assign(Value::not(arg(0)?)?),
        Opcode::BandNot => binary(Value::and, arg(0)?, Value::not(arg(1)?)?)?,
        Opcode::BorNot => binary(Value::or, arg(0)?, Value::not(arg(1)?)?)?,
        Opcode::BxorNot => binary(Value::xor, arg(0)?, Value::not(arg(1)?)?)?,
        Opcode::BandImm => binary(Value::and, arg(0)?, imm_as_ctrl_ty()?)?,
        Opcode::BorImm => binary(Value::or, arg(0)?, imm_as_ctrl_ty()?)?,
        Opcode::BxorImm => binary(Value::xor, arg(0)?, imm_as_ctrl_ty()?)?,
        Opcode::Rotl => binary(Value::rotl, arg(0)?, arg(1)?)?,
        Opcode::Rotr => binary(Value::rotr, arg(0)?, arg(1)?)?,
        Opcode::RotlImm => binary(Value::rotl, arg(0)?, imm_as_ctrl_ty()?)?,
        Opcode::RotrImm => binary(Value::rotr, arg(0)?, imm_as_ctrl_ty()?)?,
        Opcode::Ishl => binary(Value::shl, arg(0)?, arg(1)?)?,
        Opcode::Ushr => binary(Value::ushr, arg(0)?, arg(1)?)?,
        Opcode::Sshr => binary(Value::ishr, arg(0)?, arg(1)?)?,
        Opcode::IshlImm => binary(Value::shl, arg(0)?, imm_as_ctrl_ty()?)?,
        Opcode::UshrImm => binary(Value::ushr, arg(0)?, imm_as_ctrl_ty()?)?,
        Opcode::SshrImm => binary(Value::ishr, arg(0)?, imm_as_ctrl_ty()?)?,
        Opcode::Bitrev => assign(Value::reverse_bits(arg(0)?)?),
        Opcode::Clz => assign(arg(0)?.leading_zeros()?),
        Opcode::Cls => {
            let count = if Value::lt(&arg(0)?, &Value::int(0, ctrl_ty)?)? {
                arg(0)?.leading_ones()?
            } else {
                arg(0)?.leading_zeros()?
            };
            assign(Value::sub(count, Value::int(1, ctrl_ty)?)?)
        }
        Opcode::Ctz => assign(arg(0)?.trailing_zeros()?),
        Opcode::Popcnt => {
            let count = if arg(0)?.ty().is_int() {
                arg(0)?.count_ones()?
            } else {
                let lanes = extractlanes(&arg(0)?, ctrl_ty.lane_type())?
                    .into_iter()
                    .map(|lane| lane.count_ones())
                    .collect::<ValueResult<SimdVec<V>>>()?;
                vectorizelanes(&lanes, ctrl_ty)?
            };
            assign(count)
        }
        Opcode::Fcmp => assign(Value::bool(
            fcmp(inst.fp_cond_code().unwrap(), &arg(0)?, &arg(1)?)?,
            ctrl_ty.as_bool(),
        )?),
        Opcode::Ffcmp => {
            let arg0 = arg(0)?;
            let arg1 = arg(1)?;
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
                if fcmp(*f, &arg0, &arg1)? {
                    state.set_fflag(*f);
                }
            }
            ControlFlow::Continue
        }
        Opcode::Fadd => binary(Value::add, arg(0)?, arg(1)?)?,
        Opcode::Fsub => binary(Value::sub, arg(0)?, arg(1)?)?,
        Opcode::Fmul => binary(Value::mul, arg(0)?, arg(1)?)?,
        Opcode::Fdiv => binary(Value::div, arg(0)?, arg(1)?)?,
        Opcode::Sqrt => unimplemented!("Sqrt"),
        Opcode::Fma => unimplemented!("Fma"),
        Opcode::Fneg => binary(Value::sub, Value::float(0, ctrl_ty)?, arg(0)?)?,
        Opcode::Fabs => unimplemented!("Fabs"),
        Opcode::Fcopysign => unimplemented!("Fcopysign"),
        Opcode::Fmin => choose(
            Value::is_nan(&arg(0)?)? || Value::lt(&arg(0)?, &arg(1)?)?,
            arg(0)?,
            arg(1)?,
        ),
        Opcode::FminPseudo => unimplemented!("FminPseudo"),
        Opcode::Fmax => choose(
            Value::is_nan(&arg(0)?)? || Value::gt(&arg(0)?, &arg(1)?)?,
            arg(0)?,
            arg(1)?,
        ),
        Opcode::FmaxPseudo => unimplemented!("FmaxPseudo"),
        Opcode::Ceil => unimplemented!("Ceil"),
        Opcode::Floor => unimplemented!("Floor"),
        Opcode::Trunc => unimplemented!("Trunc"),
        Opcode::Nearest => unimplemented!("Nearest"),
        Opcode::IsNull => unimplemented!("IsNull"),
        Opcode::IsInvalid => unimplemented!("IsInvalid"),
        Opcode::Trueif => choose(
            state.has_iflag(inst.cond_code().unwrap()),
            Value::bool(true, ctrl_ty)?,
            Value::bool(false, ctrl_ty)?,
        ),
        Opcode::Trueff => choose(
            state.has_fflag(inst.fp_cond_code().unwrap()),
            Value::bool(true, ctrl_ty)?,
            Value::bool(false, ctrl_ty)?,
        ),
        Opcode::Bitcast
        | Opcode::RawBitcast
        | Opcode::ScalarToVector
        | Opcode::Breduce
        | Opcode::Bextend
        | Opcode::Ireduce => assign(Value::convert(
            arg(0)?,
            ValueConversionKind::Exact(ctrl_ty),
        )?),
        Opcode::Bint => {
            let bool = arg(0)?.into_bool()?;
            let int = if bool { 1 } else { 0 };
            assign(Value::int(int, ctrl_ty)?)
        }
        Opcode::Snarrow | Opcode::Unarrow | Opcode::Uunarrow => {
            let arg0 = extractlanes(&arg(0)?, ctrl_ty.lane_type())?;
            let arg1 = extractlanes(&arg(1)?, ctrl_ty.lane_type())?;
            let new_type = ctrl_ty.split_lanes().unwrap();
            let (min, max) = new_type.bounds(inst.opcode() == Opcode::Snarrow);
            let mut min: V = Value::int(min as i128, ctrl_ty.lane_type())?;
            let mut max: V = Value::int(max as i128, ctrl_ty.lane_type())?;
            if inst.opcode() == Opcode::Uunarrow {
                min = min.convert(ValueConversionKind::ToUnsigned)?;
                max = max.convert(ValueConversionKind::ToUnsigned)?;
            }
            let narrow = |mut lane: V| -> ValueResult<V> {
                if inst.opcode() == Opcode::Uunarrow {
                    lane = lane.convert(ValueConversionKind::ToUnsigned)?;
                }
                lane = Value::max(lane, min.clone())?;
                lane = Value::min(lane, max.clone())?;
                lane = lane.convert(ValueConversionKind::Truncate(new_type.lane_type()))?;
                if inst.opcode() == Opcode::Unarrow || inst.opcode() == Opcode::Uunarrow {
                    lane = lane.convert(ValueConversionKind::ToUnsigned)?;
                }
                Ok(lane)
            };
            let new_vec = arg0
                .into_iter()
                .chain(arg1)
                .map(|lane| narrow(lane))
                .collect::<ValueResult<Vec<_>>>()?;
            assign(vectorizelanes(&new_vec, new_type)?)
        }
        Opcode::Bmask => assign({
            let bool = arg(0)?;
            let bool_ty = ctrl_ty.as_bool_pedantic();
            let lanes = extractlanes(&bool, bool_ty.lane_type())?
                .into_iter()
                .map(|lane| lane.convert(ValueConversionKind::Exact(ctrl_ty.lane_type())))
                .collect::<ValueResult<SimdVec<V>>>()?;
            vectorizelanes(&lanes, ctrl_ty)?
        }),
        Opcode::Sextend => assign(Value::convert(
            arg(0)?,
            ValueConversionKind::SignExtend(ctrl_ty),
        )?),
        Opcode::Uextend => assign(Value::convert(
            arg(0)?,
            ValueConversionKind::ZeroExtend(ctrl_ty),
        )?),
        Opcode::Fpromote => assign(Value::convert(
            arg(0)?,
            ValueConversionKind::Exact(ctrl_ty),
        )?),
        Opcode::Fdemote => assign(Value::convert(
            arg(0)?,
            ValueConversionKind::RoundNearestEven(ctrl_ty),
        )?),
        Opcode::Shuffle => {
            let mask = imm().into_array()?;
            let a = Value::into_array(&arg(0)?)?;
            let b = Value::into_array(&arg(1)?)?;
            let mut new = [0u8; 16];
            for i in 0..mask.len() {
                if (mask[i] as usize) < a.len() {
                    new[i] = a[mask[i] as usize];
                } else if (mask[i] as usize - a.len()) < b.len() {
                    new[i] = b[mask[i] as usize - a.len()];
                } // else leave as 0.
            }
            assign(Value::vector(new, ctrl_ty)?)
        }
        Opcode::Swizzle => {
            let x = Value::into_array(&arg(0)?)?;
            let s = Value::into_array(&arg(1)?)?;
            let mut new = [0u8; 16];
            for i in 0..new.len() {
                if (s[i] as usize) < new.len() {
                    new[i] = x[s[i] as usize];
                } // else leave as 0
            }
            assign(Value::vector(new, ctrl_ty)?)
        }
        Opcode::Splat => {
            let mut new_vector = SimdVec::new();
            for _ in 0..ctrl_ty.lane_count() {
                new_vector.push(arg(0)?);
            }
            assign(vectorizelanes(&new_vector, ctrl_ty)?)
        }
        Opcode::Insertlane => {
            let idx = imm().into_int()? as usize;
            let mut vector = extractlanes(&arg(0)?, ctrl_ty.lane_type())?;
            vector[idx] = arg(1)?;
            assign(vectorizelanes(&vector, ctrl_ty)?)
        }
        Opcode::Extractlane => {
            let idx = imm().into_int()? as usize;
            let lanes = extractlanes(&arg(0)?, ctrl_ty.lane_type())?;
            assign(lanes[idx].clone())
        }
        Opcode::VhighBits => {
            // `ctrl_ty` controls the return type for this, so the input type
            // must be retrieved via `inst_context`.
            let lane_type = inst_context
                .type_of(inst_context.args()[0])
                .unwrap()
                .lane_type();
            let a = extractlanes(&arg(0)?, lane_type)?;
            let mut result: i128 = 0;
            for (i, val) in a.into_iter().enumerate() {
                let val = val.reverse_bits()?.into_int()?; // MSB -> LSB
                result |= (val & 1) << i;
            }
            assign(Value::int(result, ctrl_ty)?)
        }
        Opcode::Vsplit => unimplemented!("Vsplit"),
        Opcode::Vconcat => unimplemented!("Vconcat"),
        Opcode::Vselect => {
            let c = extractlanes(&arg(0)?, ctrl_ty.lane_type())?;
            let x = extractlanes(&arg(1)?, ctrl_ty.lane_type())?;
            let y = extractlanes(&arg(2)?, ctrl_ty.lane_type())?;
            let mut new_vec = SimdVec::new();
            for (c, (x, y)) in c.into_iter().zip(x.into_iter().zip(y.into_iter())) {
                if Value::eq(&c, &Value::int(0, ctrl_ty.lane_type())?)? {
                    new_vec.push(y);
                } else {
                    new_vec.push(x);
                }
            }
            assign(vectorizelanes(&new_vec, ctrl_ty)?)
        }
        Opcode::VanyTrue => assign(fold_vector(
            arg(0)?,
            ctrl_ty,
            V::bool(false, types::B1)?,
            |acc, lane| acc.or(lane),
        )?),
        Opcode::VallTrue => assign(fold_vector(
            arg(0)?,
            ctrl_ty,
            V::bool(true, types::B1)?,
            |acc, lane| acc.and(lane),
        )?),
        Opcode::SwidenLow | Opcode::SwidenHigh | Opcode::UwidenLow | Opcode::UwidenHigh => {
            let new_type = ctrl_ty.merge_lanes().unwrap();
            let conv_type = match inst.opcode() {
                Opcode::SwidenLow | Opcode::SwidenHigh => {
                    ValueConversionKind::SignExtend(new_type.lane_type())
                }
                Opcode::UwidenLow | Opcode::UwidenHigh => {
                    ValueConversionKind::ZeroExtend(new_type.lane_type())
                }
                _ => unreachable!(),
            };
            let vec_iter = extractlanes(&arg(0)?, ctrl_ty.lane_type())?.into_iter();
            let new_vec = match inst.opcode() {
                Opcode::SwidenLow | Opcode::UwidenLow => vec_iter
                    .take(new_type.lane_count() as usize)
                    .map(|lane| lane.convert(conv_type.clone()))
                    .collect::<ValueResult<Vec<_>>>()?,
                Opcode::SwidenHigh | Opcode::UwidenHigh => vec_iter
                    .skip(new_type.lane_count() as usize)
                    .map(|lane| lane.convert(conv_type.clone()))
                    .collect::<ValueResult<Vec<_>>>()?,
                _ => unreachable!(),
            };
            assign(vectorizelanes(&new_vec, new_type)?)
        }
        Opcode::FcvtToUint => unimplemented!("FcvtToUint"),
        Opcode::FcvtToUintSat => unimplemented!("FcvtToUintSat"),
        Opcode::FcvtToSint => unimplemented!("FcvtToSint"),
        Opcode::FcvtToSintSat => unimplemented!("FcvtToSintSat"),
        Opcode::FcvtFromUint => unimplemented!("FcvtFromUint"),
        Opcode::FcvtFromSint => unimplemented!("FcvtFromSint"),
        Opcode::FcvtLowFromSint => unimplemented!("FcvtLowFromSint"),
        Opcode::FvpromoteLow => unimplemented!("FvpromoteLow"),
        Opcode::Fvdemote => unimplemented!("Fvdemote"),
        Opcode::Isplit => assign_multiple(&[
            Value::convert(arg(0)?, ValueConversionKind::Truncate(types::I64))?,
            Value::convert(arg(0)?, ValueConversionKind::ExtractUpper(types::I64))?,
        ]),
        Opcode::Iconcat => assign(Value::concat(arg(0)?, arg(1)?)?),
        Opcode::AtomicRmw => unimplemented!("AtomicRmw"),
        Opcode::AtomicCas => unimplemented!("AtomicCas"),
        Opcode::AtomicLoad => unimplemented!("AtomicLoad"),
        Opcode::AtomicStore => unimplemented!("AtomicStore"),
        Opcode::Fence => unimplemented!("Fence"),
        Opcode::WideningPairwiseDotProductS => {
            let ctrl_ty = types::I16X8;
            let new_type = ctrl_ty.merge_lanes().unwrap();
            let arg0 = extractlanes(&arg(0)?, ctrl_ty.lane_type())?;
            let arg1 = extractlanes(&arg(1)?, ctrl_ty.lane_type())?;
            let new_vec = arg0
                .chunks(2)
                .into_iter()
                .zip(arg1.chunks(2))
                .into_iter()
                .map(|(x, y)| {
                    let mut z = 0i128;
                    for (lhs, rhs) in x.into_iter().zip(y.into_iter()) {
                        z += lhs.clone().into_int()? * rhs.clone().into_int()?;
                    }
                    Value::int(z, new_type.lane_type())
                })
                .collect::<ValueResult<Vec<_>>>()?;
            assign(vectorizelanes(&new_vec, new_type)?)
        }
        Opcode::SqmulRoundSat => {
            let lane_type = ctrl_ty.lane_type();
            let double_width = ctrl_ty.double_width().unwrap().lane_type();
            let arg0 = extractlanes(&arg(0)?, lane_type)?;
            let arg1 = extractlanes(&arg(1)?, lane_type)?;
            let (min, max) = lane_type.bounds(true);
            let min: V = Value::int(min as i128, double_width)?;
            let max: V = Value::int(max as i128, double_width)?;
            let new_vec = arg0
                .into_iter()
                .zip(arg1.into_iter())
                .map(|(x, y)| {
                    let x = x.into_int()?;
                    let y = y.into_int()?;
                    // temporarily double width of the value to avoid overflow.
                    let z: V = Value::int(
                        (x * y + (1 << (lane_type.bits() - 2))) >> (lane_type.bits() - 1),
                        double_width,
                    )?;
                    // check bounds, saturate, and truncate to correct width.
                    let z = Value::min(z, max.clone())?;
                    let z = Value::max(z, min.clone())?;
                    let z = z.convert(ValueConversionKind::Truncate(lane_type))?;
                    Ok(z)
                })
                .collect::<ValueResult<SimdVec<_>>>()?;
            assign(vectorizelanes(&new_vec, ctrl_ty)?)
        }
        Opcode::IaddPairwise => assign(binary_pairwise(arg(0)?, arg(1)?, ctrl_ty, Value::add)?),
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

    /// For convenience, we can unwrap the [ControlFlow] state assuming that it is a
    /// [ControlFlow::Trap], panicking otherwise.
    pub fn unwrap_trap(self) -> CraneliftTrap {
        if let ControlFlow::Trap(trap) = self {
            trap
        } else {
            panic!("expected the control flow to be a trap")
        }
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum CraneliftTrap {
    #[error("user code: {0}")]
    User(TrapCode),
    #[error("user debug")]
    Debug,
    #[error("resumable")]
    Resumable,
}

/// Compare two values using the given integer condition `code`.
fn icmp<V>(ctrl_ty: types::Type, code: IntCC, left: &V, right: &V) -> ValueResult<V>
where
    V: Value,
{
    let cmp = |bool_ty: types::Type, code: IntCC, left: &V, right: &V| -> ValueResult<V> {
        Ok(Value::bool(
            match code {
                IntCC::Equal => Value::eq(left, right)?,
                IntCC::NotEqual => !Value::eq(left, right)?,
                IntCC::SignedGreaterThan => Value::gt(left, right)?,
                IntCC::SignedGreaterThanOrEqual => Value::ge(left, right)?,
                IntCC::SignedLessThan => Value::lt(left, right)?,
                IntCC::SignedLessThanOrEqual => Value::le(left, right)?,
                IntCC::UnsignedGreaterThan => Value::gt(
                    &left.clone().convert(ValueConversionKind::ToUnsigned)?,
                    &right.clone().convert(ValueConversionKind::ToUnsigned)?,
                )?,
                IntCC::UnsignedGreaterThanOrEqual => Value::ge(
                    &left.clone().convert(ValueConversionKind::ToUnsigned)?,
                    &right.clone().convert(ValueConversionKind::ToUnsigned)?,
                )?,
                IntCC::UnsignedLessThan => Value::lt(
                    &left.clone().convert(ValueConversionKind::ToUnsigned)?,
                    &right.clone().convert(ValueConversionKind::ToUnsigned)?,
                )?,
                IntCC::UnsignedLessThanOrEqual => Value::le(
                    &left.clone().convert(ValueConversionKind::ToUnsigned)?,
                    &right.clone().convert(ValueConversionKind::ToUnsigned)?,
                )?,
                IntCC::Overflow => Value::overflow(left, right)?,
                IntCC::NotOverflow => !Value::overflow(left, right)?,
            },
            bool_ty,
        )?)
    };

    let dst_ty = ctrl_ty.as_bool();
    let lane_type = ctrl_ty.lane_type();
    let left = extractlanes(left, lane_type)?;
    let right = extractlanes(right, lane_type)?;

    let res = left
        .into_iter()
        .zip(right.into_iter())
        .map(|(l, r)| cmp(dst_ty.lane_type(), code, &l, &r))
        .collect::<ValueResult<SimdVec<V>>>()?;

    Ok(vectorizelanes(&res, dst_ty)?)
}

/// Compare two values using the given floating point condition `code`.
fn fcmp<V>(code: FloatCC, left: &V, right: &V) -> ValueResult<bool>
where
    V: Value,
{
    Ok(match code {
        FloatCC::Ordered => {
            Value::eq(left, right)? || Value::lt(left, right)? || Value::gt(left, right)?
        }
        FloatCC::Unordered => Value::uno(left, right)?,
        FloatCC::Equal => Value::eq(left, right)?,
        FloatCC::NotEqual => {
            Value::lt(left, right)? || Value::gt(left, right)? || Value::uno(left, right)?
        }
        FloatCC::OrderedNotEqual => Value::lt(left, right)? || Value::gt(left, right)?,
        FloatCC::UnorderedOrEqual => Value::eq(left, right)? || Value::uno(left, right)?,
        FloatCC::LessThan => Value::lt(left, right)?,
        FloatCC::LessThanOrEqual => Value::lt(left, right)? || Value::eq(left, right)?,
        FloatCC::GreaterThan => Value::gt(left, right)?,
        FloatCC::GreaterThanOrEqual => Value::gt(left, right)? || Value::eq(left, right)?,
        FloatCC::UnorderedOrLessThan => Value::uno(left, right)? || Value::lt(left, right)?,
        FloatCC::UnorderedOrLessThanOrEqual => {
            Value::uno(left, right)? || Value::lt(left, right)? || Value::eq(left, right)?
        }
        FloatCC::UnorderedOrGreaterThan => Value::uno(left, right)? || Value::gt(left, right)?,
        FloatCC::UnorderedOrGreaterThanOrEqual => {
            Value::uno(left, right)? || Value::gt(left, right)? || Value::eq(left, right)?
        }
    })
}

type SimdVec<V> = SmallVec<[V; 4]>;

/// Converts a SIMD vector value into a Rust array of [Value] for processing.
/// If `x` is a scalar, it will be returned as a single-element array.
fn extractlanes<V>(x: &V, lane_type: types::Type) -> ValueResult<SimdVec<V>>
where
    V: Value,
{
    let mut lanes = SimdVec::new();
    // Wrap scalar values as a single-element vector and return.
    if !x.ty().is_vector() {
        lanes.push(x.clone());
        return Ok(lanes);
    }

    let iterations = match lane_type {
        types::I8 | types::B1 | types::B8 => 1,
        types::I16 | types::B16 => 2,
        types::I32 | types::B32 => 4,
        types::I64 | types::B64 => 8,
        _ => unimplemented!("Only 128-bit vectors are currently supported."),
    };

    let x = x.into_array()?;
    for (i, _) in x.iter().enumerate() {
        let mut lane: i128 = 0;
        if i % iterations != 0 {
            continue;
        }
        for j in 0..iterations {
            lane += (x[i + j] as i128) << (8 * j);
        }

        let lane_val: V = if lane_type.is_bool() {
            Value::bool(lane != 0, lane_type)?
        } else {
            Value::int(lane, lane_type)?
        };
        lanes.push(lane_val);
    }
    return Ok(lanes);
}

/// Convert a Rust array of [Value] back into a `Value::vector`.
/// Supplying a single-element array will simply return its contained value.
fn vectorizelanes<V>(x: &[V], vector_type: types::Type) -> ValueResult<V>
where
    V: Value,
{
    // If the array is only one element, return it as a scalar.
    if x.len() == 1 {
        return Ok(x[0].clone());
    }

    let lane_type = vector_type.lane_type();
    let iterations = match lane_type {
        types::I8 | types::B1 | types::B8 => 1,
        types::I16 | types::B16 => 2,
        types::I32 | types::B32 => 4,
        types::I64 | types::B64 => 8,
        _ => unimplemented!("Only 128-bit vectors are currently supported."),
    };
    let mut result: [u8; 16] = [0; 16];
    for (i, val) in x.iter().enumerate() {
        let lane_val: i128 = val
            .clone()
            .convert(ValueConversionKind::Exact(lane_type.as_int()))?
            .into_int()?;

        for j in 0..iterations {
            result[(i * iterations) + j] = (lane_val >> (8 * j)) as u8;
        }
    }
    Value::vector(result, vector_type)
}

/// Performs a lanewise fold on a vector type
fn fold_vector<V, F>(v: V, ty: types::Type, init: V, op: F) -> ValueResult<V>
where
    V: Value,
    F: FnMut(V, V) -> ValueResult<V>,
{
    extractlanes(&v, ty.lane_type())?
        .into_iter()
        .try_fold(init, op)
}

/// Performs the supplied binary arithmetic `op` on two SIMD vectors.
fn binary_arith<V, F>(x: V, y: V, vector_type: types::Type, op: F, unsigned: bool) -> ValueResult<V>
where
    V: Value,
    F: Fn(V, V) -> ValueResult<V>,
{
    let arg0 = extractlanes(&x, vector_type.lane_type())?;
    let arg1 = extractlanes(&y, vector_type.lane_type())?;

    let result = arg0
        .into_iter()
        .zip(arg1)
        .map(|(mut lhs, mut rhs)| {
            if unsigned {
                lhs = lhs.convert(ValueConversionKind::ToUnsigned)?;
                rhs = rhs.convert(ValueConversionKind::ToUnsigned)?;
            }
            Ok(op(lhs, rhs)?)
        })
        .collect::<ValueResult<SimdVec<V>>>()?;

    vectorizelanes(&result, vector_type)
}

/// Performs the supplied pairwise arithmetic `op` on two SIMD vectors, where
/// pairs are formed from adjacent vector elements and the vectors are
/// concatenated at the end.
fn binary_pairwise<V, F>(x: V, y: V, vector_type: types::Type, op: F) -> ValueResult<V>
where
    V: Value,
    F: Fn(V, V) -> ValueResult<V>,
{
    let arg0 = extractlanes(&x, vector_type.lane_type())?;
    let arg1 = extractlanes(&y, vector_type.lane_type())?;

    let result = arg0
        .chunks(2)
        .chain(arg1.chunks(2))
        .map(|pair| op(pair[0].clone(), pair[1].clone()))
        .collect::<ValueResult<SimdVec<V>>>()?;

    vectorizelanes(&result, vector_type)
}
