//! Expanding instructions as runtime library calls.

use ir;
use ir::{get_libcall_funcref, InstBuilder};
use isa::TargetIsa;
use legalizer::boundary::legalize_libcall_signature;
use std::vec::Vec;

/// Try to expand `inst` as a library call, returning true is successful.
pub fn expand_as_libcall(inst: ir::Inst, func: &mut ir::Function, isa: &TargetIsa) -> bool {
    // Does the opcode/ctrl_type combo even have a well-known runtime library name.
    let libcall = match ir::LibCall::for_inst(func.dfg[inst].opcode(), func.dfg.ctrl_typevar(inst))
    {
        Some(lc) => lc,
        None => return false,
    };

    // Now we convert `inst` to a call. First save the arguments.
    let mut args = Vec::new();
    args.extend_from_slice(func.dfg.inst_args(inst));
    // The replace builder will preserve the instruction result values.
    let funcref = get_libcall_funcref(libcall, func, inst, isa);
    func.dfg.replace(inst).call(funcref, &args);

    // Ask the ISA to legalize the signature.
    let fn_data = &func.dfg.ext_funcs[funcref];
    let sig_data = &mut func.dfg.signatures[fn_data.signature];
    legalize_libcall_signature(sig_data, isa);

    true
}
