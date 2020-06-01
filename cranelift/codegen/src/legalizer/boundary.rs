//! Legalize ABI boundaries.
//!
//! This legalizer sub-module contains code for dealing with ABI boundaries:
//!
//! - Function arguments passed to the entry block.
//! - Function arguments passed to call instructions.
//! - Return values from call instructions.
//! - Return values passed to return instructions.
//!
//! The ABI boundary legalization happens in two phases:
//!
//! 1. The `legalize_signatures` function rewrites all the preamble signatures with ABI information
//!    and possibly new argument types. It also rewrites the entry block arguments to match.
//! 2. The `handle_call_abi` and `handle_return_abi` functions rewrite call and return instructions
//!    to match the new ABI signatures.
//!
//! Between the two phases, preamble signatures and call/return arguments don't match. This
//! intermediate state doesn't type check.

use crate::abi::{legalize_abi_value, ValueConversion};
use crate::cursor::{Cursor, FuncCursor};
use crate::flowgraph::ControlFlowGraph;
use crate::ir::instructions::CallInfo;
use crate::ir::{
    AbiParam, ArgumentLoc, ArgumentPurpose, Block, DataFlowGraph, Function, Inst, InstBuilder,
    MemFlags, SigRef, Signature, StackSlotData, StackSlotKind, Type, Value, ValueLoc,
};
use crate::isa::TargetIsa;
use crate::legalizer::split::{isplit, vsplit};
use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::mem;
use cranelift_entity::EntityList;
use log::debug;

/// Legalize all the function signatures in `func`.
///
/// This changes all signatures to be ABI-compliant with full `ArgumentLoc` annotations. It doesn't
/// change the entry block arguments, calls, or return instructions, so this can leave the function
/// in a state with type discrepancies.
pub fn legalize_signatures(func: &mut Function, isa: &dyn TargetIsa) {
    if let Some(new) = legalize_signature(&func.signature, true, isa) {
        let old = mem::replace(&mut func.signature, new);
        func.old_signature = Some(old);
    }

    for (sig_ref, sig_data) in func.dfg.signatures.iter_mut() {
        if let Some(new) = legalize_signature(sig_data, false, isa) {
            let old = mem::replace(sig_data, new);
            func.dfg.old_signatures[sig_ref] = Some(old);
        }
    }

    if let Some(entry) = func.layout.entry_block() {
        legalize_entry_params(func, entry);
        spill_entry_params(func, entry);
    }
}

/// Legalize the libcall signature, which we may generate on the fly after
/// `legalize_signatures` has been called.
pub fn legalize_libcall_signature(signature: &mut Signature, isa: &dyn TargetIsa) {
    if let Some(s) = legalize_signature(signature, false, isa) {
        *signature = s;
    }
}

/// Legalize the given signature.
///
/// `current` is true if this is the signature for the current function.
fn legalize_signature(
    signature: &Signature,
    current: bool,
    isa: &dyn TargetIsa,
) -> Option<Signature> {
    let mut cow = Cow::Borrowed(signature);
    isa.legalize_signature(&mut cow, current);
    match cow {
        Cow::Borrowed(_) => None,
        Cow::Owned(s) => Some(s),
    }
}

/// Legalize the entry block parameters after `func`'s signature has been legalized.
///
/// The legalized signature may contain more parameters than the original signature, and the
/// parameter types have been changed. This function goes through the parameters of the entry block
/// and replaces them with parameters of the right type for the ABI.
///
/// The original entry block parameters are computed from the new ABI parameters by code inserted at
/// the top of the entry block.
fn legalize_entry_params(func: &mut Function, entry: Block) {
    let mut has_sret = false;
    let mut has_link = false;
    let mut has_vmctx = false;
    let mut has_sigid = false;
    let mut has_stack_limit = false;

    // Insert position for argument conversion code.
    // We want to insert instructions before the first instruction in the entry block.
    // If the entry block is empty, append instructions to it instead.
    let mut pos = FuncCursor::new(func).at_first_inst(entry);

    // Keep track of the argument types in the ABI-legalized signature.
    let mut abi_arg = 0;

    // Process the block parameters one at a time, possibly replacing one argument with multiple new
    // ones. We do this by detaching the entry block parameters first.
    let block_params = pos.func.dfg.detach_block_params(entry);
    let mut old_arg = 0;
    while let Some(arg) = block_params.get(old_arg, &pos.func.dfg.value_lists) {
        old_arg += 1;

        let abi_type = pos.func.signature.params[abi_arg];
        let arg_type = pos.func.dfg.value_type(arg);
        if arg_type == abi_type.value_type {
            // No value translation is necessary, this argument matches the ABI type.
            // Just use the original block argument value. This is the most common case.
            pos.func.dfg.attach_block_param(entry, arg);
            match abi_type.purpose {
                ArgumentPurpose::Normal => {}
                ArgumentPurpose::FramePointer => {}
                ArgumentPurpose::CalleeSaved => {}
                ArgumentPurpose::StructReturn => {
                    debug_assert!(!has_sret, "Multiple sret arguments found");
                    has_sret = true;
                }
                ArgumentPurpose::VMContext => {
                    debug_assert!(!has_vmctx, "Multiple vmctx arguments found");
                    has_vmctx = true;
                }
                ArgumentPurpose::SignatureId => {
                    debug_assert!(!has_sigid, "Multiple sigid arguments found");
                    has_sigid = true;
                }
                ArgumentPurpose::StackLimit => {
                    debug_assert!(!has_stack_limit, "Multiple stack_limit arguments found");
                    has_stack_limit = true;
                }
                _ => panic!("Unexpected special-purpose arg {}", abi_type),
            }
            abi_arg += 1;
        } else {
            // Compute the value we want for `arg` from the legalized ABI parameters.
            let mut get_arg = |func: &mut Function, ty| {
                let abi_type = func.signature.params[abi_arg];
                debug_assert_eq!(
                    abi_type.purpose,
                    ArgumentPurpose::Normal,
                    "Can't legalize special-purpose argument"
                );
                if ty == abi_type.value_type {
                    abi_arg += 1;
                    Ok(func.dfg.append_block_param(entry, ty))
                } else {
                    Err(abi_type)
                }
            };
            let converted = convert_from_abi(&mut pos, arg_type, Some(arg), &mut get_arg);
            // The old `arg` is no longer an attached block argument, but there are probably still
            // uses of the value.
            debug_assert_eq!(pos.func.dfg.resolve_aliases(arg), converted);
        }
    }

    // The legalized signature may contain additional parameters representing special-purpose
    // registers.
    for &arg in &pos.func.signature.params[abi_arg..] {
        match arg.purpose {
            // Any normal parameters should have been processed above.
            ArgumentPurpose::Normal => {
                panic!("Leftover arg: {}", arg);
            }
            // The callee-save parameters should not appear until after register allocation is
            // done.
            ArgumentPurpose::FramePointer | ArgumentPurpose::CalleeSaved => {
                panic!("Premature callee-saved arg {}", arg);
            }
            // These can be meaningfully added by `legalize_signature()`.
            ArgumentPurpose::Link => {
                debug_assert!(!has_link, "Multiple link parameters found");
                has_link = true;
            }
            ArgumentPurpose::StructReturn => {
                debug_assert!(!has_sret, "Multiple sret parameters found");
                has_sret = true;
            }
            ArgumentPurpose::VMContext => {
                debug_assert!(!has_vmctx, "Multiple vmctx parameters found");
                has_vmctx = true;
            }
            ArgumentPurpose::SignatureId => {
                debug_assert!(!has_sigid, "Multiple sigid parameters found");
                has_sigid = true;
            }
            ArgumentPurpose::StackLimit => {
                debug_assert!(!has_stack_limit, "Multiple stack_limit parameters found");
                has_stack_limit = true;
            }
        }

        // Just create entry block values to match here. We will use them in `handle_return_abi()`
        // below.
        pos.func.dfg.append_block_param(entry, arg.value_type);
    }
}

/// Legalize the results returned from a call instruction to match the ABI signature.
///
/// The cursor `pos` points to a call instruction with at least one return value. The cursor will
/// be left pointing after the instructions inserted to convert the return values.
///
/// This function is very similar to the `legalize_entry_params` function above.
///
/// Returns the possibly new instruction representing the call.
fn legalize_inst_results<ResType>(pos: &mut FuncCursor, mut get_abi_type: ResType) -> Inst
where
    ResType: FnMut(&Function, usize) -> AbiParam,
{
    let call = pos
        .current_inst()
        .expect("Cursor must point to a call instruction");

    // We theoretically allow for call instructions that return a number of fixed results before
    // the call return values. In practice, it doesn't happen.
    debug_assert_eq!(
        pos.func.dfg[call]
            .opcode()
            .constraints()
            .num_fixed_results(),
        0,
        "Fixed results on calls not supported"
    );

    let results = pos.func.dfg.detach_results(call);
    let mut next_res = 0;
    let mut abi_res = 0;

    // Point immediately after the call.
    pos.next_inst();

    while let Some(res) = results.get(next_res, &pos.func.dfg.value_lists) {
        next_res += 1;

        let res_type = pos.func.dfg.value_type(res);
        if res_type == get_abi_type(pos.func, abi_res).value_type {
            // No value translation is necessary, this result matches the ABI type.
            pos.func.dfg.attach_result(call, res);
            abi_res += 1;
        } else {
            let mut get_res = |func: &mut Function, ty| {
                let abi_type = get_abi_type(func, abi_res);
                if ty == abi_type.value_type {
                    let last_res = func.dfg.append_result(call, ty);
                    abi_res += 1;
                    Ok(last_res)
                } else {
                    Err(abi_type)
                }
            };
            let v = convert_from_abi(pos, res_type, Some(res), &mut get_res);
            debug_assert_eq!(pos.func.dfg.resolve_aliases(res), v);
        }
    }

    call
}

fn assert_is_valid_sret_legalization(
    old_ret_list: &EntityList<Value>,
    old_sig: &Signature,
    new_sig: &Signature,
    pos: &FuncCursor,
) {
    debug_assert_eq!(
        old_sig.returns.len(),
        old_ret_list.len(&pos.func.dfg.value_lists)
    );

    // Assert that the only difference in special parameters is that there
    // is an appended struct return pointer parameter.
    let old_special_params: Vec<_> = old_sig
        .params
        .iter()
        .filter(|r| r.purpose != ArgumentPurpose::Normal)
        .collect();
    let new_special_params: Vec<_> = new_sig
        .params
        .iter()
        .filter(|r| r.purpose != ArgumentPurpose::Normal)
        .collect();
    debug_assert_eq!(old_special_params.len() + 1, new_special_params.len());
    debug_assert!(old_special_params
        .iter()
        .zip(&new_special_params)
        .all(|(old, new)| old.purpose == new.purpose));
    debug_assert_eq!(
        new_special_params.last().unwrap().purpose,
        ArgumentPurpose::StructReturn
    );

    // If the special returns have changed at all, then the only change
    // should be that the struct return pointer is returned back out of the
    // function, so that callers don't have to load its stack address again.
    let old_special_returns: Vec<_> = old_sig
        .returns
        .iter()
        .filter(|r| r.purpose != ArgumentPurpose::Normal)
        .collect();
    let new_special_returns: Vec<_> = new_sig
        .returns
        .iter()
        .filter(|r| r.purpose != ArgumentPurpose::Normal)
        .collect();
    debug_assert!(old_special_returns
        .iter()
        .zip(&new_special_returns)
        .all(|(old, new)| old.purpose == new.purpose));
    debug_assert!(
        old_special_returns.len() == new_special_returns.len()
            || (old_special_returns.len() + 1 == new_special_returns.len()
                && new_special_returns.last().unwrap().purpose == ArgumentPurpose::StructReturn)
    );
}

fn legalize_sret_call(isa: &dyn TargetIsa, pos: &mut FuncCursor, sig_ref: SigRef, call: Inst) {
    let old_ret_list = pos.func.dfg.detach_results(call);
    let old_sig = pos.func.dfg.old_signatures[sig_ref]
        .take()
        .expect("must have an old signature when using an `sret` parameter");

    // We make a bunch of assumptions about the shape of the old, multi-return
    // signature and the new, sret-using signature in this legalization
    // function. Assert that these assumptions hold true in debug mode.
    if cfg!(debug_assertions) {
        assert_is_valid_sret_legalization(
            &old_ret_list,
            &old_sig,
            &pos.func.dfg.signatures[sig_ref],
            &pos,
        );
    }

    // Go through and remove all normal return values from the `call`
    // instruction's returns list. These will be stored into the stack slot that
    // the sret points to. At the same time, calculate the size of the sret
    // stack slot.
    let mut sret_slot_size = 0;
    for (i, ret) in old_sig.returns.iter().enumerate() {
        let v = old_ret_list.get(i, &pos.func.dfg.value_lists).unwrap();
        let ty = pos.func.dfg.value_type(v);
        if ret.purpose == ArgumentPurpose::Normal {
            debug_assert_eq!(ret.location, ArgumentLoc::Unassigned);
            let ty = legalized_type_for_sret(ty);
            let size = ty.bytes();
            sret_slot_size = round_up_to_multiple_of_type_align(sret_slot_size, ty) + size;
        } else {
            let new_v = pos.func.dfg.append_result(call, ty);
            pos.func.dfg.change_to_alias(v, new_v);
        }
    }

    let stack_slot = pos.func.stack_slots.push(StackSlotData {
        kind: StackSlotKind::StructReturnSlot,
        size: sret_slot_size,
        offset: None,
    });

    // Append the sret pointer to the `call` instruction's arguments.
    let ptr_type = Type::triple_pointer_type(isa.triple());
    let sret_arg = pos.ins().stack_addr(ptr_type, stack_slot, 0);
    pos.func.dfg.append_inst_arg(call, sret_arg);

    // The sret pointer might be returned by the signature as well. If so, we
    // need to add it to the `call` instruction's results list.
    //
    // Additionally, when the sret is explicitly returned in this calling
    // convention, then use it when loading the sret returns back into ssa
    // values to avoid keeping the original `sret_arg` live and potentially
    // having to do spills and fills.
    let sret =
        if pos.func.dfg.signatures[sig_ref].uses_special_return(ArgumentPurpose::StructReturn) {
            pos.func.dfg.append_result(call, ptr_type)
        } else {
            sret_arg
        };

    // Finally, load each of the call's return values out of the sret stack
    // slot.
    pos.goto_after_inst(call);
    let mut offset = 0;
    for i in 0..old_ret_list.len(&pos.func.dfg.value_lists) {
        if old_sig.returns[i].purpose != ArgumentPurpose::Normal {
            continue;
        }

        let old_v = old_ret_list.get(i, &pos.func.dfg.value_lists).unwrap();
        let ty = pos.func.dfg.value_type(old_v);
        let mut legalized_ty = legalized_type_for_sret(ty);

        offset = round_up_to_multiple_of_type_align(offset, legalized_ty);

        let new_legalized_v =
            pos.ins()
                .load(legalized_ty, MemFlags::trusted(), sret, offset as i32);

        // "Illegalize" the loaded value from the legalized type back to its
        // original `ty`. This is basically the opposite of
        // `legalize_type_for_sret_store`.
        let mut new_v = new_legalized_v;
        if ty.is_bool() {
            legalized_ty = legalized_ty.as_bool_pedantic();
            new_v = pos.ins().raw_bitcast(legalized_ty, new_v);

            if ty.bits() < legalized_ty.bits() {
                legalized_ty = ty;
                new_v = pos.ins().breduce(legalized_ty, new_v);
            }
        }

        pos.func.dfg.change_to_alias(old_v, new_v);

        offset += legalized_ty.bytes();
    }

    pos.func.dfg.old_signatures[sig_ref] = Some(old_sig);
}

/// Compute original value of type `ty` from the legalized ABI arguments.
///
/// The conversion is recursive, controlled by the `get_arg` closure which is called to retrieve an
/// ABI argument. It returns:
///
/// - `Ok(arg)` if the requested type matches the next ABI argument.
/// - `Err(arg_type)` if further conversions are needed from the ABI argument `arg_type`.
///
/// If the `into_result` value is provided, the converted result will be written into that value.
fn convert_from_abi<GetArg>(
    pos: &mut FuncCursor,
    ty: Type,
    into_result: Option<Value>,
    get_arg: &mut GetArg,
) -> Value
where
    GetArg: FnMut(&mut Function, Type) -> Result<Value, AbiParam>,
{
    // Terminate the recursion when we get the desired type.
    let arg_type = match get_arg(pos.func, ty) {
        Ok(v) => {
            debug_assert_eq!(pos.func.dfg.value_type(v), ty);
            debug_assert_eq!(into_result, None);
            return v;
        }
        Err(t) => t,
    };

    // Reconstruct how `ty` was legalized into the `arg_type` argument.
    let conversion = legalize_abi_value(ty, &arg_type);

    debug!("convert_from_abi({}): {:?}", ty, conversion);

    // The conversion describes value to ABI argument. We implement the reverse conversion here.
    match conversion {
        // Construct a `ty` by concatenating two ABI integers.
        ValueConversion::IntSplit => {
            let abi_ty = ty.half_width().expect("Invalid type for conversion");
            let lo = convert_from_abi(pos, abi_ty, None, get_arg);
            let hi = convert_from_abi(pos, abi_ty, None, get_arg);
            debug!(
                "intsplit {}: {}, {}: {}",
                lo,
                pos.func.dfg.value_type(lo),
                hi,
                pos.func.dfg.value_type(hi)
            );
            pos.ins().with_results([into_result]).iconcat(lo, hi)
        }
        // Construct a `ty` by concatenating two halves of a vector.
        ValueConversion::VectorSplit => {
            let abi_ty = ty.half_vector().expect("Invalid type for conversion");
            let lo = convert_from_abi(pos, abi_ty, None, get_arg);
            let hi = convert_from_abi(pos, abi_ty, None, get_arg);
            pos.ins().with_results([into_result]).vconcat(lo, hi)
        }
        // Construct a `ty` by bit-casting from an integer type.
        ValueConversion::IntBits => {
            debug_assert!(!ty.is_int());
            let abi_ty = Type::int(ty.bits()).expect("Invalid type for conversion");
            let arg = convert_from_abi(pos, abi_ty, None, get_arg);
            pos.ins().with_results([into_result]).bitcast(ty, arg)
        }
        // ABI argument is a sign-extended version of the value we want.
        ValueConversion::Sext(abi_ty) => {
            let arg = convert_from_abi(pos, abi_ty, None, get_arg);
            // TODO: Currently, we don't take advantage of the ABI argument being sign-extended.
            // We could insert an `assert_sreduce` which would fold with a following `sextend` of
            // this value.
            pos.ins().with_results([into_result]).ireduce(ty, arg)
        }
        ValueConversion::Uext(abi_ty) => {
            let arg = convert_from_abi(pos, abi_ty, None, get_arg);
            // TODO: Currently, we don't take advantage of the ABI argument being sign-extended.
            // We could insert an `assert_ureduce` which would fold with a following `uextend` of
            // this value.
            pos.ins().with_results([into_result]).ireduce(ty, arg)
        }
    }
}

/// Convert `value` to match an ABI signature by inserting instructions at `pos`.
///
/// This may require expanding the value to multiple ABI arguments. The conversion process is
/// recursive and controlled by the `put_arg` closure. When a candidate argument value is presented
/// to the closure, it will perform one of two actions:
///
/// 1. If the suggested argument has an acceptable value type, consume it by adding it to the list
///    of arguments and return `Ok(())`.
/// 2. If the suggested argument doesn't have the right value type, don't change anything, but
///    return the `Err(AbiParam)` that is needed.
///
fn convert_to_abi<PutArg>(
    pos: &mut FuncCursor,
    cfg: &ControlFlowGraph,
    value: Value,
    put_arg: &mut PutArg,
) where
    PutArg: FnMut(&mut Function, Value) -> Result<(), AbiParam>,
{
    // Start by invoking the closure to either terminate the recursion or get the argument type
    // we're trying to match.
    let arg_type = match put_arg(pos.func, value) {
        Ok(_) => return,
        Err(t) => t,
    };

    let ty = pos.func.dfg.value_type(value);
    match legalize_abi_value(ty, &arg_type) {
        ValueConversion::IntSplit => {
            let curpos = pos.position();
            let srcloc = pos.srcloc();
            let (lo, hi) = isplit(&mut pos.func, cfg, curpos, srcloc, value);
            convert_to_abi(pos, cfg, lo, put_arg);
            convert_to_abi(pos, cfg, hi, put_arg);
        }
        ValueConversion::VectorSplit => {
            let curpos = pos.position();
            let srcloc = pos.srcloc();
            let (lo, hi) = vsplit(&mut pos.func, cfg, curpos, srcloc, value);
            convert_to_abi(pos, cfg, lo, put_arg);
            convert_to_abi(pos, cfg, hi, put_arg);
        }
        ValueConversion::IntBits => {
            debug_assert!(!ty.is_int());
            let abi_ty = Type::int(ty.bits()).expect("Invalid type for conversion");
            let arg = pos.ins().bitcast(abi_ty, value);
            convert_to_abi(pos, cfg, arg, put_arg);
        }
        ValueConversion::Sext(abi_ty) => {
            let arg = pos.ins().sextend(abi_ty, value);
            convert_to_abi(pos, cfg, arg, put_arg);
        }
        ValueConversion::Uext(abi_ty) => {
            let arg = pos.ins().uextend(abi_ty, value);
            convert_to_abi(pos, cfg, arg, put_arg);
        }
    }
}

/// Check if a sequence of arguments match a desired sequence of argument types.
fn check_arg_types(dfg: &DataFlowGraph, args: &[Value], types: &[AbiParam]) -> bool {
    let arg_types = args.iter().map(|&v| dfg.value_type(v));
    let sig_types = types.iter().map(|&at| at.value_type);
    arg_types.eq(sig_types)
}

/// Check if the arguments of the call `inst` match the signature.
///
/// Returns `Ok(())` if the signature matches and no changes are needed, or `Err(sig_ref)` if the
/// signature doesn't match.
fn check_call_signature(dfg: &DataFlowGraph, inst: Inst) -> Result<(), SigRef> {
    // Extract the signature and argument values.
    let (sig_ref, args) = match dfg[inst].analyze_call(&dfg.value_lists) {
        CallInfo::Direct(func, args) => (dfg.ext_funcs[func].signature, args),
        CallInfo::Indirect(sig_ref, args) => (sig_ref, args),
        CallInfo::NotACall => panic!("Expected call, got {:?}", dfg[inst]),
    };
    let sig = &dfg.signatures[sig_ref];

    if check_arg_types(dfg, args, &sig.params[..])
        && check_arg_types(dfg, dfg.inst_results(inst), &sig.returns[..])
    {
        // All types check out.
        Ok(())
    } else {
        // Call types need fixing.
        Err(sig_ref)
    }
}

/// Check if the arguments of the return `inst` match the signature.
fn check_return_signature(dfg: &DataFlowGraph, inst: Inst, sig: &Signature) -> bool {
    check_arg_types(dfg, dfg.inst_variable_args(inst), &sig.returns)
}

/// Insert ABI conversion code for the arguments to the call or return instruction at `pos`.
///
/// - `abi_args` is the number of arguments that the ABI signature requires.
/// - `get_abi_type` is a closure that can provide the desired `AbiParam` for a given ABI
///   argument number in `0..abi_args`.
///
fn legalize_inst_arguments<ArgType>(
    pos: &mut FuncCursor,
    cfg: &ControlFlowGraph,
    abi_args: usize,
    mut get_abi_type: ArgType,
) where
    ArgType: FnMut(&Function, usize) -> AbiParam,
{
    let inst = pos
        .current_inst()
        .expect("Cursor must point to a call instruction");

    // Lift the value list out of the call instruction so we modify it.
    let mut vlist = pos.func.dfg[inst]
        .take_value_list()
        .expect("Call must have a value list");

    // The value list contains all arguments to the instruction, including the callee on an
    // indirect call which isn't part of the call arguments that must match the ABI signature.
    // Figure out how many fixed values are at the front of the list. We won't touch those.
    let num_fixed_values = pos.func.dfg[inst]
        .opcode()
        .constraints()
        .num_fixed_value_arguments();
    let have_args = vlist.len(&pos.func.dfg.value_lists) - num_fixed_values;
    if abi_args < have_args {
        // This happens with multiple return values after we've legalized the
        // signature but haven't legalized the return instruction yet. This
        // legalization is handled in `handle_return_abi`.
        pos.func.dfg[inst].put_value_list(vlist);
        return;
    }

    // Grow the value list to the right size and shift all the existing arguments to the right.
    // This lets us write the new argument values into the list without overwriting the old
    // arguments.
    //
    // Before:
    //
    //    <-->              fixed_values
    //        <-----------> have_args
    //   [FFFFOOOOOOOOOOOOO]
    //
    // After grow_at():
    //
    //    <-->                     fixed_values
    //               <-----------> have_args
    //        <------------------> abi_args
    //   [FFFF-------OOOOOOOOOOOOO]
    //               ^
    //               old_arg_offset
    //
    // After writing the new arguments:
    //
    //    <-->                     fixed_values
    //        <------------------> abi_args
    //   [FFFFNNNNNNNNNNNNNNNNNNNN]
    //
    vlist.grow_at(
        num_fixed_values,
        abi_args - have_args,
        &mut pos.func.dfg.value_lists,
    );
    let old_arg_offset = num_fixed_values + abi_args - have_args;

    let mut abi_arg = 0;
    for old_arg in 0..have_args {
        let old_value = vlist
            .get(old_arg_offset + old_arg, &pos.func.dfg.value_lists)
            .unwrap();
        let mut put_arg = |func: &mut Function, arg| {
            let abi_type = get_abi_type(func, abi_arg);
            if func.dfg.value_type(arg) == abi_type.value_type {
                // This is the argument type we need.
                vlist.as_mut_slice(&mut func.dfg.value_lists)[num_fixed_values + abi_arg] = arg;
                abi_arg += 1;
                Ok(())
            } else {
                Err(abi_type)
            }
        };
        convert_to_abi(pos, cfg, old_value, &mut put_arg);
    }

    // Put the modified value list back.
    pos.func.dfg[inst].put_value_list(vlist);
}

/// Ensure that the `ty` being returned is a type that can be loaded and stored
/// (potentially after another narrowing legalization) from memory, since it
/// will go into the `sret` space.
fn legalized_type_for_sret(ty: Type) -> Type {
    if ty.is_bool() {
        let bits = std::cmp::max(8, ty.bits());
        Type::int(bits).unwrap()
    } else {
        ty
    }
}

/// Insert any legalization code required to ensure that `val` can be stored
/// into the `sret` memory. Returns the (potentially new, potentially
/// unmodified) legalized value and its type.
fn legalize_type_for_sret_store(pos: &mut FuncCursor, val: Value, ty: Type) -> (Value, Type) {
    if ty.is_bool() {
        let bits = std::cmp::max(8, ty.bits());
        let ty = Type::int(bits).unwrap();
        let val = pos.ins().bint(ty, val);
        (val, ty)
    } else {
        (val, ty)
    }
}

/// Insert ABI conversion code before and after the call instruction at `pos`.
///
/// Instructions inserted before the call will compute the appropriate ABI values for the
/// callee's new ABI-legalized signature. The function call arguments are rewritten in place to
/// match the new signature.
///
/// Instructions will be inserted after the call to convert returned ABI values back to the
/// original return values. The call's result values will be adapted to match the new signature.
///
/// Returns `true` if any instructions were inserted.
pub fn handle_call_abi(
    isa: &dyn TargetIsa,
    mut inst: Inst,
    func: &mut Function,
    cfg: &ControlFlowGraph,
) -> bool {
    let pos = &mut FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    // Start by checking if the argument types already match the signature.
    let sig_ref = match check_call_signature(&pos.func.dfg, inst) {
        Ok(_) => return spill_call_arguments(pos),
        Err(s) => s,
    };

    let sig = &pos.func.dfg.signatures[sig_ref];
    let old_sig = &pos.func.dfg.old_signatures[sig_ref];

    if sig.uses_struct_return_param()
        && old_sig
            .as_ref()
            .map_or(false, |s| !s.uses_struct_return_param())
    {
        legalize_sret_call(isa, pos, sig_ref, inst);
    } else {
        if !pos.func.dfg.signatures[sig_ref].returns.is_empty() {
            inst = legalize_inst_results(pos, |func, abi_res| {
                func.dfg.signatures[sig_ref].returns[abi_res]
            });
        }
    }

    // Go back and fix the call arguments to match the ABI signature.
    pos.goto_inst(inst);
    let abi_args = pos.func.dfg.signatures[sig_ref].params.len();
    legalize_inst_arguments(pos, cfg, abi_args, |func, abi_arg| {
        func.dfg.signatures[sig_ref].params[abi_arg]
    });

    debug_assert!(
        check_call_signature(&pos.func.dfg, inst).is_ok(),
        "Signature still wrong: {}, {}{}",
        pos.func.dfg.display_inst(inst, None),
        sig_ref,
        pos.func.dfg.signatures[sig_ref]
    );

    // Go back and insert spills for any stack arguments.
    pos.goto_inst(inst);
    spill_call_arguments(pos);

    // Yes, we changed stuff.
    true
}

/// Insert ABI conversion code before and after the return instruction at `inst`.
///
/// Return `true` if any instructions were inserted.
pub fn handle_return_abi(inst: Inst, func: &mut Function, cfg: &ControlFlowGraph) -> bool {
    // Check if the returned types already match the signature.
    if check_return_signature(&func.dfg, inst, &func.signature) {
        return false;
    }

    // Count the special-purpose return values (`link`, `sret`, and `vmctx`) that were appended to
    // the legalized signature.
    let special_args = func
        .signature
        .returns
        .iter()
        .rev()
        .take_while(|&rt| {
            rt.purpose == ArgumentPurpose::Link
                || rt.purpose == ArgumentPurpose::StructReturn
                || rt.purpose == ArgumentPurpose::VMContext
        })
        .count();
    let abi_args = func.signature.returns.len() - special_args;

    let pos = &mut FuncCursor::new(func).at_inst(inst);
    pos.use_srcloc(inst);

    legalize_inst_arguments(pos, cfg, abi_args, |func, abi_arg| {
        func.signature.returns[abi_arg]
    });
    // Append special return arguments for any `sret`, `link`, and `vmctx` return values added to
    // the legalized signature. These values should simply be propagated from the entry block
    // arguments.
    if special_args > 0 {
        debug!(
            "Adding {} special-purpose arguments to {}",
            special_args,
            pos.func.dfg.display_inst(inst, None)
        );
        let mut vlist = pos.func.dfg[inst].take_value_list().unwrap();
        let mut sret = None;

        for arg in &pos.func.signature.returns[abi_args..] {
            match arg.purpose {
                ArgumentPurpose::Link
                | ArgumentPurpose::StructReturn
                | ArgumentPurpose::VMContext => {}
                ArgumentPurpose::Normal => panic!("unexpected return value {}", arg),
                _ => panic!("Unsupported special purpose return value {}", arg),
            }
            // A `link`/`sret`/`vmctx` return value can only appear in a signature that has a
            // unique matching argument. They are appended at the end, so search the signature from
            // the end.
            let idx = pos
                .func
                .signature
                .params
                .iter()
                .rposition(|t| t.purpose == arg.purpose)
                .expect("No matching special purpose argument.");
            // Get the corresponding entry block value and add it to the return instruction's
            // arguments.
            let val = pos
                .func
                .dfg
                .block_params(pos.func.layout.entry_block().unwrap())[idx];
            debug_assert_eq!(pos.func.dfg.value_type(val), arg.value_type);
            vlist.push(val, &mut pos.func.dfg.value_lists);

            if let ArgumentPurpose::StructReturn = arg.purpose {
                sret = Some(val);
            }
        }

        // Store all the regular returns into the retptr space and remove them
        // from the `return` instruction's value list.
        if let Some(sret) = sret {
            let mut offset = 0;
            let num_regular_rets = vlist.len(&pos.func.dfg.value_lists) - special_args;
            for i in 0..num_regular_rets {
                debug_assert_eq!(
                    pos.func.old_signature.as_ref().unwrap().returns[i].purpose,
                    ArgumentPurpose::Normal,
                );

                // The next return value to process is always at `0`, since the
                // list is emptied as we iterate.
                let v = vlist.get(0, &pos.func.dfg.value_lists).unwrap();
                let ty = pos.func.dfg.value_type(v);
                let (v, ty) = legalize_type_for_sret_store(pos, v, ty);

                let size = ty.bytes();
                offset = round_up_to_multiple_of_type_align(offset, ty);

                pos.ins().store(MemFlags::trusted(), v, sret, offset as i32);
                vlist.remove(0, &mut pos.func.dfg.value_lists);

                offset += size;
            }
        }
        pos.func.dfg[inst].put_value_list(vlist);
    }

    debug_assert_eq!(
        pos.func.dfg.inst_variable_args(inst).len(),
        abi_args + special_args
    );
    debug_assert!(
        check_return_signature(&pos.func.dfg, inst, &pos.func.signature),
        "Signature still wrong: {} / signature {}",
        pos.func.dfg.display_inst(inst, None),
        pos.func.signature
    );

    // Yes, we changed stuff.
    true
}

fn round_up_to_multiple_of_type_align(bytes: u32, ty: Type) -> u32 {
    // We don't have a dedicated alignment for types, so assume they are
    // size-aligned.
    let align = ty.bytes();
    round_up_to_multiple_of_pow2(bytes, align)
}

/// Round `n` up to the next multiple of `to` that is greater than or equal to
/// `n`.
///
/// `to` must be a power of two and greater than zero.
///
/// This is useful for rounding an offset or pointer up to some type's required
/// alignment.
fn round_up_to_multiple_of_pow2(n: u32, to: u32) -> u32 {
    debug_assert!(to > 0);
    debug_assert!(to.is_power_of_two());

    // The simple version of this function is
    //
    //     (n + to - 1) / to * to
    //
    // Consider the numerator: `n + to - 1`. This is ensuring that if there is
    // any remainder for `n / to`, then the result of the division is one
    // greater than `n / to`, and that otherwise we get exactly the same result
    // as `n / to` due to integer division rounding off the remainder. In other
    // words, we only round up if `n` is not aligned to `to`.
    //
    // However, we know `to` is a power of two, and therefore `anything / to` is
    // equivalent to `anything >> log2(to)` and `anything * to` is equivalent to
    // `anything << log2(to)`. We can therefore rewrite our simplified function
    // into the following:
    //
    //     (n + to - 1) >> log2(to) << log2(to)
    //
    // But shifting a value right by some number of bits `b` and then shifting
    // it left by that same number of bits `b` is equivalent to clearing the
    // bottom `b` bits of the number. We can clear the bottom `b` bits of a
    // number by bit-wise and'ing the number with the bit-wise not of `2^b - 1`.
    // Plugging this into our function and simplifying, we get:
    //
    //       (n + to - 1) >> log2(to) << log2(to)
    //     = (n + to - 1) & !(2^log2(to) - 1)
    //     = (n + to - 1) & !(to - 1)
    //
    // And now we have the final version of this function!

    (n + to - 1) & !(to - 1)
}

/// Assign stack slots to incoming function parameters on the stack.
///
/// Values that are passed into the function on the stack must be assigned to an `IncomingArg`
/// stack slot already during legalization.
fn spill_entry_params(func: &mut Function, entry: Block) {
    for (abi, &arg) in func
        .signature
        .params
        .iter()
        .zip(func.dfg.block_params(entry))
    {
        if let ArgumentLoc::Stack(offset) = abi.location {
            let ss = func.stack_slots.make_incoming_arg(abi.value_type, offset);
            func.locations[arg] = ValueLoc::Stack(ss);
        }
    }
}

/// Assign stack slots to outgoing function arguments on the stack.
///
/// Values that are passed to a called function on the stack must be assigned to a matching
/// `OutgoingArg` stack slot. The assignment must happen immediately before the call.
///
/// TODO: The outgoing stack slots can be written a bit earlier, as long as there are no branches
/// or calls between writing the stack slots and the call instruction. Writing the slots earlier
/// could help reduce register pressure before the call.
fn spill_call_arguments(pos: &mut FuncCursor) -> bool {
    let inst = pos
        .current_inst()
        .expect("Cursor must point to a call instruction");
    let sig_ref = pos
        .func
        .dfg
        .call_signature(inst)
        .expect("Call instruction expected.");

    // Start by building a list of stack slots and arguments to be replaced.
    // This requires borrowing `pos.func.dfg`, so we can't change anything.
    let arglist = {
        let locations = &pos.func.locations;
        let stack_slots = &mut pos.func.stack_slots;
        pos.func
            .dfg
            .inst_variable_args(inst)
            .iter()
            .zip(&pos.func.dfg.signatures[sig_ref].params)
            .enumerate()
            .filter_map(|(idx, (&arg, abi))| {
                match abi.location {
                    ArgumentLoc::Stack(offset) => {
                        // Assign `arg` to a new stack slot, unless it's already in the correct
                        // slot. The legalization needs to be idempotent, so we should see a
                        // correct outgoing slot on the second pass.
                        let ss = stack_slots.get_outgoing_arg(abi.value_type, offset);
                        if locations[arg] != ValueLoc::Stack(ss) {
                            Some((idx, arg, ss))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
            .collect::<Vec<_>>()
    };

    if arglist.is_empty() {
        return false;
    }

    // Insert the spill instructions and rewrite call arguments.
    for (idx, arg, ss) in arglist {
        let stack_val = pos.ins().spill(arg);
        pos.func.locations[stack_val] = ValueLoc::Stack(ss);
        pos.func.dfg.inst_variable_args_mut(inst)[idx] = stack_val;
    }

    // We changed stuff.
    true
}

#[cfg(test)]
mod tests {
    use super::round_up_to_multiple_of_pow2;

    #[test]
    fn round_up_to_multiple_of_pow2_works() {
        for (n, to, expected) in vec![
            (0, 1, 0),
            (1, 1, 1),
            (2, 1, 2),
            (0, 2, 0),
            (1, 2, 2),
            (2, 2, 2),
            (3, 2, 4),
            (0, 4, 0),
            (1, 4, 4),
            (2, 4, 4),
            (3, 4, 4),
            (4, 4, 4),
            (5, 4, 8),
        ] {
            let actual = round_up_to_multiple_of_pow2(n, to);
            assert_eq!(
                actual, expected,
                "round_up_to_multiple_of_pow2(n = {}, to = {}) = {} (expected {})",
                n, to, actual, expected
            );
        }
    }
}
