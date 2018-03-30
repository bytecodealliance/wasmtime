//! Expanding instructions as runtime library calls.

use ir;
use ir::InstBuilder;

/// Try to expand `inst` as a library call, returning true is successful.
pub fn expand_as_libcall(inst: ir::Inst, func: &mut ir::Function) -> bool {
    // Does the opcode/ctrl_type combo even have a well-known runtime library name.
    let libcall =
        match ir::LibCall::for_inst(func.dfg[inst].opcode(), func.dfg.ctrl_typevar(inst)) {
            Some(lc) => lc,
            None => return false,
        };

    let funcref = find_funcref(libcall, func).unwrap_or_else(|| make_funcref(libcall, inst, func));

    // Now we convert `inst` to a call. First save the arguments.
    let mut args = Vec::new();
    args.extend_from_slice(func.dfg.inst_args(inst));
    // The replace builder will preserve the instruction result values.
    func.dfg.replace(inst).call(funcref, &args);

    // TODO: ask the ISA to legalize the signature.

    true
}

/// Get the existing function reference for `libcall` in `func` if it exists.
fn find_funcref(libcall: ir::LibCall, func: &ir::Function) -> Option<ir::FuncRef> {
    // We're assuming that all libcall function decls are at the end.
    // If we get this wrong, worst case we'll have duplicate libcall decls which is harmless.
    for (fref, func_data) in func.dfg.ext_funcs.iter().rev() {
        match func_data.name {
            ir::ExternalName::LibCall(lc) => {
                if lc == libcall {
                    return Some(fref);
                }
            }
            _ => break,
        }
    }
    None
}

/// Create a funcref for `libcall` with a signature matching `inst`.
fn make_funcref(libcall: ir::LibCall, inst: ir::Inst, func: &mut ir::Function) -> ir::FuncRef {
    // Start with a system_v calling convention. We'll give the ISA a chance to change it.
    let mut sig = ir::Signature::new(ir::CallConv::SystemV);
    for &v in func.dfg.inst_args(inst) {
        sig.params.push(ir::AbiParam::new(func.dfg.value_type(v)));
    }
    for &v in func.dfg.inst_results(inst) {
        sig.returns.push(ir::AbiParam::new(func.dfg.value_type(v)));
    }
    let sigref = func.import_signature(sig);

    func.import_function(ir::ExtFuncData {
        name: ir::ExternalName::LibCall(libcall),
        signature: sigref,
    })
}
