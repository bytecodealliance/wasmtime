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

use abi::{legalize_abi_value, ValueConversion};
use ir::{Function, Cursor, DataFlowGraph, Inst, InstBuilder, Ebb, Type, Value, Signature, SigRef,
         ArgumentType};
use ir::instructions::CallInfo;
use isa::TargetIsa;

/// Legalize all the function signatures in `func`.
///
/// This changes all signatures to be ABI-compliant with full `ArgumentLoc` annotations. It doesn't
/// change the entry block arguments, calls, or return instructions, so this can leave the function
/// in a state with type discrepancies.
pub fn legalize_signatures(func: &mut Function, isa: &TargetIsa) {
    isa.legalize_signature(&mut func.signature);
    for sig in func.dfg.signatures.keys() {
        isa.legalize_signature(&mut func.dfg.signatures[sig]);
    }

    if let Some(entry) = func.layout.entry_block() {
        legalize_entry_arguments(func, entry);
    }
}

/// Legalize the entry block arguments after `func`'s signature has been legalized.
///
/// The legalized signature may contain more arguments than the original signature, and the
/// argument types have been changed. This function goes through the arguments to the entry EBB and
/// replaces them with arguments of the right type for the ABI.
///
/// The original entry EBB arguments are computed from the new ABI arguments by code inserted at
/// the top of the entry block.
fn legalize_entry_arguments(func: &mut Function, entry: Ebb) {
    // Insert position for argument conversion code.
    // We want to insert instructions before the first instruction in the entry block.
    // If the entry block is empty, append instructions to it instead.
    let mut pos = Cursor::new(&mut func.layout);
    pos.goto_top(entry);
    pos.next_inst();

    // Keep track of the argument types in the ABI-legalized signature.
    let abi_types = &func.signature.argument_types;
    let mut abi_arg = 0;

    // Process the EBB arguments one at a time, possibly replacing one argument with multiple new
    // ones. We do this by detaching the entry EBB arguments first.
    let mut next_arg = func.dfg.detach_ebb_args(entry);
    while let Some(arg) = next_arg {
        // Get the next argument before we mutate `arg`.
        next_arg = func.dfg.next_ebb_arg(arg);

        let arg_type = func.dfg.value_type(arg);
        if arg_type == abi_types[abi_arg].value_type {
            // No value translation is necessary, this argument matches the ABI type.
            // Just use the original EBB argument value. This is the most common case.
            func.dfg.attach_ebb_arg(entry, arg);
            abi_arg += 1;
        } else {
            // Compute the value we want for `arg` from the legalized ABI arguments.
            let mut get_arg = |dfg: &mut DataFlowGraph, ty| {
                let abi_type = abi_types[abi_arg];
                if ty == abi_type.value_type {
                    abi_arg += 1;
                    Ok(dfg.append_ebb_arg(entry, ty))
                } else {
                    Err(abi_type)
                }
            };
            let converted = convert_from_abi(&mut func.dfg, &mut pos, arg_type, &mut get_arg);
            // The old `arg` is no longer an attached EBB argument, but there are probably still
            // uses of the value. Make it an alias to the converted value.
            func.dfg.change_to_alias(arg, converted);
        }
    }
}

/// Legalize the results returned from a call instruction to match the ABI signature.
///
/// The cursor `pos` points to a call instruction with at least one return value. The cursor will
/// be left pointing after the instructions inserted to convert the return values.
///
/// This function is very similar to the `legalize_entry_arguments` function above.
///
/// Returns the possibly new instruction representing the call.
fn legalize_inst_results<ResType>(dfg: &mut DataFlowGraph,
                                  pos: &mut Cursor,
                                  mut get_abi_type: ResType)
                                  -> Inst
    where ResType: FnMut(&DataFlowGraph, usize) -> ArgumentType
{
    let mut call = pos.current_inst().expect("Cursor must point to a call instruction");

    // We theoretically allow for call instructions that return a number of fixed results before
    // the call return values. In practice, it doesn't happen.
    let fixed_results = dfg[call].opcode().constraints().fixed_results();
    assert_eq!(fixed_results, 0, "Fixed results  on calls not supported");

    let mut next_res = dfg.detach_secondary_results(call);
    // The currently last result on the call instruction.
    let mut last_res = dfg.first_result(call);
    let mut abi_res = 0;

    // The first result requires special handling.
    let first_ty = dfg.value_type(last_res);
    if first_ty != get_abi_type(dfg, abi_res).value_type {
        // Move the call out of the way, so we can redefine the first result.
        let copy = call;
        call = dfg.redefine_first_value(pos);
        last_res = dfg.first_result(call);
        // Set up a closure that can attach new results to `call`.
        let mut get_res = |dfg: &mut DataFlowGraph, ty| {
            let abi_type = get_abi_type(dfg, abi_res);
            if ty == abi_type.value_type {
                // Don't append the first result - it's not detachable.
                if fixed_results + abi_res == 0 {
                    *dfg[call].first_type_mut() = ty;
                    debug_assert_eq!(last_res, dfg.first_result(call));
                } else {
                    last_res = dfg.append_secondary_result(last_res, ty);
                }
                abi_res += 1;
                Ok(last_res)
            } else {
                Err(abi_type)
            }
        };

        let v = convert_from_abi(dfg, pos, first_ty, &mut get_res);
        dfg.replace(copy).copy(v);
    }

    // Point immediately after the call and any instructions dealing with the first result.
    pos.next_inst();

    // Now do the secondary results.
    while let Some(res) = next_res {
        next_res = dfg.next_secondary_result(res);

        let res_type = dfg.value_type(res);
        if res_type == get_abi_type(dfg, abi_res).value_type {
            // No value translation is necessary, this result matches the ABI type.
            dfg.attach_secondary_result(last_res, res);
            last_res = res;
            abi_res += 1;
        } else {
            let mut get_res = |dfg: &mut DataFlowGraph, ty| {
                let abi_type = get_abi_type(dfg, abi_res);
                if ty == abi_type.value_type {
                    last_res = dfg.append_secondary_result(last_res, ty);
                    abi_res += 1;
                    Ok(last_res)
                } else {
                    Err(abi_type)
                }
            };
            let v = convert_from_abi(dfg, pos, res_type, &mut get_res);
            // The old `res` is no longer an attached result.
            dfg.change_to_alias(res, v);
        }
    }

    call
}

/// Compute original value of type `ty` from the legalized ABI arguments.
///
/// The conversion is recursive, controlled by the `get_arg` closure which is called to retrieve an
/// ABI argument. It returns:
///
/// - `Ok(arg)` if the requested type matches the next ABI argument.
/// - `Err(arg_type)` if further conversions are needed from the ABI argument `arg_type`.
///
fn convert_from_abi<GetArg>(dfg: &mut DataFlowGraph,
                            pos: &mut Cursor,
                            ty: Type,
                            get_arg: &mut GetArg)
                            -> Value
    where GetArg: FnMut(&mut DataFlowGraph, Type) -> Result<Value, ArgumentType>
{
    // Terminate the recursion when we get the desired type.
    let arg_type = match get_arg(dfg, ty) {
        Ok(v) => return v,
        Err(t) => t,
    };

    // Reconstruct how `ty` was legalized into the `arg_type` argument.
    let conversion = legalize_abi_value(ty, &arg_type);

    // The conversion describes value to ABI argument. We implement the reverse conversion here.
    match conversion {
        // Construct a `ty` by concatenating two ABI integers.
        ValueConversion::IntSplit => {
            let abi_ty = ty.half_width().expect("Invalid type for conversion");
            let lo = convert_from_abi(dfg, pos, abi_ty, get_arg);
            let hi = convert_from_abi(dfg, pos, abi_ty, get_arg);
            dfg.ins(pos).iconcat_lohi(lo, hi)
        }
        // Construct a `ty` by concatenating two halves of a vector.
        ValueConversion::VectorSplit => {
            let abi_ty = ty.half_vector().expect("Invalid type for conversion");
            let lo = convert_from_abi(dfg, pos, abi_ty, get_arg);
            let hi = convert_from_abi(dfg, pos, abi_ty, get_arg);
            dfg.ins(pos).vconcat(lo, hi)
        }
        // Construct a `ty` by bit-casting from an integer type.
        ValueConversion::IntBits => {
            assert!(!ty.is_int());
            let abi_ty = Type::int(ty.bits()).expect("Invalid type for conversion");
            let arg = convert_from_abi(dfg, pos, abi_ty, get_arg);
            dfg.ins(pos).bitcast(ty, arg)
        }
        // ABI argument is a sign-extended version of the value we want.
        ValueConversion::Sext(abi_ty) => {
            let arg = convert_from_abi(dfg, pos, abi_ty, get_arg);
            // TODO: Currently, we don't take advantage of the ABI argument being sign-extended.
            // We could insert an `assert_sreduce` which would fold with a following `sextend` of
            // this value.
            dfg.ins(pos).ireduce(ty, arg)
        }
        ValueConversion::Uext(abi_ty) => {
            let arg = convert_from_abi(dfg, pos, abi_ty, get_arg);
            // TODO: Currently, we don't take advantage of the ABI argument being sign-extended.
            // We could insert an `assert_ureduce` which would fold with a following `uextend` of
            // this value.
            dfg.ins(pos).ireduce(ty, arg)
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
///    return the `Err(ArgumentType)` that is needed.
///
fn convert_to_abi<PutArg>(dfg: &mut DataFlowGraph,
                          pos: &mut Cursor,
                          value: Value,
                          put_arg: &mut PutArg)
    where PutArg: FnMut(&mut DataFlowGraph, Value) -> Result<(), ArgumentType>
{
    // Start by invoking the closure to either terminate the recursion or get the argument type
    // we're trying to match.
    let arg_type = match put_arg(dfg, value) {
        Ok(_) => return,
        Err(t) => t,
    };

    let ty = dfg.value_type(value);
    match legalize_abi_value(ty, &arg_type) {
        ValueConversion::IntSplit => {
            let (lo, hi) = dfg.ins(pos).isplit_lohi(value);
            convert_to_abi(dfg, pos, lo, put_arg);
            convert_to_abi(dfg, pos, hi, put_arg);
        }
        ValueConversion::VectorSplit => {
            let (lo, hi) = dfg.ins(pos).vsplit(value);
            convert_to_abi(dfg, pos, lo, put_arg);
            convert_to_abi(dfg, pos, hi, put_arg);
        }
        ValueConversion::IntBits => {
            assert!(!ty.is_int());
            let abi_ty = Type::int(ty.bits()).expect("Invalid type for conversion");
            let arg = dfg.ins(pos).bitcast(abi_ty, value);
            convert_to_abi(dfg, pos, arg, put_arg);
        }
        ValueConversion::Sext(abi_ty) => {
            let arg = dfg.ins(pos).sextend(abi_ty, value);
            convert_to_abi(dfg, pos, arg, put_arg);
        }
        ValueConversion::Uext(abi_ty) => {
            let arg = dfg.ins(pos).uextend(abi_ty, value);
            convert_to_abi(dfg, pos, arg, put_arg);
        }
    }
}

/// Check if a sequence of arguments match a desired sequence of argument types.
fn check_arg_types<Args>(dfg: &DataFlowGraph, args: Args, types: &[ArgumentType]) -> bool
    where Args: IntoIterator<Item = Value>
{
    let mut n = 0;
    for arg in args {
        match types.get(n) {
            Some(&ArgumentType { value_type, .. }) => {
                if dfg.value_type(arg) != value_type {
                    return false;
                }
            }
            None => return false,
        }
        n += 1
    }

    // Also verify that the number of arguments matches.
    n == types.len()
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

    if check_arg_types(dfg, args.iter().cloned(), &sig.argument_types[..]) &&
       check_arg_types(dfg, dfg.inst_results(inst), &sig.return_types[..]) {
        // All types check out.
        Ok(())
    } else {
        // Call types need fixing.
        Err(sig_ref)
    }
}

/// Check if the arguments of the return `inst` match the signature.
fn check_return_signature(dfg: &DataFlowGraph, inst: Inst, sig: &Signature) -> bool {
    let fixed_values = dfg[inst].opcode().constraints().fixed_value_arguments();
    check_arg_types(dfg,
                    dfg[inst]
                        .arguments(&dfg.value_lists)
                        .iter()
                        .skip(fixed_values)
                        .cloned(),
                    &sig.return_types)
}

/// Insert ABI conversion code for the arguments to the call or return instruction at `pos`.
///
/// - `abi_args` is the number of arguments that the ABI signature requires.
/// - `get_abi_type` is a closure that can provide the desired `ArgumentType` for a given ABI
///   argument number in `0..abi_args`.
///
fn legalize_inst_arguments<ArgType>(dfg: &mut DataFlowGraph,
                                    pos: &mut Cursor,
                                    abi_args: usize,
                                    mut get_abi_type: ArgType)
    where ArgType: FnMut(&DataFlowGraph, usize) -> ArgumentType
{
    let inst = pos.current_inst().expect("Cursor must point to a call instruction");

    // Lift the value list out of the call instruction so we modify it.
    let mut vlist = dfg[inst].take_value_list().expect("Call must have a value list");

    // The value list contains all arguments to the instruction, including the callee on an
    // indirect call which isn't part of the call arguments that must match the ABI signature.
    // Figure out how many fixed values are at the front of the list. We won't touch those.
    let fixed_values = dfg[inst].opcode().constraints().fixed_value_arguments();
    let have_args = vlist.len(&dfg.value_lists) - fixed_values;

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
    vlist.grow_at(fixed_values, abi_args - have_args, &mut dfg.value_lists);
    let old_arg_offset = fixed_values + abi_args - have_args;

    let mut abi_arg = 0;
    for old_arg in 0..have_args {
        let old_value = vlist.get(old_arg_offset + old_arg, &dfg.value_lists).unwrap();
        let mut put_arg = |dfg: &mut DataFlowGraph, arg| {
            let abi_type = get_abi_type(dfg, abi_arg);
            if dfg.value_type(arg) == abi_type.value_type {
                // This is the argument type we need.
                vlist.as_mut_slice(&mut dfg.value_lists)[fixed_values + abi_arg] = arg;
                abi_arg += 1;
                Ok(())
            } else {
                Err(abi_type)
            }
        };
        convert_to_abi(dfg, pos, old_value, &mut put_arg);
    }

    // Put the modified value list back.
    dfg[inst].put_value_list(vlist);
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
pub fn handle_call_abi(dfg: &mut DataFlowGraph, pos: &mut Cursor) -> bool {
    let mut inst = pos.current_inst().expect("Cursor must point to a call instruction");

    // Start by checking if the argument types already match the signature.
    let sig_ref = match check_call_signature(dfg, inst) {
        Ok(_) => return false,
        Err(s) => s,
    };

    // OK, we need to fix the call arguments to match the ABI signature.
    let abi_args = dfg.signatures[sig_ref].argument_types.len();
    legalize_inst_arguments(dfg,
                            pos,
                            abi_args,
                            |dfg, abi_arg| dfg.signatures[sig_ref].argument_types[abi_arg]);

    if !dfg.signatures[sig_ref].return_types.is_empty() {
        inst = legalize_inst_results(dfg,
                                     pos,
                                     |dfg, abi_res| dfg.signatures[sig_ref].return_types[abi_res]);
    }

    debug_assert!(check_call_signature(dfg, inst).is_ok(),
                  "Signature still wrong: {}, {}{}",
                  dfg.display_inst(inst),
                  sig_ref,
                  dfg.signatures[sig_ref]);

    // Yes, we changed stuff.
    true
}

/// Insert ABI conversion code before and after the call instruction at `pos`.
///
/// Return `true` if any instructions were inserted.
pub fn handle_return_abi(dfg: &mut DataFlowGraph, pos: &mut Cursor, sig: &Signature) -> bool {
    let inst = pos.current_inst().expect("Cursor must point to a return instruction");

    // Check if the returned types already match the signature.
    if check_return_signature(dfg, inst, sig) {
        return false;
    }

    let abi_args = sig.return_types.len();
    legalize_inst_arguments(dfg, pos, abi_args, |_, abi_arg| sig.return_types[abi_arg]);

    debug_assert!(check_return_signature(dfg, inst, sig),
                  "Signature still wrong: {}, sig{}",
                  dfg.display_inst(inst),
                  sig);

    // Yes, we changed stuff.
    true
}
