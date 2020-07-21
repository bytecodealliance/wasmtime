//! Expanding instructions as runtime library calls.

use crate::ir;
use crate::ir::{libcall::get_libcall_funcref, InstBuilder};
use crate::isa::{CallConv, TargetIsa};
use crate::legalizer::boundary::legalize_libcall_signature;
use alloc::vec::Vec;

/// Try to expand `inst` as a library call, returning true is successful.
pub fn expand_as_libcall(inst: ir::Inst, func: &mut ir::Function, isa: &dyn TargetIsa) -> bool {
    // Does the opcode/ctrl_type combo even have a well-known runtime library name.
    let libcall = match ir::LibCall::for_inst(func.dfg[inst].opcode(), func.dfg.ctrl_typevar(inst))
    {
        Some(lc) => lc,
        None => return false,
    };

    // Now we convert `inst` to a call. First save the arguments.
    let mut args = Vec::new();
    args.extend_from_slice(func.dfg.inst_args(inst));

    let call_conv = CallConv::for_libcall(isa.flags(), isa.default_call_conv());
    if call_conv.extends_baldrdash() {
        let vmctx = func
            .special_param(ir::ArgumentPurpose::VMContext)
            .expect("Missing vmctx parameter for baldrdash libcall");
        args.push(vmctx);
    }

    // The replace builder will preserve the instruction result values.
    let funcref = get_libcall_funcref(libcall, call_conv, func, inst, isa);
    func.dfg.replace(inst).call(funcref, &args);

    // Ask the ISA to legalize the signature.
    let fn_data = &func.dfg.ext_funcs[funcref];
    let sig_data = &mut func.dfg.signatures[fn_data.signature];
    legalize_libcall_signature(sig_data, isa);

    true
}
